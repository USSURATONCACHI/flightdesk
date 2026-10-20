//! Camera, hit testing and level of detail: the maths behind "smooth" and "clickable".

use bpmn::Ref;
use cockpit::viewport::camera::{Camera, CameraPose, MAX_ZOOM, MIN_ZOOM};
use cockpit::viewport::hit::hit_test;
use cockpit::viewport::render::Lod;
use egui::{Pos2, Rect, Vec2};

fn vp() -> Rect {
    Rect::from_min_size(Pos2::new(50.0, 80.0), Vec2::new(1200.0, 700.0))
}

fn model() -> bpmn::Model {
    bpmn::parse(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../bpmn/tests/fixtures/order.bpmn")).unwrap()).unwrap()
}

#[test]
fn zoom_to_cursor_is_exact_for_any_anchor_and_factor() {
    let mut c = Camera { target: CameraPose { center: Pos2::new(700.0, 320.0), zoom: 0.73 }, ..Default::default() };
    c.snap();
    for (ax, ay, f) in [(60.0, 90.0, 1.25), (1249.0, 779.0, 0.8), (650.0, 430.0, 3.0), (100.0, 700.0, 0.1), (1200.0, 100.0, 1.0015)] {
        let anchor = Pos2::new(ax, ay);
        let before = c.screen_to_world_target(vp(), anchor);
        c.zoom_by(vp(), f, anchor);
        let after = c.screen_to_world_target(vp(), anchor);
        assert!((before - after).length() < 1e-2, "anchor drifted by {:?} for factor {f}", before - after);
    }
}

#[test]
fn pan_and_drag_move_content_with_the_pointer() {
    let mut c = Camera { target: CameraPose { center: Pos2::new(0.0, 0.0), zoom: 2.0 }, ..Default::default() };
    c.snap();
    let world_under_cursor = c.screen_to_world(vp(), Pos2::new(300.0, 300.0));
    c.drag_screen(Vec2::new(100.0, -40.0));
    // the same world point is now 100 px right and 40 px up
    let moved = c.world_to_screen(vp(), world_under_cursor);
    assert!((moved - Pos2::new(400.0, 260.0)).length() < 1e-3);
    // drag is immediate (current == target), pan_screen only moves the target
    assert_eq!(c.current, c.target);
    c.pan_screen(Vec2::new(50.0, 0.0));
    assert_ne!(c.current, c.target);
    assert!(c.is_animating());
}

#[test]
fn smoothing_converges_independently_of_frame_rate() {
    let run = |dt: f32| {
        let mut c = Camera::default();
        c.snap();
        c.target = CameraPose { center: Pos2::new(2000.0, -500.0), zoom: 0.05 };
        let mut t = 0.0;
        while c.tick(dt, false) {
            t += dt;
            assert!(t < 5.0, "never converged at dt {dt}");
        }
        (t, c.current)
    };
    let (t60, p60) = run(1.0 / 60.0);
    let (t20, p20) = run(1.0 / 20.0);
    assert_eq!(p60, p20, "both end exactly on the target");
    assert!((t60 - t20).abs() < 0.3, "settling time is about the same at 60 and 20 fps ({t60} vs {t20})");
    assert!(t60 < 1.0, "settles within a second");
}

#[test]
fn inertia_decays_and_any_input_cancels_it() {
    let mut c = Camera::default();
    c.snap();
    c.inertia = Vec2::new(-3000.0, 0.0);
    let start = c.target.center;
    let mut frames = 0;
    while c.tick(1.0 / 60.0, true) {
        frames += 1;
        assert!(frames < 600);
    }
    assert!(c.target.center.x > start.x + 100.0, "the flick carried the view");
    assert_eq!(c.inertia, Vec2::ZERO);
    // inertia is dropped when the setting is off
    c.inertia = Vec2::new(-3000.0, 0.0);
    let before = c.target.center;
    c.tick(1.0 / 60.0, false);
    assert_eq!(c.target.center, before);
    assert_eq!(c.inertia, Vec2::ZERO);
    // and when the user zooms
    c.inertia = Vec2::new(-3000.0, 0.0);
    c.zoom_by(vp(), 1.1, vp().center());
    assert_eq!(c.inertia, Vec2::ZERO);
}

#[test]
fn fit_respects_padding_and_max_zoom_and_handles_degenerate_input() {
    let mut c = Camera::default();
    let world = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(50.0, 20.0));
    c.fit(vp(), world, 24.0, 1.5);
    c.snap();
    assert_eq!(c.target.zoom, 1.5, "small diagrams are not blown up past max_zoom");
    assert_eq!(c.target.center, world.center());
    let wide = Rect::from_min_size(Pos2::ZERO, Vec2::new(12_000.0, 600.0));
    c.fit(vp(), wide, 24.0, 10.0);
    c.snap();
    assert!(c.contains_world(vp(), wide));
    let s = c.world_rect_to_screen(vp(), wide);
    assert!(s.width() <= vp().width() - 2.0 * 24.0 + 0.01);
    // zero-size worlds and tiny viewports are ignored rather than producing NaN
    let before = c.target;
    c.fit(vp(), Rect::from_min_size(Pos2::ZERO, Vec2::ZERO), 24.0, 10.0);
    c.fit(Rect::from_min_size(Pos2::ZERO, Vec2::new(10.0, 10.0)), wide, 24.0, 10.0);
    assert_eq!(c.target, before);
    assert!(c.target.zoom.is_finite());
    assert!((MIN_ZOOM..=MAX_ZOOM).contains(&c.target.zoom));
}

