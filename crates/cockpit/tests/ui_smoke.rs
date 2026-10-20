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

/// Alt doubles the wheel zoom and shows the orbiting badge. The rendered frame is written to
/// `FLIGHTDECK_TEST_OUT` when set, for eyeballing.
#[test]
fn alt_doubles_wheel_zoom_and_renders_badge() {
    use egui::{Event, Modifiers, MouseWheelUnit, Pos2, Vec2};
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/../bpmn/tests/fixtures/order.bpmn");
    let zoom_after = |alt: bool| -> (f32, Option<image::RgbaImage>) {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx, Settings::default(), 1.0);
        app.offline = true;
        app.navigate(cockpit::state::Nav::File { path: fixture.into() }, cockpit::state::NavMode::NewTab);
        let mut builder = Harness::builder().with_size(egui::Vec2::new(1200.0, 800.0)).allow_missing_glyphs();
        if alt {
            builder = builder.wgpu();
        }
        let mut harness = builder.build_ui_state(|ui, app: &mut App| app.frame(ui), app);
        harness.run_steps(4);
        let canvas = Pos2::new(400.0, 400.0);
        harness.hover_at(canvas);
        harness.step();
        let mods = if alt { Modifiers::ALT } else { Modifiers::NONE };
        harness.input_mut().events.push(Event::ModifiersChanged(mods));
        harness.input_mut().events.push(Event::MouseWheel { unit: MouseWheelUnit::Point, delta: Vec2::new(0.0, 120.0), phase: egui::TouchPhase::Move, source: egui::MouseWheelSource::Wheel, modifiers: mods });
        harness.step();
        // let the camera settle while Alt stays held
        for _ in 0..40 {
            harness.step();
        }
        let img = if alt { harness.render().ok() } else { None };
        let z = match harness.state().tabs[0].place() {
            cockpit::state::Place::File(p) => p.view.camera.target.zoom,
            _ => panic!("file place expected"),
        };
        (z, img)
    };
    let (z_plain, _) = zoom_after(false);
    let (z_alt, img) = zoom_after(true);
    // both start from the same fitted zoom; compare the applied factors in log space
    let base = {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx, Settings::default(), 1.0);
        app.offline = true;
        app.navigate(cockpit::state::Nav::File { path: fixture.into() }, cockpit::state::NavMode::NewTab);
        let mut h = Harness::builder().with_size(egui::Vec2::new(1200.0, 800.0)).allow_missing_glyphs().build_ui_state(|ui, app: &mut App| app.frame(ui), app);
        h.run_steps(4);
        match h.state().tabs[0].place() {
            cockpit::state::Place::File(p) => p.view.camera.target.zoom,
            _ => unreachable!(),
        }
    };
    let f_plain = (z_plain / base).ln();
    let f_alt = (z_alt / base).ln();
    assert!(f_plain > 0.05, "a wheel tick zooms in: {f_plain}");
    assert!((f_alt / f_plain - 2.0).abs() < 0.05, "Alt doubles the zoom rate: {f_alt} vs {f_plain}");
    if let (Some(img), Ok(dir)) = (img, std::env::var("FLIGHTDECK_TEST_OUT")) {
        img.save(format!("{dir}/alt_badge.png")).unwrap();
    }
}

/// Modified shortcuts must not be swallowed by their plain variants: egui's `consume_key`
/// accepts extra Shift/Alt, so the plain check has to come second. Shift+Tab walks back,
/// Alt+`+` zooms by the Alt factor, and the keyboard zoom multiplier scales `+`.
#[test]
fn modified_shortcuts_are_not_shadowed_by_plain_ones() {
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/../bpmn/tests/fixtures/order.bpmn");
    let ctx = egui::Context::default();
    // zoom_speed 1 makes the keyboard step exactly 1.25; alt 0.5 is a precision-mode factor
    let settings = Settings { zoom_speed: 1.0, alt_speed: 0.5, ..Default::default() };
    let mut app = App::new(&ctx, settings, 1.0);
    app.offline = true;
    app.navigate(cockpit::state::Nav::File { path: fixture.into() }, cockpit::state::NavMode::NewTab);
    let mut harness = Harness::builder().with_size(egui::Vec2::new(1400.0, 860.0)).allow_missing_glyphs().build_ui_state(|ui, app: &mut App| app.frame(ui), app);
    harness.run_steps(5);
    let sel = |h: &mut Harness<'_, App>| call(h.state_mut(), "ui_state", json!({})).unwrap()["place"]["diagram"]["selection"]["element_id"].as_str().map(str::to_string);
    let zoom = |h: &mut Harness<'_, App>| call(h.state_mut(), "ui_state", json!({})).unwrap()["place"]["diagram"]["zoom"].as_f64().unwrap();

    call(harness.state_mut(), "ui_key", json!({"key": "Tab"})).unwrap();
    harness.run_steps(2);
    let first = sel(&mut harness).expect("Tab selects the first element in reading order");
    call(harness.state_mut(), "ui_key", json!({"key": "Tab"})).unwrap();
    harness.run_steps(2);
    let second = sel(&mut harness).unwrap();
    assert_ne!(first, second);
    call(harness.state_mut(), "ui_key", json!({"key": "Tab", "shift": true})).unwrap();
    harness.run_steps(2);
    assert_eq!(sel(&mut harness).as_deref(), Some(first.as_str()), "Shift+Tab must walk back, not forward");

    // settle the camera between key presses so ratios are exact
    let z0 = {
        call(harness.state_mut(), "ui_zoom", json!({"level": 1.0})).unwrap();
        harness.run_steps(2);
        zoom(&mut harness)
    };
    call(harness.state_mut(), "ui_key", json!({"key": "Plus"})).unwrap();
    harness.run_steps(2);
    let z1 = zoom(&mut harness);
    assert!((z1 / z0 - 1.25).abs() < 1e-3, "plain + steps by 1.25 at keyboard multiplier 1, got {}", z1 / z0);
    call(harness.state_mut(), "ui_key", json!({"key": "Plus", "alt": true})).unwrap();
    harness.run_steps(2);
    let z2 = zoom(&mut harness);
    let expect = 1.25f64.powf(0.5);
    assert!((z2 / z1 - expect).abs() < 1e-3, "Alt+`+` must use the Alt factor (0.5 → {expect:.4}), got {}", z2 / z1);
}
