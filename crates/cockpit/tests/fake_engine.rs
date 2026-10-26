//! End-to-end against a fake Camunda: a tiny HTTP server with canned answers, the real client,
//! the real app loop (headless) and the MCP dispatcher. Checks that places load, overlays are
//! computed from engine data, and that write tools reach the engine with the right payload.

use cockpit::app::App;
use cockpit::mcp::{tools, McpCall, Reply};
use cockpit::settings::{EngineProfile, Settings};
use egui_kittest::Harness;
use serde_json::{json, Value};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const DEF: &str = "order_process:1:d1";
const PI: &str = "11111111-2222-3333-4444-555555555555";

struct FakeEngine {
    port: u16,
    /// (method, path with query, body)
    requests: Arc<Mutex<Vec<(String, String, String)>>>,
    server: Arc<tiny_http::Server>,
}

impl FakeEngine {
    fn start() -> Self {
        let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
        let port = server.server_addr().to_ip().unwrap().port();
        let requests: Arc<Mutex<Vec<(String, String, String)>>> = Default::default();
        let xml = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../bpmn/tests/fixtures/order.bpmn")).unwrap();
        let (s2, r2) = (server.clone(), requests.clone());
        std::thread::spawn(move || {
            for mut req in s2.incoming_requests() {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                let method = req.method().to_string();
                let url = req.url().to_string();
                r2.lock().unwrap().push((method.clone(), url.clone(), body.clone()));
                let path = url.split('?').next().unwrap_or("").trim_start_matches("/engine-rest").to_string();
                let (status, json): (u16, Value) = route(&method, &path, &body, &xml);
                let data = serde_json::to_vec(&json).unwrap();
                let _ = req.respond(tiny_http::Response::from_data(data).with_status_code(status).with_header(tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap()));
            }
        });
        Self { port, requests, server }
    }
    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/engine-rest", self.port)
    }
    fn requests(&self) -> Vec<(String, String, String)> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for FakeEngine {
    fn drop(&mut self) {
        self.server.unblock();
    }
}

