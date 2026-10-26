//! Headless UI tests for the XML view, the visual diff and documentation blocks: the real app
//! loop driven through the MCP dispatcher and synthetic input, on the synthetic fixtures.

use cockpit::app::App;
use cockpit::mcp::{tools, McpCall, Reply};
use cockpit::settings::Settings;
use cockpit::state::{Nav, NavMode};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::mpsc::sync_channel;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../bpmn/tests/fixtures")).join(name)
}

fn call(app: &mut App, tool: &str, args: Value) -> Result<Value, String> {
    let (tx, rx) = sync_channel(1);
    tools::dispatch(app, McpCall { tool: tool.into(), args, reply: Reply::new(tx) });
    rx.try_recv().unwrap_or_else(|_| Err("no synchronous reply".into()))
}

fn place(app: &mut App) -> Value {
    call(app, "ui_state", json!({})).unwrap()["place"].clone()
}

fn harness(nav: Nav) -> Harness<'static, App> {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../locales"));
    cockpit::i18n::load(&[dir], "en-US");
    let ctx = egui::Context::default();
    let mut app = App::new(&ctx, Settings::default(), 1.0);
    app.offline = true;
    app.navigate(nav, NavMode::NewTab);
    let mut h = Harness::builder().with_size(egui::Vec2::new(1400.0, 860.0)).allow_missing_glyphs().build_ui_state(|ui, app: &mut App| app.frame(ui), app);
    h.run_steps(4);
    h
}

fn diff_nav() -> Nav {
    use cockpit::places::source::DocSource;
    Nav::Diff { left: Some(DocSource::File(fixture("order.bpmn"))), right: Some(DocSource::File(fixture("order-v2.bpmn"))) }
}

/// Line and column (1-based, characters) where `needle` starts in a fixture.
fn position(file: &str, needle: &str) -> (u64, u64) {
    let src = std::fs::read_to_string(fixture(file)).unwrap();
    let at = src.find(needle).unwrap();
    let line = src[..at].matches('\n').count() + 1;
    let col = src[..at].rsplit('\n').next().unwrap().chars().count() + 1;
    (line as u64, col as u64)
}

#[test]
fn xml_view_jumps_to_the_element_highlights_it_and_is_reused() {
    let mut h = harness(Nav::File { path: fixture("order.bpmn") });
    call(h.state_mut(), "ui_select", json!({"element_id": "Task_Script"})).unwrap();
    let r = call(h.state_mut(), "ui_view_xml", json!({})).unwrap();
    assert_eq!(r["element"], "Task_Script", "defaults to the selection");
    h.run_steps(4);
    let p = place(h.state_mut());
    assert_eq!(p["kind"], "xml");
    assert_eq!(p["diff"], false);
    let (line, col) = position("order.bpmn", "<bpmn:scriptTask id=\"Task_Script\"");
    assert_eq!(p["highlight"]["id"], "Task_Script");
    assert_eq!((p["highlight"]["line"].as_u64().unwrap(), p["highlight"]["col"].as_u64().unwrap()), (line, col));
    assert_eq!(h.state().tabs.len(), 2);

    // Escape removes the highlight.
    call(h.state_mut(), "ui_key", json!({"key": "Escape"})).unwrap();
    h.run_steps(3);
    assert!(place(h.state_mut())["highlight"].is_null());

    // Back on the diagram, Ctrl+U for another element switches to the same XML tab.
    call(h.state_mut(), "ui_tabs", json!({"action": "activate", "index": 0})).unwrap();
    h.run_steps(2);
    call(h.state_mut(), "ui_select", json!({"element_id": "Flow_Large"})).unwrap();
    call(h.state_mut(), "ui_key", json!({"key": "U", "ctrl": true})).unwrap();
    h.run_steps(5);
    assert_eq!(h.state().tabs.len(), 2, "the open XML tab is reused");
    let p = place(h.state_mut());
    assert_eq!(p["kind"], "xml");
    assert_eq!(p["highlight"]["id"], "Flow_Large", "flows can be shown too");

    // Find counts every occurrence, case-insensitively.
    let src = std::fs::read_to_string(fixture("order.bpmn")).unwrap();
    let r = call(h.state_mut(), "ui_xml", json!({"find": "task_approve"})).unwrap();
    assert_eq!(r["find"]["matches"].as_u64().unwrap() as usize, src.to_lowercase().matches("task_approve").count());
}

