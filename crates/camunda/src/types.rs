//! Camunda 7 REST DTOs. Fields are `Option` wherever the engine may send `null`.
#![allow(clippy::struct_excessive_bools)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProcessDefinition {
    pub id: String,
    pub key: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub version: i64,
    pub resource: Option<String>,
    pub deployment_id: Option<String>,
    pub diagram: Option<String>,
    pub suspended: bool,
    pub tenant_id: Option<String>,
    pub version_tag: Option<String>,
    pub history_time_to_live: Option<i64>,
    pub startable_in_tasklist: bool,
}

impl ProcessDefinition {
    pub fn display_name(&self) -> &str {
        self.name.as_deref().filter(|n| !n.is_empty()).unwrap_or(&self.key)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct IncidentStatistics {
    pub incident_type: String,
    pub incident_count: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProcessDefinitionStatistics {
    pub id: String,
    pub instances: i64,
    pub failed_jobs: i64,
    pub incidents: Vec<IncidentStatistics>,
    pub definition: ProcessDefinition,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ActivityStatistics {
    pub id: String,
    pub instances: i64,
    pub failed_jobs: i64,
    pub incidents: Vec<IncidentStatistics>,
}

impl ActivityStatistics {
    pub fn incident_count(&self) -> i64 {
        self.incidents.iter().map(|i| i.incident_count).sum()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProcessDefinitionDiagram {
    pub id: String,
    pub bpmn20_xml: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProcessInstance {
    pub id: String,
    pub definition_id: String,
    pub business_key: Option<String>,
    pub case_instance_id: Option<String>,
    pub ended: bool,
    pub suspended: bool,
    pub tenant_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoricProcessInstance {
    pub id: String,
    pub business_key: Option<String>,
    pub process_definition_id: String,
    pub process_definition_key: Option<String>,
    pub process_definition_name: Option<String>,
    pub process_definition_version: Option<i64>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub removal_time: Option<String>,
    pub duration_in_millis: Option<i64>,
    pub start_user_id: Option<String>,
    pub start_activity_id: Option<String>,
    pub delete_reason: Option<String>,
    pub root_process_instance_id: Option<String>,
    pub super_process_instance_id: Option<String>,
    pub tenant_id: Option<String>,
    /// ACTIVE, SUSPENDED, COMPLETED, EXTERNALLY_TERMINATED, INTERNALLY_TERMINATED
    pub state: Option<String>,
}

impl HistoricProcessInstance {
    pub fn is_finished(&self) -> bool {
        self.end_time.is_some()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ActivityInstance {
    pub id: String,
    pub parent_activity_instance_id: Option<String>,
    pub activity_id: String,
    pub activity_name: Option<String>,
    pub activity_type: Option<String>,
    pub process_instance_id: String,
    pub process_definition_id: String,
    pub child_activity_instances: Vec<ActivityInstance>,
    pub child_transition_instances: Vec<TransitionInstance>,
    pub execution_ids: Vec<String>,
    pub incident_ids: Vec<String>,
    pub incidents: Vec<ActivityInstanceIncident>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ActivityInstanceIncident {
    pub id: String,
    pub activity_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct TransitionInstance {
    pub id: String,
    pub parent_activity_instance_id: Option<String>,
    pub activity_id: String,
    pub activity_name: Option<String>,
    pub activity_type: Option<String>,
    pub process_instance_id: String,
    pub process_definition_id: String,
    pub execution_id: Option<String>,
    pub incident_ids: Vec<String>,
}

/// A flattened token: an activity instance or a transition instance (async continuation).
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub id: String,
    pub activity_id: String,
    pub activity_type: Option<String>,
    pub execution_ids: Vec<String>,
    pub incident_ids: Vec<String>,
    pub is_transition: bool,
    /// Ids of the ancestor activity instances (innermost last), for scope display.
    pub path: Vec<String>,
}

impl ActivityInstance {
    /// Flatten the tree into leaf tokens (activity instances without children, plus transition
    /// instances). Scope activities with children are not tokens themselves.
    pub fn tokens(&self) -> Vec<Token> {
        let mut out = vec![];
        self.collect_tokens(&mut vec![], &mut out, true);
        out
    }
    fn collect_tokens(&self, path: &mut Vec<String>, out: &mut Vec<Token>, is_root: bool) {
        let is_leaf = self.child_activity_instances.is_empty() && self.child_transition_instances.is_empty();
        if is_leaf && !is_root {
            out.push(Token {
                id: self.id.clone(),
                activity_id: self.activity_id.clone(),
                activity_type: self.activity_type.clone(),
                execution_ids: self.execution_ids.clone(),
                incident_ids: self.incident_ids.clone(),
                is_transition: false,
                path: path.clone(),
            });
            return;
        }
        if !is_root {
            path.push(self.activity_id.clone());
        }
        for t in &self.child_transition_instances {
            out.push(Token {
                id: t.id.clone(),
                activity_id: t.activity_id.clone(),
                activity_type: t.activity_type.clone(),
                execution_ids: t.execution_id.iter().cloned().collect(),
                incident_ids: t.incident_ids.clone(),
                is_transition: true,
                path: path.clone(),
            });
        }
        for c in &self.child_activity_instances {
            c.collect_tokens(path, out, false);
        }
        if !is_root {
            path.pop();
        }
    }
    /// All activity instances (including scopes) with their activity ids, depth-first.
    pub fn all(&self) -> Vec<&ActivityInstance> {
        let mut v = vec![self];
        for c in &self.child_activity_instances {
            v.extend(c.all());
        }
        v
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoricActivityInstance {
    pub id: String,
    pub parent_activity_instance_id: Option<String>,
    pub activity_id: String,
    pub activity_name: Option<String>,
    pub activity_type: Option<String>,
    pub process_definition_key: Option<String>,
    pub process_definition_id: String,
    pub process_instance_id: String,
    pub execution_id: Option<String>,
    pub task_id: Option<String>,
    pub called_process_instance_id: Option<String>,
    pub assignee: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub duration_in_millis: Option<i64>,
    pub canceled: bool,
    pub complete_scope: bool,
    pub tenant_id: Option<String>,
}

impl HistoricActivityInstance {
    /// `Task_Reserve#multiInstanceBody` → `Task_Reserve`
    pub fn base_activity_id(&self) -> &str {
        self.activity_id.split('#').next().unwrap_or(&self.activity_id)
    }
    pub fn is_multi_instance_body(&self) -> bool {
        self.activity_id.ends_with("#multiInstanceBody")
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct VariableValue {
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub value: Value,
    pub value_info: Value,
}

impl VariableValue {
    pub fn new(type_: &str, value: Value) -> Self {
        Self { type_: Some(type_.to_string()), value, value_info: Value::Object(Default::default()) }
    }
    pub fn string(s: &str) -> Self {
        Self::new("String", Value::String(s.to_string()))
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct VariableInstance {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub value: Value,
    pub value_info: Value,
    pub process_definition_id: Option<String>,
    pub process_instance_id: Option<String>,
    pub execution_id: Option<String>,
    pub task_id: Option<String>,
    pub batch_id: Option<String>,
    pub activity_instance_id: Option<String>,
    pub error_message: Option<String>,
    pub tenant_id: Option<String>,
    // history only
    pub state: Option<String>,
    pub create_time: Option<String>,
    pub removal_time: Option<String>,
}

impl VariableInstance {
    pub fn as_value(&self) -> VariableValue {
        VariableValue { type_: self.type_.clone(), value: self.value.clone(), value_info: self.value_info.clone() }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Incident {
    pub id: String,
    pub process_definition_id: Option<String>,
    pub process_instance_id: Option<String>,
    pub execution_id: Option<String>,
    pub incident_timestamp: Option<String>,
    pub create_time: Option<String>,
    pub end_time: Option<String>,
    pub incident_type: String,
    pub activity_id: Option<String>,
    pub failed_activity_id: Option<String>,
    pub cause_incident_id: Option<String>,
    pub root_cause_incident_id: Option<String>,
    /// Job id for failedJob, external task id for failedExternalTask.
    pub configuration: Option<String>,
    pub incident_message: Option<String>,
    pub tenant_id: Option<String>,
    pub job_definition_id: Option<String>,
    pub annotation: Option<String>,
    pub open: Option<bool>,
    pub deleted: Option<bool>,
    pub resolved: Option<bool>,
    pub process_definition_key: Option<String>,
    pub root_process_instance_id: Option<String>,
}

impl Incident {
    pub fn time(&self) -> Option<&str> {
        self.incident_timestamp.as_deref().or(self.create_time.as_deref())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Job {
    pub id: String,
    pub job_definition_id: Option<String>,
    pub process_instance_id: Option<String>,
    pub process_definition_id: Option<String>,
    pub process_definition_key: Option<String>,
    pub execution_id: Option<String>,
    pub exception_message: Option<String>,
    pub failed_activity_id: Option<String>,
    pub retries: i64,
    pub due_date: Option<String>,
    pub suspended: bool,
    pub priority: i64,
    pub tenant_id: Option<String>,
    pub create_time: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct JobDefinition {
    pub id: String,
    pub process_definition_id: Option<String>,
    pub process_definition_key: Option<String>,
    pub activity_id: Option<String>,
    pub job_type: Option<String>,
    pub job_configuration: Option<String>,
    pub suspended: bool,
    pub override_job_priority: Option<i64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ExternalTask {
    pub id: String,
    pub activity_id: Option<String>,
    pub activity_instance_id: Option<String>,
    pub error_message: Option<String>,
    pub execution_id: Option<String>,
    pub lock_expiration_time: Option<String>,
    pub process_definition_id: Option<String>,
    pub process_definition_key: Option<String>,
    pub process_instance_id: Option<String>,
    pub retries: Option<i64>,
    pub suspended: bool,
    pub worker_id: Option<String>,
    pub topic_name: Option<String>,
    pub tenant_id: Option<String>,
    pub priority: i64,
    pub business_key: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Task {
    pub id: String,
    pub name: Option<String>,
    pub assignee: Option<String>,
    pub owner: Option<String>,
    pub created: Option<String>,
    pub due: Option<String>,
    pub follow_up: Option<String>,
    pub last_updated: Option<String>,
    pub delegation_state: Option<String>,
    pub description: Option<String>,
    pub execution_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub priority: i64,
    pub process_definition_id: Option<String>,
    pub process_instance_id: Option<String>,
    pub task_definition_key: Option<String>,
    pub suspended: bool,
    pub form_key: Option<String>,
    pub tenant_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct IdentityLink {
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Batch {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub total_jobs: i64,
    pub jobs_created: i64,
    pub batch_jobs_per_seed: i64,
    pub invocations_per_batch_job: i64,
    pub seed_job_definition_id: Option<String>,
    pub monitor_job_definition_id: Option<String>,
    pub batch_job_definition_id: Option<String>,
    pub suspended: bool,
    pub tenant_id: Option<String>,
    pub create_user_id: Option<String>,
    pub start_time: Option<String>,
    pub execution_start_time: Option<String>,
    // statistics endpoint
    pub remaining_jobs: Option<i64>,
    pub completed_jobs: Option<i64>,
    pub failed_jobs: Option<i64>,
    // history endpoint
    pub end_time: Option<String>,
    pub removal_time: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Count {
    pub count: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Version {
    pub version: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct UserProfile {
    pub id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Deployment {
    pub id: String,
    pub name: Option<String>,
    pub source: Option<String>,
    pub deployment_time: Option<String>,
    pub tenant_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DecisionDefinition {
    pub id: String,
    pub key: String,
    pub name: Option<String>,
    pub version: i64,
    pub decision_requirements_definition_id: Option<String>,
    pub tenant_id: Option<String>,
    pub version_tag: Option<String>,
}

/// One instruction of a process instance modification.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModificationInstruction {
    /// startBeforeActivity | startAfterActivity | startTransition | cancel
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ancestor_activity_instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_current_active_activity_instances: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<serde_json::Map<String, Value>>,
}

impl ModificationInstruction {
    pub fn start_before(activity_id: &str) -> Self {
        Self { type_: "startBeforeActivity".into(), activity_id: Some(activity_id.into()), transition_id: None, activity_instance_id: None, transition_instance_id: None, ancestor_activity_instance_id: None, cancel_current_active_activity_instances: None, variables: None }
    }
    pub fn start_after(activity_id: &str) -> Self {
        Self { type_: "startAfterActivity".into(), ..Self::start_before(activity_id) }
    }
    pub fn cancel_activity(activity_id: &str) -> Self {
        Self { type_: "cancel".into(), ..Self::start_before(activity_id) }
    }
    pub fn cancel_instance(activity_instance_id: &str) -> Self {
        Self { type_: "cancel".into(), activity_id: None, activity_instance_id: Some(activity_instance_id.into()), ..Self::start_before("") }
    }
    pub fn cancel_transition(transition_instance_id: &str) -> Self {
        Self { type_: "cancel".into(), activity_id: None, transition_instance_id: Some(transition_instance_id.into()), ..Self::start_before("") }
    }
    pub fn describe(&self) -> String {
        match self.type_.as_str() {
            "startBeforeActivity" => format!("Start before {}", self.activity_id.as_deref().unwrap_or("?")),
            "startAfterActivity" => format!("Start after {}", self.activity_id.as_deref().unwrap_or("?")),
            "startTransition" => format!("Start transition {}", self.transition_id.as_deref().unwrap_or("?")),
            "cancel" => {
                if let Some(a) = &self.activity_id {
                    format!("Cancel all tokens at {a}")
                } else if let Some(i) = &self.activity_instance_id {
                    format!("Cancel token {}", short_id(i))
                } else if let Some(i) = &self.transition_instance_id {
                    format!("Cancel transition {}", short_id(i))
                } else {
                    "Cancel".to_string()
                }
            }
            other => other.to_string(),
        }
    }
}

/// `Task_Approve:30f87682-…` → `Task_Approve:30f8`; plain uuid → first 8 chars.
pub fn short_id(id: &str) -> String {
    if let Some((a, b)) = id.rsplit_once(':') {
        if b.len() >= 8 {
            return format!("{a}:{}", &b[..8]);
        }
    }
    if id.len() > 8 && id.chars().filter(|c| *c == '-').count() == 4 {
        return id[..8].to_string();
    }
    id.to_string()
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct MessageCorrelationResult {
    pub result_type: Option<String>,
    pub process_instance: Option<ProcessInstance>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokens_flatten() {
        let json = r#"{"id":"pi","parentActivityInstanceId":null,"activityId":"def","activityType":"processDefinition","processInstanceId":"pi","processDefinitionId":"d","childActivityInstances":[{"id":"Task_A:1","parentActivityInstanceId":"pi","activityId":"Task_A","activityType":"userTask","processInstanceId":"pi","processDefinitionId":"d","childActivityInstances":[],"childTransitionInstances":[],"executionIds":["e1"],"activityName":"A","incidentIds":[],"incidents":[],"name":"A"},{"id":"Sub:1","parentActivityInstanceId":"pi","activityId":"Sub","activityType":"subProcess","processInstanceId":"pi","processDefinitionId":"d","childActivityInstances":[{"id":"Task_B:1","parentActivityInstanceId":"Sub:1","activityId":"Task_B","activityType":"serviceTask","processInstanceId":"pi","processDefinitionId":"d","childActivityInstances":[],"childTransitionInstances":[],"executionIds":["e2"],"incidentIds":["inc1"],"incidents":[]}],"childTransitionInstances":[{"id":"t1","parentActivityInstanceId":"Sub:1","activityId":"Task_C","activityType":"serviceTask","processInstanceId":"pi","processDefinitionId":"d","executionId":"e3","incidentIds":[]}],"executionIds":["e4"],"incidentIds":[],"incidents":[]}],"childTransitionInstances":[],"executionIds":["pi"],"incidentIds":[],"incidents":[]}"#;
        let ai: ActivityInstance = serde_json::from_str(json).unwrap();
        let t = ai.tokens();
        assert_eq!(t.len(), 3);
        assert_eq!(t[0].activity_id, "Task_A");
        assert!(t[1].is_transition && t[1].activity_id == "Task_C" && t[1].path == vec!["Sub"]);
        assert_eq!(t[2].activity_id, "Task_B");
        assert_eq!(t[2].incident_ids, vec!["inc1"]);
    }
    #[test]
    fn short_ids() {
        assert_eq!(short_id("Task_Approve:30f87682-cc52-11f1-8f3a-c6104db61f7c"), "Task_Approve:30f87682");
        assert_eq!(short_id("30f87682-cc52-11f1-8f3a-c6104db61f7c"), "30f87682");
        assert_eq!(short_id("abc"), "abc");
    }
}