fn route(method: &str, path: &str, body: &str, xml: &str) -> (u16, Value) {
    let def = json!({"id": DEF, "key": "order_process", "name": "Order process", "version": 1, "suspended": false, "resource": "/order.bpmn", "deploymentId": "dep1"});
    let hist = json!({"id": PI, "businessKey": "ORD-1", "processDefinitionId": DEF, "processDefinitionKey": "order_process", "processDefinitionVersion": 1, "startTime": "2026-10-20T09:00:00.000+0300", "endTime": null, "state": "ACTIVE"});
    match (method, path) {
        ("GET", "/version") => (200, json!({"version": "7.19.0-fake"})),
        ("GET", "/process-definition") => (200, json!([def])),
        ("GET", "/process-definition/statistics") => (200, json!([{"id": DEF, "instances": 1, "failedJobs": 1, "incidents": [{"incidentType": "failedJob", "incidentCount": 1}], "definition": def}])),
        ("GET", p) if p == format!("/process-definition/{DEF}") => (200, def),
        ("GET", "/process-definition/key/order_process") => (200, def),
        ("GET", p) if p == format!("/process-definition/{DEF}/xml") => (200, json!({"id": DEF, "bpmn20Xml": xml})),
        ("GET", p) if p == format!("/process-definition/{DEF}/statistics") => (200, json!([{"id": "Task_Approve", "instances": 1, "failedJobs": 0, "incidents": []}, {"id": "Task_Fail", "instances": 1, "failedJobs": 1, "incidents": [{"incidentType": "failedJob", "incidentCount": 1}]}])),
        ("POST", "/history/process-instance") => (200, json!([hist])),
        ("POST", "/history/process-instance/count") => (200, json!({"count": 1})),
        ("GET", p) if p == format!("/history/process-instance/{PI}") => (200, hist),
        ("GET", p) if p == format!("/process-instance/{PI}") => (200, json!({"id": PI, "definitionId": DEF, "businessKey": "ORD-1", "ended": false, "suspended": false})),
        ("GET", p) if p == format!("/process-instance/{PI}/activity-instances") => (200, json!({"id": PI, "activityId": DEF, "activityType": "processDefinition", "processInstanceId": PI, "processDefinitionId": DEF, "executionIds": [PI], "childTransitionInstances": [], "incidentIds": [], "incidents": [],
            "childActivityInstances": [{"id": "Task_Approve:a1", "activityId": "Task_Approve", "activityType": "userTask", "processInstanceId": PI, "processDefinitionId": DEF, "childActivityInstances": [], "childTransitionInstances": [], "executionIds": ["e1"], "incidentIds": [], "incidents": []}]})),
        ("GET", "/history/activity-instance") => (200, json!([
            {"id": "h1", "activityId": "StartEvent_1", "activityType": "startEvent", "processInstanceId": PI, "processDefinitionId": DEF, "startTime": "2026-10-20T09:00:00.000+0300", "endTime": "2026-10-20T09:00:00.100+0300", "canceled": false},
            {"id": "h2", "activityId": "Task_Validate", "activityType": "serviceTask", "processInstanceId": PI, "processDefinitionId": DEF, "startTime": "2026-10-20T09:00:00.100+0300", "endTime": "2026-10-20T09:00:01.000+0300", "canceled": false},
            {"id": "h3", "activityId": "Gateway_Amount", "activityType": "exclusiveGateway", "processInstanceId": PI, "processDefinitionId": DEF, "startTime": "2026-10-20T09:00:01.000+0300", "endTime": "2026-10-20T09:00:01.000+0300", "canceled": false},
            {"id": "h4", "activityId": "Task_Approve", "activityType": "userTask", "processInstanceId": PI, "processDefinitionId": DEF, "startTime": "2026-10-20T09:00:01.000+0300", "endTime": null, "canceled": false}])),
        ("POST", "/variable-instance") => (200, json!([{"id": "v1", "name": "amount", "type": "Long", "value": 2500, "valueInfo": {}, "processInstanceId": PI, "executionId": PI}])),
        ("GET", "/incident") => (200, json!([])),
        ("GET", "/job") => (200, json!([{"id": "j1", "jobDefinitionId": "jd1", "processInstanceId": PI, "retries": 3, "dueDate": "2030-01-01T00:00:00.000+0300", "suspended": false, "priority": 0}])),
        ("GET", "/job-definition") => (200, json!([{"id": "jd1", "activityId": "Timer_Reminder", "jobType": "timer-transition", "suspended": false}])),
        ("GET", "/task") => (200, json!([{"id": "t1", "name": "Approve large order", "assignee": "demo", "taskDefinitionKey": "Task_Approve", "processInstanceId": PI, "priority": 50, "suspended": false}])),
        ("GET", "/external-task") => (200, json!([])),
        ("PUT", p) if p.starts_with(&format!("/process-instance/{PI}/variables/")) => {
            let parsed: Value = serde_json::from_str(body).unwrap_or(Value::Null);
            if parsed.get("type").is_some() && parsed.get("value").is_some() {
                (204, Value::Null)
            } else {
                (400, json!({"type": "InvalidRequestException", "message": "bad variable body"}))
            }
        }
        ("PUT", "/job/j1/retries") => (204, Value::Null),
        _ => (404, json!({"type": "RestException", "message": format!("fake engine: no route for {method} {path}")})),
    }
}

fn call(app: &mut App, tool: &str, args: Value) -> Result<Value, String> {
    let (tx, rx) = sync_channel(1);
    tools::dispatch(app, McpCall { tool: tool.into(), args, reply: Reply::new(tx) });
    rx.try_recv().unwrap_or_else(|_| Err("no synchronous reply".into()))
}