#[test]
fn show_in_diagram_from_xml_selects_the_element_in_the_diagram_tab() {
    let mut h = harness(Nav::File { path: fixture("order.bpmn") });
    call(h.state_mut(), "ui_view_xml", json!({"element_id": "Task_Ship"})).unwrap();
    h.run_steps(4);
    assert_eq!(place(h.state_mut())["kind"], "xml");
    // The toolbar button goes back to the diagram tab and selects the highlighted element.
    h.get_by_label("Show diagram").click();
    h.run_steps(6);
    let p = place(h.state_mut());
    assert_eq!(p["kind"], "file");
    assert_eq!(p["diagram"]["selection"]["element_id"], "Task_Ship");
    assert_eq!(h.state().tabs.len(), 2, "the existing diagram tab is reused");
}

#[test]
fn diff_marks_every_kind_of_change_and_steps_through_them() {
    let mut h = harness(diff_nav());
    h.run_steps(3);
    let p = place(h.state_mut());
    assert_eq!(p["kind"], "diff");
    assert_eq!(p["loaded"], true, "{p}");
    let c = &p["counts"];
    assert!(c["added"].as_u64().unwrap() >= 2 && c["removed"].as_u64().unwrap() >= 2 && c["changed"].as_u64().unwrap() >= 3 && c["moved"].as_u64().unwrap() >= 1, "{c}");
    let total = p["visible_changes"].as_u64().unwrap();
    assert_eq!(p["marked"].as_u64().unwrap(), total, "every change is marked in the unified view");

    // n steps through the changes in list order; each one is selected.
    call(h.state_mut(), "ui_key", json!({"key": "N"})).unwrap();
    h.run_steps(3);
    let first = place(h.state_mut());
    assert_eq!(first["selection"]["id"], first["current_change"]);
    let d = call(h.state_mut(), "ui_diff", json!({"step": "next"})).unwrap();
    assert_ne!(d["current_change"], first["current_change"]);

    // A removed element exists only on the before side: the after view has nothing selected,
    // the before view selects it, and side by side carries it over.
    call(h.state_mut(), "ui_diff", json!({"mode": "before"})).unwrap();
    call(h.state_mut(), "ui_select", json!({"element_id": "Task_Notify"})).unwrap();
    h.run_steps(3);
    let p = place(h.state_mut());
    assert_eq!(p["selection"]["status"], "removed");
    call(h.state_mut(), "ui_diff", json!({"mode": "after"})).unwrap();
    h.run_steps(2);
    assert!(place(h.state_mut())["selection"].is_null(), "removed items do not exist after");
    call(h.state_mut(), "ui_diff", json!({"mode": "unified"})).unwrap();
    call(h.state_mut(), "ui_select", json!({"element_id": "Task_Validate"})).unwrap();
    h.run_steps(2);
    call(h.state_mut(), "ui_diff", json!({"mode": "side-by-side"})).unwrap();
    h.run_steps(3);
    let p = place(h.state_mut());
    assert_eq!(p["mode"], "side-by-side");
    assert_eq!(p["selection"]["id"], "Task_Validate");
    assert_eq!(p["selection"]["status"], "changed");

    // Hiding a status removes its marks.
    call(h.state_mut(), "ui_diff", json!({"mode": "unified", "show": {"moved": false}})).unwrap();
    h.run_steps(2);
    let p = place(h.state_mut());
    assert_eq!(p["marked"].as_u64().unwrap(), total - c["moved"].as_u64().unwrap());
}