#[test]
fn hit_testing_prefers_small_things_on_top_and_uses_true_shapes() {
    let m = model();
    let id = |r: Option<Ref>| match r {
        Some(Ref::Element(i)) => m.elements[i].id.clone(),
        Some(Ref::Flow(i)) => format!("flow:{}", m.flows[i].id),
        None => "none".into(),
    };
    // boundary timer sits on the approve task's border: the event wins over the task
    assert_eq!(id(hit_test(&m, Pos2::new(620.0, 160.0), 2.0)), "Timer_Reminder");
    // a task body hit
    assert_eq!(id(hit_test(&m, Pos2::new(330.0, 180.0), 2.0)), "Task_Validate");
    // events are circles: the bounding-box corner is empty space
    assert_eq!(id(hit_test(&m, Pos2::new(193.0, 163.0), 0.5)), "none");
    assert_eq!(id(hit_test(&m, Pos2::new(210.0, 180.0), 0.5)), "StartEvent_1");
    // gateways are diamonds: the corner of their box misses, the centre hits
    assert_eq!(id(hit_test(&m, Pos2::new(437.0, 157.0), 0.5)), "none");
    assert_eq!(id(hit_test(&m, Pos2::new(460.0, 180.0), 0.5)), "Gateway_Amount");
    // flows are hit within the tolerance only
    assert_eq!(id(hit_test(&m, Pos2::new(600.0, 181.0), 3.0)), "flow:Flow_Small");
    assert_eq!(id(hit_test(&m, Pos2::new(600.0, 190.0), 3.0)), "none");
    // lane/pool bodies are empty space (so dragging pans), their header strips are hittable
    assert_eq!(id(hit_test(&m, Pos2::new(900.0, 200.0), 2.0)), "none");
    assert_eq!(id(hit_test(&m, Pos2::new(160.0, 150.0), 2.0)), "Lane_Sales");
    assert_eq!(id(hit_test(&m, Pos2::new(130.0, 300.0), 2.0)), "Participant_Shop");
    // inside an expanded subprocess, children win over the container; its header strip selects the container
    assert_eq!(id(hit_test(&m, Pos2::new(690.0, 420.0), 2.0)), "Task_Script");
    assert_eq!(id(hit_test(&m, Pos2::new(600.0, 350.0), 2.0)), "Sub_Review");
}

#[test]
fn level_of_detail_thresholds() {
    assert_eq!(Lod::for_zoom(2.0), Lod::Full);
    assert_eq!(Lod::for_zoom(0.6), Lod::Full);
    assert_eq!(Lod::for_zoom(0.59), Lod::Medium);
    assert_eq!(Lod::for_zoom(0.3), Lod::Medium);
    assert_eq!(Lod::for_zoom(0.29), Lod::Low);
    assert_eq!(Lod::for_zoom(0.12), Lod::Low);
    assert_eq!(Lod::for_zoom(0.119), Lod::Blocks);
    assert!(Lod::Full < Lod::Blocks, "ordering lets the renderer write `lod <= Lod::Medium`");
}