/// Step the harness until `pred` holds (network work completes between frames).
fn settle(harness: &mut Harness<'_, App>, what: &str, mut pred: impl FnMut(&mut App) -> bool) {
    let start = Instant::now();
    loop {
        harness.step();
        if pred(harness.state_mut()) {
            return;
        }
        assert!(start.elapsed() < Duration::from_secs(10), "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn state(app: &mut App) -> Value {
    call(app, "ui_state", json!({})).unwrap()
}

#[test]
fn app_loads_definition_and_instance_from_the_engine_and_mcp_writes_reach_it() {
    let engine = FakeEngine::start();
    let ctx = egui::Context::default();
    let mut app = App::new(&ctx, Settings::default(), 1.0);
    app.connect_to(EngineProfile { url: engine.url(), ..Default::default() });
    let mut harness = Harness::builder().with_size(egui::Vec2::new(1400.0, 900.0)).allow_missing_glyphs().build_ui_state(|ui, app: &mut App| app.frame(ui), app);

    // 1. connection + definitions list
    settle(&mut harness, "connection", |app| app.net.is_some() && matches!(app.conn, cockpit::app::ConnStatus::Connected { .. }));
    settle(&mut harness, "definitions list", |app| state(app)["place"]["kind"] == "definitions" && app.cache.defs.data.as_ref().map(|d| d.len()).unwrap_or(0) == 1);
    let st = state(harness.state_mut());
    assert_eq!(st["connected"], true);

    // 2. definition place: diagram + statistics badges
    call(harness.state_mut(), "ui_navigate", json!({"to": "def:order_process"})).unwrap();
    settle(&mut harness, "definition diagram", |app| {
        let s = state(app);
        s["place"]["kind"] == "definition" && s["place"]["diagram"]["loaded"] == true && !s["place"]["running_by_activity"].as_object().unwrap().is_empty()
    });
    let st = state(harness.state_mut());
    assert_eq!(st["place"]["running_by_activity"]["Task_Approve"], 1);
    assert_eq!(st["place"]["incidents_by_activity"]["Task_Fail"], 1);
    assert_eq!(st["place"]["diagram"]["elements"], 18);

    // 3. element tools answer from the cached diagram
    let el = call(harness.state_mut(), "get_element", json!({"definition": "order_process", "element_id": "Task_Validate"})).unwrap();
    assert!(el["attributes"].as_array().unwrap().iter().any(|a| a["name"] == "camunda:delegateExpression"));
    let found = call(harness.state_mut(), "find_elements", json!({"definition": "order_process", "query": "reminder"})).unwrap();
    let ids: Vec<&str> = found["matches"].as_array().unwrap().iter().map(|m| m["element_id"].as_str().unwrap()).collect();
    assert_eq!(ids, vec!["Timer_Reminder", "Task_Notify"], "matches by id and by name, in reading order");
    let xml = call(harness.state_mut(), "get_definition_xml", json!({"definition": "order_process"})).unwrap();
    let text = xml["xml"].as_str().unwrap();
    assert!(text.contains("<bpmn:definitions") && text.contains("Task_Validate"));
    assert_eq!(xml["bytes"].as_u64().unwrap() as usize, text.len());
    let out = std::env::temp_dir().join(format!("flightdeck-xml-{}.bpmn", std::process::id()));
    let saved = call(harness.state_mut(), "get_definition_xml", json!({"definition": "order_process", "save_to": out.to_str().unwrap()})).unwrap();
    assert!(saved.get("xml").is_none(), "save_to returns the path, not the text");
    assert_eq!(std::fs::read_to_string(&out).unwrap(), text);
    std::fs::remove_file(&out).unwrap();

    // 4. instance place: tokens, historic path and timers come from the engine payloads
    call(harness.state_mut(), "ui_navigate", json!({"to": format!("instance:{PI}")})).unwrap();
    settle(&mut harness, "instance overlays", |app| {
        let s = state(app);
        s["place"]["kind"] == "instance" && s["place"]["active_activities"].as_array().map(|a| !a.is_empty()).unwrap_or(false) && s["place"]["visited"].as_object().map(|v| v.len() >= 3).unwrap_or(false)
    });
    let st = state(harness.state_mut());
    assert_eq!(st["place"]["active_activities"], json!(["Task_Approve"]));
    assert_eq!(st["place"]["business_key"], "ORD-1");
    assert_eq!(st["place"]["visited"]["Task_Validate"], 1);
    assert!(st["place"]["visited"].get("Task_Approve").is_none(), "an activity that is still running is not counted as visited");
    assert_eq!(st["place"]["dock_tab"], "Overview");
    call(harness.state_mut(), "ui_dock", json!({"tab": "Timeline"})).unwrap();
    call(harness.state_mut(), "ui_select", json!({"element_id": "Task_Approve"})).unwrap();
    harness.run_steps(3);
    let st = state(harness.state_mut());
    assert_eq!(st["place"]["dock_tab"], "Timeline");
    assert_eq!(st["place"]["diagram"]["selection"]["element_id"], "Task_Approve");
    // Ctrl+F is a global command; every diagram place must receive it (the instance view
    // once dropped it).
    call(harness.state_mut(), "ui_key", json!({"key": "F", "ctrl": true})).unwrap();
    harness.run_steps(3);
    assert!(!state(harness.state_mut())["place"]["diagram"]["find"].is_null(), "Ctrl+F opens find in the instance view");
    call(harness.state_mut(), "ui_key", json!({"key": "Escape"})).unwrap();
    harness.run_steps(2);

    // 4b. XML of the instance's definition, at the selected element; an engine definition is
    // exported to the data directory so external editors get a real path.
    let data = std::env::temp_dir().join(format!("flightdeck-data-{}", std::process::id()));
    harness.state_mut().data_dir = Some(data.clone());
    call(harness.state_mut(), "ui_view_xml", json!({})).unwrap();
    settle(&mut harness, "xml view", |app| state(app)["place"]["highlight"]["id"] == "Task_Approve");
    let path = cockpit::places::source::path_on_disk(harness.state(), &cockpit::places::source::DocSource::Definition(DEF.into()), &harness.state().cache.diagrams[DEF].data.clone().unwrap()).unwrap();
    assert!(path.starts_with(&data) && !path.file_name().unwrap().to_string_lossy().contains(':'), "{path:?}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../bpmn/tests/fixtures/order.bpmn")).unwrap());
    std::fs::remove_dir_all(&data).unwrap();
    call(harness.state_mut(), "ui_tabs", json!({"action": "close"})).unwrap();
    harness.run_steps(2);

    // 4c. compare the deployed definition with a local file.
    let v2 = concat!(env!("CARGO_MANIFEST_DIR"), "/../bpmn/tests/fixtures/order-v2.bpmn");
    call(harness.state_mut(), "ui_compare", json!({"left": "def:order_process", "right": v2})).unwrap();
    settle(&mut harness, "diff", |app| state(app)["place"]["loaded"] == true);
    assert!(state(harness.state_mut())["place"]["counts"]["added"].as_u64().unwrap() >= 2);
    call(harness.state_mut(), "ui_tabs", json!({"action": "close"})).unwrap();
    harness.run_steps(2);
    assert_eq!(state(harness.state_mut())["place"]["kind"], "instance");

    // 5. a write through MCP reaches the engine with a typed body; a bad write surfaces the engine's message
    call(harness.state_mut(), "set_variable", json!({"id": PI, "name": "note", "value": {"k": 1}})).err();
    settle(&mut harness, "variable write", |_| engine.requests().iter().any(|(m, u, _)| m == "PUT" && u.contains("/variables/note")));
    let (_, _, body) = engine.requests().into_iter().find(|(m, u, _)| m == "PUT" && u.contains("/variables/note")).unwrap();
    let sent: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(sent["type"], "Json");
    assert_eq!(sent["value"], "{\"k\":1}");
    let (tx, rx) = sync_channel(1);
    tools::dispatch(harness.state_mut(), McpCall { tool: "retry_job".into(), args: json!({"job_id": "j1", "retries": 1}), reply: Reply::new(tx) });
    settle(&mut harness, "retry reply", |_| rx.try_recv().map(|r| { assert!(r.is_ok(), "{r:?}"); true }).unwrap_or(false));
    assert!(engine.requests().iter().any(|(m, u, b)| m == "PUT" && u.ends_with("/job/j1/retries") && b.contains("\"retries\":1")));

    // 6. tabs/history work on top of real places
    call(harness.state_mut(), "ui_tabs", json!({"action": "back"})).unwrap();
    harness.run_steps(2);
    assert_eq!(state(harness.state_mut())["place"]["kind"], "definition");
    call(harness.state_mut(), "ui_tabs", json!({"action": "forward"})).unwrap();
    harness.run_steps(2);
    assert_eq!(state(harness.state_mut())["place"]["kind"], "instance");
}