#[test]
fn xml_diff_folds_unchanged_runs_and_unfolds_the_focused_element() {
    let mut h = harness(diff_nav());
    h.run_steps(2);
    call(h.state_mut(), "ui_view_xml", json!({"element_id": "Task_Validate"})).unwrap();
    h.run_steps(4);
    let p = place(h.state_mut());
    assert_eq!(p["kind"], "xml");
    assert_eq!(p["diff"], true);
    assert!(p["added"].as_u64().unwrap() > 0 && p["deleted"].as_u64().unwrap() > 0, "{p}");
    assert!(p["folds"].as_u64().unwrap() > 0, "unchanged runs are folded");
    assert!(p["displayed_rows"].as_u64().unwrap() < p["lines"].as_u64().unwrap());
    let (line, _) = position("order-v2.bpmn", "<bpmn:serviceTask id=\"Task_Validate\"");
    assert_eq!(p["highlight"]["line"].as_u64().unwrap(), line);
    // A removed element is found on the old side.
    call(h.state_mut(), "ui_tabs", json!({"action": "activate", "index": 0})).unwrap();
    h.run_steps(2);
    call(h.state_mut(), "ui_view_xml", json!({"element_id": "Task_Notify"})).unwrap();
    h.run_steps(4);
    let p = place(h.state_mut());
    assert_eq!(p["highlight"]["side"], "old");
    // The whole file shows every row.
    let r = call(h.state_mut(), "ui_xml", json!({"whole_file": true})).unwrap();
    h.run_steps(2);
    let p = place(h.state_mut());
    assert_eq!(p["displayed_rows"], p["lines"], "{r}");
}

#[test]
fn diff_bpmn_tool_reports_property_changes() {
    let mut h = harness(Nav::Definitions);
    let r = call(h.state_mut(), "diff_bpmn", json!({"left": fixture("order.bpmn").to_str().unwrap(), "right": fixture("order-v2.bpmn").to_str().unwrap()})).unwrap();
    let items = r["items"].as_array().unwrap();
    let flow = items.iter().find(|i| i["id"] == "Flow_Large").expect("changed condition reported");
    assert_eq!(flow["status"], "changed");
    let cond = flow["changes"].as_array().unwrap().iter().find(|c| c["key"] == "condition").unwrap();
    assert!(cond["old"].as_str().unwrap().contains("1000") && cond["new"].as_str().unwrap().contains("5000"));
    assert!(items.iter().any(|i| i["id"] == "Task_Fraud" && i["status"] == "added"));
    assert!(items.iter().any(|i| i["id"] == "Task_Notify" && i["status"] == "removed"));
    assert!(call(h.state_mut(), "diff_bpmn", json!({"left": "/no/such.bpmn", "right": "/no/other.bpmn"})).unwrap_err().contains("no such file"));
}

#[test]
fn background_fit_diagram_menu_item_does_not_crash() {
    // It used to send a select of element usize::MAX, which the file and definition views indexed.
    let mut h = harness(Nav::File { path: fixture("order.bpmn") });
    let rect = h.state().last_diagram_viewport;
    // Right-click on empty canvas (top-left corner of the view) and choose "Fit diagram".
    let p = rect.min + egui::vec2(8.0, 8.0);
    h.input_mut().events.push(egui::Event::PointerMoved(p));
    h.run_steps(1);
    for pressed in [true, false] {
        h.input_mut().events.push(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Secondary, pressed, modifiers: Default::default() });
    }
    h.run_steps(3);
    h.get_by_label_contains("Fit diagram").click();
    h.run_steps(3);
    assert_eq!(place(h.state_mut())["kind"], "file");
}

#[test]
fn documentation_is_rendered_after_every_other_inspector_field() {
    let mut h = harness(Nav::File { path: fixture("order.bpmn") });
    call(h.state_mut(), "ui_select", json!({"element_id": "Task_Validate"})).unwrap();
    h.run_steps(4);
    let doc = h.get_by_label("Checks the order payload.").rect();
    for other in ["Identity", "Implementation", "Input / output mapping", "XML"] {
        let r = h.get_by_label_contains(other).rect();
        assert!(r.max.y <= doc.min.y, "{other} at {r:?} must come before the documentation at {doc:?}");
    }
}
