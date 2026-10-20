//! Headless smoke test: boot the real app in offline mode, open the fixture diagram, drive it
//! through the MCP tool dispatcher and make sure nothing panics and state is reported.

use cockpit::app::App;
use cockpit::mcp::{tools, McpCall, Reply};
use cockpit::settings::Settings;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use serde_json::{json, Value};
use std::sync::mpsc::sync_channel;

fn call(app: &mut App, tool: &str, args: Value) -> Result<Value, String> {
    let (tx, rx) = sync_channel(1);
    tools::dispatch(app, McpCall { tool: tool.into(), args, reply: Reply::new(tx) });
    rx.try_recv().unwrap_or_else(|_| Err("no synchronous reply".into()))
}

#[test]
fn offline_viewer_and_ui_tools() {
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/../bpmn/tests/fixtures/order.bpmn");
    let ctx = egui::Context::default();
    let mut app = App::new(&ctx, Settings::default(), 1.0);
    app.offline = true;
    app.navigate(cockpit::state::Nav::File { path: fixture.into() }, cockpit::state::NavMode::NewTab);
    let mut harness = Harness::builder().with_size(egui::Vec2::new(1400.0, 860.0)).allow_missing_glyphs().build_ui_state(|ui, app: &mut App| app.frame(ui), app);
    harness.run_steps(5);
    let st = call(harness.state_mut(), "ui_state", json!({})).unwrap();
    assert_eq!(st["place"]["kind"], "file");
    assert_eq!(st["place"]["diagram"]["loaded"], true);
    assert_eq!(st["place"]["diagram"]["elements"], 18);

    call(harness.state_mut(), "ui_select", json!({"element_id": "Task_Approve"})).unwrap();
    harness.run_steps(3);
    let st = call(harness.state_mut(), "ui_state", json!({})).unwrap();
    assert_eq!(st["place"]["diagram"]["selection"]["element_id"], "Task_Approve");

    let f = call(harness.state_mut(), "ui_find", json!({"query": "timer"})).unwrap();
    assert_eq!(f["count"], 1);
    harness.run_steps(2);
    let st = call(harness.state_mut(), "ui_state", json!({})).unwrap();
    assert_eq!(st["place"]["diagram"]["find"]["matches"], 1);

    // Tab cycles selection along the flow; Escape closes find then clears selection.
    call(harness.state_mut(), "ui_key", json!({"key": "Escape"})).unwrap();
    harness.run_steps(2);
    call(harness.state_mut(), "ui_key", json!({"key": "Escape"})).unwrap();
    harness.run_steps(2);
    let st = call(harness.state_mut(), "ui_state", json!({})).unwrap();
    assert!(st["place"]["diagram"]["selection"].is_null(), "{st}");
    assert!(st["place"]["diagram"]["find"].is_null());

    let z = call(harness.state_mut(), "ui_zoom", json!({"level": 2.0})).unwrap();
    assert_eq!(z["zoom"], 2.0);
    let e = call(harness.state_mut(), "ui_navigate", json!({"to": "nowhere"})).unwrap_err();
    assert!(e.contains("unknown target"));
    assert!(call(harness.state_mut(), "list_definitions", json!({})).is_err(), "engine tools must fail cleanly offline");
    let cmds = call(harness.state_mut(), "ui_commands", json!({})).unwrap();
    assert!(cmds.as_array().unwrap().len() > 20);
    harness.run_steps(2);
}

#[test]
fn theme_picker_does_not_hang() {
    theme_picker_flow(false);
}

/// Same flow but with real wgpu rendering of every frame (catches render-side panics).
#[test]
fn theme_picker_renders_with_wgpu() {
    theme_picker_flow(true);
}

fn theme_picker_flow(render: bool) {
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/../bpmn/tests/fixtures/order.bpmn");
    let ctx = egui::Context::default();
    let mut app = App::new(&ctx, Settings::default(), 1.0);
    app.offline = true;
    app.navigate(cockpit::state::Nav::File { path: fixture.into() }, cockpit::state::NavMode::NewTab);
    let mut builder = Harness::builder().with_size(egui::Vec2::new(1400.0, 860.0)).allow_missing_glyphs();
    if render {
        builder = builder.wgpu();
    }
    let mut harness = builder.build_ui_state(|ui, app: &mut App| app.frame(ui), app);
    harness.run_steps(3);
    if render {
        harness.render().expect("render");
    }
    for mode in ["Light", "Dark", "System"] {
        let btn = harness.query_by_label_contains("☾").or_else(|| harness.query_by_label_contains("☀")).expect("theme button");
        btn.click();
        harness.run_steps(2);
        harness.get_by_label(mode).click();
        harness.run_steps(3);
        if render {
            harness.render().expect("render after theme change");
        }
    }
}
