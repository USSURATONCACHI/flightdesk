//! The MCP server's protocol layer and the tool dispatcher's safety rules, without HTTP.

use cockpit::app::App;
use cockpit::mcp::{handle_message, tool_result, tools, McpCall, Queue, Reply};
use cockpit::settings::Settings;
use serde_json::{json, Value};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};

fn rpc(method: &str, params: Value, id: Option<i64>) -> Value {
    let mut m = json!({"jsonrpc": "2.0", "method": method, "params": params});
    if let Some(i) = id {
        m["id"] = json!(i);
    }
    m
}

#[test]
fn initialize_negotiates_a_supported_protocol_version() {
    let q: Queue = Default::default();
    let ctx = egui::Context::default();
    let stats = Arc::new(Mutex::new(Default::default()));
    let r = handle_message(&rpc("initialize", json!({"protocolVersion": "2025-03-26"}), Some(1)), &q, &ctx, &stats).unwrap();
    assert_eq!(r["result"]["protocolVersion"], "2025-03-26", "known versions are echoed");
    let r = handle_message(&rpc("initialize", json!({"protocolVersion": "1999-01-01"}), Some(2)), &q, &ctx, &stats).unwrap();
    assert_eq!(r["result"]["protocolVersion"], cockpit::mcp::PROTOCOL_VERSION, "unknown versions get our latest");
    assert_eq!(r["id"], 2);
    assert!(r["result"]["serverInfo"]["name"] == "flightdeck");
}

#[test]
fn notifications_get_no_response_and_unknown_methods_an_error() {
    let q: Queue = Default::default();
    let ctx = egui::Context::default();
    let stats = Arc::new(Mutex::new(Default::default()));
    assert!(handle_message(&rpc("notifications/initialized", json!({}), None), &q, &ctx, &stats).is_none());
    let r = handle_message(&rpc("bogus/method", json!({}), Some(7)), &q, &ctx, &stats).unwrap();
    assert_eq!(r["error"]["code"], -32601);
    assert_eq!(r["id"], 7);
    let r = handle_message(&rpc("ping", json!({}), Some(8)), &q, &ctx, &stats).unwrap();
    assert_eq!(r["result"], json!({}));
}

#[test]
fn tool_catalogue_is_well_formed() {
    let specs = tools::specs();
    assert!(specs.len() >= 50);
    let mut names = std::collections::HashSet::new();
    for s in &specs {
        let name = s["name"].as_str().unwrap();
        assert!(names.insert(name), "duplicate tool {name}");
        assert!(!s["description"].as_str().unwrap().is_empty());
        let schema = &s["inputSchema"];
        assert_eq!(schema["type"], "object", "{name}");
        for req in schema["required"].as_array().unwrap() {
            assert!(schema["properties"].get(req.as_str().unwrap()).is_some(), "{name}: required `{req}` is not a property");
        }
        assert!(tools::exists(name));
    }
    assert!(!tools::exists("rm_rf"));
    // destructive tools require an explicit confirm flag
    for t in ["delete_instance", "modify_instance", "restart_instance", "batch", "complete_task", "resolve_incident", "suspend_definition", "delete_variable"] {
        let s = specs.iter().find(|s| s["name"] == t).unwrap();
        assert!(s["inputSchema"]["required"].as_array().unwrap().iter().any(|r| r == "confirm"), "{t} must require confirm");
    }
}

#[test]
fn unknown_tools_are_reported_as_tool_errors_not_protocol_errors() {
    let q: Queue = Default::default();
    let ctx = egui::Context::default();
    let stats = Arc::new(Mutex::new(Default::default()));
    let r = handle_message(&rpc("tools/call", json!({"name": "nope", "arguments": {}}), Some(1)), &q, &ctx, &stats).unwrap();
    assert_eq!(r["result"]["isError"], true);
    assert!(q.lock().unwrap().is_empty(), "nothing was queued for the UI thread");
    assert_eq!(stats.lock().unwrap().errors, 1);
}

