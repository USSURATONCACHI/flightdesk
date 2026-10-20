//! Async client for the Camunda 7 REST API (`engine-rest`). No UI types here.

pub mod query;
pub mod types;

pub use query::*;
pub use types::*;

use reqwest::{Method, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Auth {
    None,
    Basic { user: String, password: String },
    Bearer(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Base URL of the REST API, e.g. `http://localhost:8080/engine-rest`.
    pub base_url: String,
    pub auth: Auth,
    pub timeout: Duration,
    pub accept_invalid_certs: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { base_url: "http://localhost:8080/engine-rest".into(), auth: Auth::None, timeout: Duration::from_secs(30), accept_invalid_certs: false }
    }
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum ApiError {
    /// The engine answered with an error status; `kind`/`message` come from its JSON body when present.
    #[error("{status} {kind}: {message}")]
    Status { status: u16, kind: String, message: String },
    #[error("network: {0}")]
    Network(String),
    #[error("timeout")]
    Timeout,
    #[error("decode: {0}")]
    Decode(String),
}

impl ApiError {
    pub fn status(&self) -> Option<u16> {
        match self {
            ApiError::Status { status, .. } => Some(*status),
            _ => None,
        }
    }
    pub fn is_unauthorized(&self) -> bool {
        matches!(self.status(), Some(401) | Some(403))
    }
    pub fn is_not_found(&self) -> bool {
        self.status() == Some(404)
    }
    pub fn short(&self) -> String {
        match self {
            ApiError::Status { status, message, .. } => format!("HTTP {status}: {}", first_line(message)),
            ApiError::Network(m) => format!("Network error: {}", first_line(m)),
            ApiError::Timeout => "Request timed out".into(),
            ApiError::Decode(m) => format!("Bad response: {}", first_line(m)),
        }
    }
}

fn first_line(s: &str) -> String {
    let l = s.lines().next().unwrap_or("");
    if l.len() > 200 {
        format!("{}…", &l[..200])
    } else {
        l.to_string()
    }
}

pub type Result<T> = std::result::Result<T, ApiError>;

#[derive(Clone, Debug)]
pub struct Client {
    http: reqwest::Client,
    cfg: Config,
}

/// Normalise what people paste: strips trailing slashes and accepts a Cockpit URL or bare host.
pub fn normalize_base_url(input: &str) -> String {
    let mut s = input.trim().trim_end_matches('/').to_string();
    if !s.starts_with("http://") && !s.starts_with("https://") {
        s = format!("http://{s}");
    }
    // Cockpit URLs → engine-rest
    for marker in ["/camunda/app", "/camunda/api", "/app/cockpit"] {
        if let Some(i) = s.find(marker) {
            s.truncate(i);
            s.push_str("/engine-rest");
            return s;
        }
    }
    if s.ends_with("/engine-rest") || s.contains("/engine-rest/") {
        return s;
    }
    // bare host or app root
    format!("{s}/engine-rest")
}

impl Client {
    pub fn new(cfg: Config) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(cfg.timeout)
            .danger_accept_invalid_certs(cfg.accept_invalid_certs)
            .gzip(true)
            .build()
            .map_err(|e| ApiError::Network(e.to_string()))?;
        Ok(Self { http, cfg })
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    fn req(&self, method: Method, path: &str) -> RequestBuilder {
        let url = format!("{}/{}", self.cfg.base_url.trim_end_matches('/'), path.trim_start_matches('/'));
        let r = self.http.request(method, url).header("Accept", "application/json");
        match &self.cfg.auth {
            Auth::None => r,
            Auth::Basic { user, password } => r.basic_auth(user, Some(password)),
            Auth::Bearer(t) => r.bearer_auth(t),
        }
    }

    async fn send<T: DeserializeOwned>(&self, rb: RequestBuilder) -> Result<T> {
        let resp = rb.send().await.map_err(map_reqwest)?;
        let status = resp.status();
        let bytes = resp.bytes().await.map_err(map_reqwest)?;
        if !status.is_success() {
            return Err(status_error(status, &bytes));
        }
        if bytes.is_empty() || status == StatusCode::NO_CONTENT {
            // Allow `()` and Value::Null targets.
            return serde_json::from_slice(b"null").map_err(|e| ApiError::Decode(e.to_string()));
        }
        serde_json::from_slice(&bytes).map_err(|e| ApiError::Decode(format!("{e}; body: {}", first_line(&String::from_utf8_lossy(&bytes[..bytes.len().min(300)])))))
    }

    async fn send_text(&self, rb: RequestBuilder) -> Result<String> {
        let resp = rb.send().await.map_err(map_reqwest)?;
        let status = resp.status();
        let bytes = resp.bytes().await.map_err(map_reqwest)?;
        if !status.is_success() {
            return Err(status_error(status, &bytes));
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, String)]) -> Result<T> {
        self.send(self.req(Method::GET, path).query(query)).await
    }
    async fn post<T: DeserializeOwned, B: Serialize + ?Sized>(&self, path: &str, body: &B) -> Result<T> {
        self.send(self.req(Method::POST, path).json(body)).await
    }
    async fn post_q<T: DeserializeOwned, B: Serialize + ?Sized>(&self, path: &str, query: &[(&str, String)], body: &B) -> Result<T> {
        self.send(self.req(Method::POST, path).query(query).json(body)).await
    }
    async fn put<B: Serialize + ?Sized>(&self, path: &str, body: &B) -> Result<()> {
        let _: Value = self.send(self.req(Method::PUT, path).json(body)).await?;
        Ok(())
    }
    async fn delete(&self, path: &str, query: &[(&str, String)]) -> Result<()> {
        let _: Value = self.send(self.req(Method::DELETE, path).query(query)).await?;
        Ok(())
    }

    // ----- engine -----
    pub async fn version(&self) -> Result<Version> {
        self.get("version", &[]).await
    }
    pub async fn user_profile(&self, id: &str) -> Result<UserProfile> {
        self.get(&format!("user/{id}/profile"), &[]).await
    }

    // ----- definitions -----
    pub async fn process_definitions(&self, latest_only: bool) -> Result<Vec<ProcessDefinition>> {
        let mut q = vec![("sortBy", "key".to_string()), ("sortOrder", "asc".to_string())];
        if latest_only {
            q.push(("latestVersion", "true".into()));
        }
        self.get("process-definition", &q).await
    }
    pub async fn process_definition_versions(&self, key: &str) -> Result<Vec<ProcessDefinition>> {
        self.get("process-definition", &[("key", key.to_string()), ("sortBy", "version".into()), ("sortOrder", "desc".into())]).await
    }
    pub async fn process_definition(&self, id: &str) -> Result<ProcessDefinition> {
        self.get(&format!("process-definition/{id}"), &[]).await
    }
    pub async fn process_definition_by_key(&self, key: &str) -> Result<ProcessDefinition> {
        self.get(&format!("process-definition/key/{key}"), &[]).await
    }
    pub async fn process_definition_xml(&self, id: &str) -> Result<String> {
        let d: ProcessDefinitionDiagram = self.get(&format!("process-definition/{id}/xml"), &[]).await?;
        Ok(d.bpmn20_xml)
    }
    /// Per-definition instance / incident counts for every definition (all versions).
    pub async fn process_definition_statistics(&self) -> Result<Vec<ProcessDefinitionStatistics>> {
        self.get("process-definition/statistics", &[("incidents", "true".into()), ("failedJobs", "true".into())]).await
    }
    /// Per-activity instance / incident counts for one definition version.
    pub async fn activity_statistics(&self, definition_id: &str) -> Result<Vec<ActivityStatistics>> {
        self.get(&format!("process-definition/{definition_id}/statistics"), &[("incidents", "true".into()), ("failedJobs", "true".into())]).await
    }
    pub async fn suspend_definition(&self, id: &str, suspended: bool, include_instances: bool) -> Result<()> {
        self.put(&format!("process-definition/{id}/suspended"), &json!({"suspended": suspended, "includeProcessInstances": include_instances})).await
    }
    pub async fn start_instance(&self, definition_id: &str, body: &Value) -> Result<ProcessInstance> {
        self.post(&format!("process-definition/{definition_id}/start"), body).await
    }

    // ----- instances -----
    pub async fn process_instances(&self, q: &ProcessInstanceQuery, first: usize, max: usize) -> Result<Vec<ProcessInstance>> {
        self.post_q("process-instance", &[("firstResult", first.to_string()), ("maxResults", max.to_string())], q).await
    }
    pub async fn process_instances_count(&self, q: &ProcessInstanceQuery) -> Result<i64> {
        let c: Count = self.post("process-instance/count", q).await?;
        Ok(c.count)
    }
    pub async fn process_instance(&self, id: &str) -> Result<ProcessInstance> {
        self.get(&format!("process-instance/{id}"), &[]).await
    }
    pub async fn history_process_instances(&self, q: &HistoricProcessInstanceQuery, first: usize, max: usize) -> Result<Vec<HistoricProcessInstance>> {
        self.post_q("history/process-instance", &[("firstResult", first.to_string()), ("maxResults", max.to_string())], q).await
    }
    pub async fn history_process_instances_count(&self, q: &HistoricProcessInstanceQuery) -> Result<i64> {
        let c: Count = self.post("history/process-instance/count", q).await?;
        Ok(c.count)
    }
    pub async fn history_process_instance(&self, id: &str) -> Result<HistoricProcessInstance> {
        self.get(&format!("history/process-instance/{id}"), &[]).await
    }
    pub async fn activity_instances(&self, process_instance_id: &str) -> Result<ActivityInstance> {
        self.get(&format!("process-instance/{process_instance_id}/activity-instances"), &[]).await
    }
    pub async fn history_activity_instances(&self, process_instance_id: &str) -> Result<Vec<HistoricActivityInstance>> {
        self.get(
            "history/activity-instance",
            &[("processInstanceId", process_instance_id.to_string()), ("sortBy", "startTime".into()), ("sortOrder", "asc".into()), ("maxResults", "5000".into())],
        )
        .await
    }
    /// Number of historic activity instances per activity for a definition (heat map).
    pub async fn history_activity_count(&self, definition_id: &str, activity_id: &str) -> Result<i64> {
        let c: Count = self.get("history/activity-instance/count", &[("processDefinitionId", definition_id.to_string()), ("activityId", activity_id.to_string())]).await?;
        Ok(c.count)
    }
    pub async fn suspend_instance(&self, id: &str, suspended: bool) -> Result<()> {
        self.put(&format!("process-instance/{id}/suspended"), &json!({"suspended": suspended})).await
    }
    pub async fn delete_instance(&self, id: &str, reason: Option<&str>, skip_custom_listeners: bool, skip_io_mappings: bool, skip_subprocesses: bool) -> Result<()> {
        let mut q = vec![
            ("skipCustomListeners", skip_custom_listeners.to_string()),
            ("skipIoMappings", skip_io_mappings.to_string()),
            ("skipSubprocesses", skip_subprocesses.to_string()),
        ];
        if let Some(r) = reason {
            q.push(("deleteReason", r.to_string()));
        }
        self.delete(&format!("process-instance/{id}"), &q).await
    }
    pub async fn delete_history_instance(&self, id: &str) -> Result<()> {
        self.delete(&format!("history/process-instance/{id}"), &[]).await
    }
    pub async fn modify_instance(&self, id: &str, body: &ModificationRequest, async_: bool) -> Result<Option<Batch>> {
        if async_ {
            let b: Batch = self.post(&format!("process-instance/{id}/modification-async"), body).await?;
            Ok(Some(b))
        } else {
            let _: Value = self.post(&format!("process-instance/{id}/modification"), body).await?;
            Ok(None)
        }
    }
    pub async fn restart_instances(&self, definition_id: &str, body: &RestartRequest, async_: bool) -> Result<Option<Batch>> {
        if async_ {
            let b: Batch = self.post(&format!("process-definition/{definition_id}/restart-async"), body).await?;
            Ok(Some(b))
        } else {
            let _: Value = self.post(&format!("process-definition/{definition_id}/restart"), body).await?;
            Ok(None)
        }
    }
    pub async fn called_process_instances(&self, super_id: &str) -> Result<Vec<HistoricProcessInstance>> {
        self.history_process_instances(&HistoricProcessInstanceQuery { super_process_instance_id: Some(super_id.into()), ..Default::default() }, 0, 500).await
    }

    // ----- variables -----
    pub async fn variables(&self, process_instance_id: &str) -> Result<Vec<VariableInstance>> {
        self.post_q(
            "variable-instance",
            &[("deserializeValues", "false".into()), ("maxResults", "5000".into())],
            &VariableInstanceQuery { process_instance_id_in: Some(vec![process_instance_id.into()]), sorting: Some(vec![Sorting { sort_by: "variableName".into(), sort_order: "asc".into() }]), ..Default::default() },
        )
        .await
    }
    pub async fn history_variables(&self, process_instance_id: &str) -> Result<Vec<VariableInstance>> {
        self.get(
            "history/variable-instance",
            &[("processInstanceId", process_instance_id.to_string()), ("deserializeValues", "false".into()), ("sortBy", "variableName".into()), ("sortOrder", "asc".into()), ("maxResults", "5000".into())],
        )
        .await
    }
    /// Values of one variable for many instances (used for variable columns in search results).
    pub async fn history_variable_for_instances(&self, name: &str, instance_ids: &[String]) -> Result<Vec<VariableInstance>> {
        let mut q = vec![("deserializeValues", "false".to_string()), ("maxResults", "5000".into())];
        q.push(("variableName", name.to_string()));
        q.push(("processInstanceIdIn", instance_ids.join(",")));
        self.get("history/variable-instance", &q).await
    }
    pub async fn put_variable(&self, execution_id: &str, name: &str, value: &VariableValue) -> Result<()> {
        self.put(&format!("execution/{execution_id}/localVariables/{}", enc(name)), value).await
    }
    pub async fn put_process_variable(&self, process_instance_id: &str, name: &str, value: &VariableValue) -> Result<()> {
        self.put(&format!("process-instance/{process_instance_id}/variables/{}", enc(name)), value).await
    }
    pub async fn delete_variable(&self, execution_id: &str, name: &str) -> Result<()> {
        self.delete(&format!("execution/{execution_id}/localVariables/{}", enc(name)), &[]).await
    }
    pub async fn delete_process_variable(&self, process_instance_id: &str, name: &str) -> Result<()> {
        self.delete(&format!("process-instance/{process_instance_id}/variables/{}", enc(name)), &[]).await
    }

    // ----- incidents -----
    pub async fn incidents(&self, query: &[(&str, String)]) -> Result<Vec<Incident>> {
        let mut q: Vec<(&str, String)> = query.to_vec();
        q.push(("sortBy", "incidentTimestamp".into()));
        q.push(("sortOrder", "desc".into()));
        if !q.iter().any(|(k, _)| *k == "maxResults") {
            q.push(("maxResults", "500".into()));
        }
        self.get("incident", &q).await
    }
    pub async fn incidents_count(&self, query: &[(&str, String)]) -> Result<i64> {
        let c: Count = self.get("incident/count", query).await?;
        Ok(c.count)
    }
    pub async fn history_incidents(&self, process_instance_id: &str) -> Result<Vec<Incident>> {
        self.get("history/incident", &[("processInstanceId", process_instance_id.to_string()), ("sortBy", "createTime".into()), ("sortOrder", "desc".into()), ("maxResults", "500".into())]).await
    }
    pub async fn set_incident_annotation(&self, id: &str, annotation: &str) -> Result<()> {
        self.put(&format!("incident/{id}/annotation"), &json!({"annotation": annotation})).await
    }
    pub async fn resolve_incident(&self, id: &str) -> Result<()> {
        self.delete(&format!("incident/{id}"), &[]).await
    }

    // ----- jobs -----
    pub async fn jobs(&self, query: &[(&str, String)]) -> Result<Vec<Job>> {
        let mut q: Vec<(&str, String)> = query.to_vec();
        if !q.iter().any(|(k, _)| *k == "maxResults") {
            q.push(("maxResults", "500".into()));
        }
        self.get("job", &q).await
    }
    pub async fn job_stacktrace(&self, id: &str) -> Result<String> {
        self.send_text(self.req(Method::GET, &format!("job/{id}/stacktrace")).header("Accept", "text/plain")).await
    }
    pub async fn set_job_retries(&self, id: &str, retries: i64) -> Result<()> {
        self.put(&format!("job/{id}/retries"), &json!({"retries": retries})).await
    }
    pub async fn set_job_duedate(&self, id: &str, due: Option<&str>) -> Result<()> {
        self.put(&format!("job/{id}/duedate"), &json!({"duedate": due})).await
    }
    pub async fn execute_job(&self, id: &str) -> Result<()> {
        let _: Value = self.post(&format!("job/{id}/execute"), &json!({})).await?;
        Ok(())
    }
    pub async fn suspend_job(&self, id: &str, suspended: bool) -> Result<()> {
        self.put(&format!("job/{id}/suspended"), &json!({"suspended": suspended})).await
    }
    pub async fn set_job_priority(&self, id: &str, priority: i64) -> Result<()> {
        self.put(&format!("job/{id}/priority"), &json!({"priority": priority})).await
    }
    pub async fn job_definitions(&self, definition_id: &str) -> Result<Vec<JobDefinition>> {
        self.get("job-definition", &[("processDefinitionId", definition_id.to_string()), ("maxResults", "2000".into())]).await
    }

    // ----- external tasks -----
    pub async fn external_tasks(&self, query: &[(&str, String)]) -> Result<Vec<ExternalTask>> {
        let mut q: Vec<(&str, String)> = query.to_vec();
        if !q.iter().any(|(k, _)| *k == "maxResults") {
            q.push(("maxResults", "500".into()));
        }
        self.get("external-task", &q).await
    }
    pub async fn external_task_error_details(&self, id: &str) -> Result<String> {
        self.send_text(self.req(Method::GET, &format!("external-task/{id}/errorDetails")).header("Accept", "text/plain")).await
    }
    pub async fn set_external_task_retries(&self, id: &str, retries: i64) -> Result<()> {
        self.put(&format!("external-task/{id}/retries"), &json!({"retries": retries})).await
    }
    pub async fn unlock_external_task(&self, id: &str) -> Result<()> {
        let _: Value = self.post(&format!("external-task/{id}/unlock"), &json!({})).await?;
        Ok(())
    }
    pub async fn set_external_task_priority(&self, id: &str, priority: i64) -> Result<()> {
        self.put(&format!("external-task/{id}/priority"), &json!({"priority": priority})).await
    }

    // ----- user tasks -----
    pub async fn tasks(&self, query: &[(&str, String)]) -> Result<Vec<Task>> {
        let mut q: Vec<(&str, String)> = query.to_vec();
        if !q.iter().any(|(k, _)| *k == "maxResults") {
            q.push(("maxResults", "500".into()));
        }
        self.get("task", &q).await
    }
    pub async fn task_identity_links(&self, id: &str) -> Result<Vec<IdentityLink>> {
        self.get(&format!("task/{id}/identity-links"), &[]).await
    }
    pub async fn set_assignee(&self, task_id: &str, user: Option<&str>) -> Result<()> {
        let _: Value = self.post(&format!("task/{task_id}/assignee"), &json!({"userId": user})).await?;
        Ok(())
    }
    pub async fn claim_task(&self, task_id: &str, user: &str) -> Result<()> {
        let _: Value = self.post(&format!("task/{task_id}/claim"), &json!({"userId": user})).await?;
        Ok(())
    }
    pub async fn unclaim_task(&self, task_id: &str) -> Result<()> {
        let _: Value = self.post(&format!("task/{task_id}/unclaim"), &json!({})).await?;
        Ok(())
    }
    pub async fn update_task(&self, task: &Task) -> Result<()> {
        self.put(&format!("task/{}", task.id), task).await
    }
    pub async fn complete_task(&self, task_id: &str, variables: &Value) -> Result<()> {
        let _: Value = self.post(&format!("task/{task_id}/complete"), &json!({"variables": variables})).await?;
        Ok(())
    }

    // ----- messages -----
    pub async fn correlate_message(&self, body: &Value) -> Result<Vec<MessageCorrelationResult>> {
        self.post("message", body).await
    }

    // ----- batches -----
    pub async fn batches(&self) -> Result<Vec<Batch>> {
        self.get("batch/statistics", &[("maxResults", "200".into())]).await
    }
    pub async fn history_batches(&self) -> Result<Vec<Batch>> {
        self.get("history/batch", &[("maxResults", "200".into()), ("sortBy", "startTime".into()), ("sortOrder", "desc".into())]).await
    }
    pub async fn delete_batch(&self, id: &str, cascade: bool) -> Result<()> {
        self.delete(&format!("batch/{id}"), &[("cascade", cascade.to_string())]).await
    }
    pub async fn suspend_batch(&self, id: &str, suspended: bool) -> Result<()> {
        self.put(&format!("batch/{id}/suspended"), &json!({"suspended": suspended})).await
    }
    /// Batch operations over a runtime query or an explicit id list.
    pub async fn suspend_async(&self, ids: &[String], suspended: bool) -> Result<Batch> {
        self.post("process-instance/suspended-async", &json!({"processInstanceIds": ids, "suspended": suspended})).await
    }
    pub async fn delete_async(&self, ids: &[String], reason: &str, skip_custom_listeners: bool, skip_subprocesses: bool) -> Result<Batch> {
        self.post("process-instance/delete", &json!({"processInstanceIds": ids, "deleteReason": reason, "skipCustomListeners": skip_custom_listeners, "skipSubprocesses": skip_subprocesses})).await
    }
    pub async fn set_retries_async(&self, ids: &[String], retries: i64) -> Result<Batch> {
        self.post("process-instance/job-retries", &json!({"processInstanceIds": ids, "retries": retries})).await
    }
    pub async fn set_external_task_retries_async(&self, ids: &[String], retries: i64) -> Result<Batch> {
        self.post("external-task/retries-async", &json!({"processInstanceIds": ids, "retries": retries})).await
    }
    pub async fn set_variables_async(&self, ids: &[String], variables: &Value) -> Result<Batch> {
        self.post("process-instance/variables-async", &json!({"processInstanceIds": ids, "variables": variables})).await
    }

    // ----- deployments / dmn -----
    pub async fn deployment(&self, id: &str) -> Result<Deployment> {
        self.get(&format!("deployment/{id}"), &[]).await
    }
}

fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn map_reqwest(e: reqwest::Error) -> ApiError {
    if e.is_timeout() {
        ApiError::Timeout
    } else if e.is_decode() {
        ApiError::Decode(e.to_string())
    } else {
        ApiError::Network(e.to_string())
    }
}

fn status_error(status: StatusCode, body: &[u8]) -> ApiError {
    let (kind, message) = match serde_json::from_slice::<Value>(body) {
        Ok(v) => (
            v.get("type").and_then(|t| t.as_str()).unwrap_or("Error").to_string(),
            v.get("message").and_then(|t| t.as_str()).map(String::from).unwrap_or_else(|| String::from_utf8_lossy(body).into_owned()),
        ),
        Err(_) => {
            let text = String::from_utf8_lossy(body).into_owned();
            let text = if text.trim().is_empty() { status.canonical_reason().unwrap_or("error").to_string() } else { text };
            ("HttpError".to_string(), text)
        }
    };
    ApiError::Status { status: status.as_u16(), kind, message }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn base_url_normalisation() {
        assert_eq!(normalize_base_url("localhost:8080"), "http://localhost:8080/engine-rest");
        assert_eq!(normalize_base_url("http://host/engine-rest/"), "http://host/engine-rest");
        assert_eq!(normalize_base_url("https://host:8443/camunda/app/cockpit/default/#/dashboard"), "https://host:8443/engine-rest");
        assert_eq!(normalize_base_url("http://host/camunda/engine-rest"), "http://host/camunda/engine-rest");
    }
    #[test]
    fn query_serialisation_skips_none() {
        let q = ProcessInstanceQuery { business_key_like: Some("%A%".into()), ..Default::default() };
        assert_eq!(serde_json::to_string(&q).unwrap(), r#"{"businessKeyLike":"%A%"}"#);
    }
}