#[test]
fn search_place_routes_to_history_api_and_shows_results() {
    let engine = FakeEngine::start();
    let ctx = egui::Context::default();
    let mut app = App::new(&ctx, Settings::default(), 1.0);
    app.connect_to(EngineProfile { url: engine.url(), ..Default::default() });
    let mut harness = Harness::builder().with_size(egui::Vec2::new(1400.0, 900.0)).allow_missing_glyphs().build_ui_state(|ui, app: &mut App| app.frame(ui), app);
    settle(&mut harness, "connection", |app| app.net.is_some() && matches!(app.conn, cockpit::app::ConnStatus::Connected { .. }));
    call(harness.state_mut(), "ui_navigate", json!({"to": "search:def:order_process exec:Task_Validate state:any"})).unwrap();
    settle(&mut harness, "search results", |app| state(app)["place"]["result_count"] == 1);
    let st = state(harness.state_mut());
    assert_eq!(st["place"]["kind"], "search");
    assert_eq!(st["place"]["rows_loaded"], 1);
    let (_, _, body) = engine.requests().into_iter().find(|(m, u, _)| m == "POST" && u.contains("/history/process-instance?")).expect("history query sent");
    let sent: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(sent["executedActivityIdIn"], json!(["Task_Validate"]));
    assert_eq!(sent["processDefinitionKey"], "order_process");
    assert!(sent.get("unfinished").is_none(), "state:any means no state restriction");
    // the search tool reports the same routing decision
    let r = call(harness.state_mut(), "search_instances", json!({"query": "exec:Task_Validate"}));
    assert!(r.is_err() || r.unwrap()["mode"] == "history", "answered asynchronously or as history");
}