#[test]
fn tool_results_become_text_or_image_content() {
    let r = tool_result(Ok(json!({"a": 1})));
    assert_eq!(r["isError"], false);
    assert_eq!(r["content"][0]["type"], "text");
    assert_eq!(r["structuredContent"]["a"], 1);
    let r = tool_result(Ok(json!("plain string")));
    assert_eq!(r["content"][0]["text"], "plain string");
    assert!(r.get("structuredContent").is_none());
    let r = tool_result(Ok(json!({"_png_base64": "AAAA", "width": 2})));
    assert_eq!(r["content"][0]["type"], "image");
    assert_eq!(r["content"][0]["mimeType"], "image/png");
    assert!(r["content"][1]["text"].as_str().unwrap().contains("\"width\":2"));
    assert!(!r["content"][1]["text"].as_str().unwrap().contains("_png_base64"), "the payload is not echoed as text");
    let r = tool_result(Err("boom".into()));
    assert_eq!(r["isError"], true);
    assert_eq!(r["content"][0]["text"], "boom");
}

fn offline_app() -> App {
    let ctx = egui::Context::default();
    let mut app = App::new(&ctx, Settings::default(), 1.0);
    app.offline = true;
    app
}

fn call(app: &mut App, tool: &str, args: Value) -> Result<Value, String> {
    let (tx, rx) = sync_channel(1);
    tools::dispatch(app, McpCall { tool: tool.into(), args, reply: Reply::new(tx) });
    rx.try_recv().unwrap_or_else(|_| Err("no synchronous reply".into()))
}

#[test]
fn destructive_tools_refuse_without_confirm_before_touching_the_network() {
    let mut app = offline_app();
    for (tool, args) in [
        ("delete_instance", json!({"id": "x"})),
        ("delete_variable", json!({"id": "x", "name": "v"})),
        ("modify_instance", json!({"id": "x", "instructions": [{"type": "cancel", "activityId": "a"}]})),
        ("batch", json!({"operation": "delete", "instance_ids": ["x"]})),
        ("complete_task", json!({"task_id": "t"})),
        ("resolve_incident", json!({"incident_id": "i"})),
    ] {
        let e = call(&mut app, tool, args).unwrap_err();
        assert!(e.contains("confirm"), "{tool}: {e}");
    }
    // with confirm but no engine, the answer is a clear "not connected", still no panic
    let e = call(&mut app, "delete_instance", json!({"id": "x", "confirm": true})).unwrap_err();
    assert!(e.contains("not connected"));
    // required arguments are validated before anything else
    let e = call(&mut app, "set_variable", json!({"id": "x"})).unwrap_err();
    assert!(e.contains("missing argument"));
}

#[test]
fn ui_tools_report_the_app_state_and_validate_input() {
    let mut app = offline_app();
    let st = call(&mut app, "ui_state", json!({})).unwrap();
    assert_eq!(st["connected"], false);
    assert_eq!(st["tabs"].as_array().unwrap().len(), 0);
    assert!(call(&mut app, "ui_navigate", json!({"to": "mars"})).unwrap_err().contains("unknown target"));
    assert!(call(&mut app, "ui_select", json!({"element_id": "X"})).unwrap_err().contains("no diagram"));
    assert!(call(&mut app, "ui_key", json!({"key": "NoSuchKey"})).unwrap_err().contains("unknown key"));
    call(&mut app, "ui_key", json!({"key": "Escape"})).unwrap();
    assert_eq!(app.synthetic_events.len(), 2, "a press and a release are queued for the next frame");
    let s = call(&mut app, "ui_settings", json!({"set": {"theme": "light", "page_size": 25}})).unwrap();
    assert_eq!(s["theme"], "Light");
    assert_eq!(s["page_size"], 25);
    assert!(call(&mut app, "ui_settings", json!({"set": {"nonsense": 1}})).is_err());
    let cmds = call(&mut app, "ui_commands", json!({})).unwrap();
    let back = cmds.as_array().unwrap().iter().find(|c| c["name"] == "Back").unwrap();
    assert_eq!(back["enabled"], false, "no history yet");
    assert!(call(&mut app, "ui_command", json!({"name": "back"})).unwrap_err().contains("not enabled"));
    assert!(call(&mut app, "ui_confirm", json!({"accept": true})).unwrap_err().contains("no pending"));
}
