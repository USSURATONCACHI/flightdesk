//! Runs against a live engine when `CAMUNDA_URL` is set (e.g. http://localhost:8089/engine-rest).
use camunda::*;

fn client() -> Option<Client> {
    let url = std::env::var("CAMUNDA_URL").ok()?;
    Some(Client::new(Config { base_url: url, ..Default::default() }).unwrap())
}

#[tokio::test]
async fn read_everything() {
    let Some(c) = client() else { return };
    let v = c.version().await.unwrap();
    assert!(v.version.starts_with('7'));
    let defs = c.process_definitions(true).await.unwrap();
    assert!(!defs.is_empty());
    let stats = c.process_definition_statistics().await.unwrap();
    assert_eq!(stats.len(), c.process_definitions(false).await.unwrap().len());
    let order = defs.iter().find(|d| d.key == "order_process").expect("order_process deployed");
    let xml = c.process_definition_xml(&order.id).await.unwrap();
    assert!(xml.contains("<bpmn:definitions"));
    let act = c.activity_statistics(&order.id).await.unwrap();
    assert!(act.iter().any(|a| a.id == "Task_Approve" && a.instances >= 1));
    let pis = c.process_instances(&ProcessInstanceQuery { process_definition_key: Some("order_process".into()), ..Default::default() }, 0, 50).await.unwrap();
    assert!(pis.len() >= 5);
    let n = c.process_instances_count(&ProcessInstanceQuery { business_key_like: Some("ORD-2%".into()), ..Default::default() }).await.unwrap();
    assert_eq!(n, 3);
    let hist = c.history_process_instances(&HistoricProcessInstanceQuery { finished: Some(true), ..Default::default() }, 0, 10).await.unwrap();
    assert!(hist.iter().any(|h| h.business_key.as_deref() == Some("ORD-6001") && h.state.as_deref() == Some("COMPLETED")));
    let byvar = c
        .history_process_instances(&HistoricProcessInstanceQuery { variables: Some(vec![VariableCriterion { name: "amount".into(), operator: "gt".into(), value: serde_json::json!(4000) }]), ..Default::default() }, 0, 10)
        .await
        .unwrap();
    assert!(byvar.len() >= 2);
    let pi = pis.iter().find(|p| p.business_key.as_deref() == Some("ORD-2001")).unwrap();
    let ai = c.activity_instances(&pi.id).await.unwrap();
    let tokens = ai.tokens();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].activity_id, "Task_Approve");
    let vars = c.variables(&pi.id).await.unwrap();
    assert!(vars.iter().any(|v| v.name == "amount" && v.type_.as_deref() == Some("Long")));
    let hv = c.history_variables(&pi.id).await.unwrap();
    assert!(hv.len() >= vars.len());
    let ha = c.history_activity_instances(&pi.id).await.unwrap();
    assert!(!ha.is_empty());
    let tasks = c.tasks(&[("processInstanceId", pi.id.clone())]).await.unwrap();
    assert_eq!(tasks[0].task_definition_key.as_deref(), Some("Task_Approve"));
    let links = c.task_identity_links(&tasks[0].id).await.unwrap();
    assert!(links.iter().any(|l| l.group_id.as_deref() == Some("finance")));
    let jobs = c.jobs(&[("processInstanceId", pi.id.clone())]).await.unwrap();
    assert!(jobs.iter().any(|j| j.due_date.is_some()), "timer job expected");
    let inc = c.incidents(&[]).await.unwrap();
    assert!(!inc.is_empty());
    let trace = c.job_stacktrace(inc[0].configuration.as_deref().unwrap()).await.unwrap();
    assert!(trace.contains("Exception") || trace.contains("boom"));
    let et = c.external_tasks(&[("topicName", "stock".to_string())]).await.unwrap();
    assert!(et.len() >= 2);
    let hinc = c.history_incidents(inc[0].process_instance_id.as_deref().unwrap()).await.unwrap();
    assert!(!hinc.is_empty());
    let b = c.batches().await.unwrap();
    let _ = b;
    // 404 maps to a status error with the engine's message
    let err = c.process_instance("does-not-exist").await.unwrap_err();
    assert_eq!(err.status(), Some(404));
    assert!(err.to_string().contains("does-not-exist"));
}

#[tokio::test]
async fn write_roundtrip() {
    let Some(c) = client() else { return };
    let pis = c.process_instances(&ProcessInstanceQuery { business_key: Some("ORD-4001".into()), ..Default::default() }, 0, 1).await.unwrap();
    let pi = &pis[0];
    c.put_process_variable(&pi.id, "note", &VariableValue::string("set by test")).await.unwrap();
    let vars = c.variables(&pi.id).await.unwrap();
    assert!(vars.iter().any(|v| v.name == "note" && v.value == serde_json::json!("set by test")));
    c.put_process_variable(&pi.id, "payload", &VariableValue::new("Json", serde_json::json!(r#"{"a":[1,2,3],"b":{"c":"d"}}"#))).await.unwrap();
    c.delete_process_variable(&pi.id, "note").await.unwrap();
    c.suspend_instance(&pi.id, true).await.unwrap();
    assert!(c.process_instance(&pi.id).await.unwrap().suspended);
    c.suspend_instance(&pi.id, false).await.unwrap();
}
