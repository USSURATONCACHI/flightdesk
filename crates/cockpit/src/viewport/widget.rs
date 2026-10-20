//! The interactive diagram widget: camera input (mouse, wheel, keys, pinch), selection,
//! hover tooltips, the find bar, the minimap and the zoom indicator.

use super::camera::Camera;
use super::hit::hit_test;
use super::render::{BadgeHit, BadgeKind, DiagramStyle, Overlays, Renderer};
use bpmn::{ElementKind, Model, Ref};
use egui::{Align2, CornerRadius, CursorIcon, FontId, Id, Key, Modifiers, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub struct DiagramSettings {
    /// true: wheel zooms (Ctrl+wheel pans); false: wheel pans (Ctrl+wheel zooms).
    pub wheel_zooms: bool,
    pub inertia: bool,
    pub pan_speed: f32,
    pub zoom_step: f32,
    pub hover_delay: f64,
}

impl Default for DiagramSettings {
    fn default() -> Self {
        Self { wheel_zooms: true, inertia: true, pan_speed: 600.0, zoom_step: 1.25, hover_delay: 0.25 }
    }
}

#[derive(Clone, Debug, Default)]
pub struct FindState {
    pub open: bool,
    pub query: String,
    pub case_sensitive: bool,
    pub include_flows: bool,
    pub categories: Vec<String>,
    pub matches: Vec<bpmn::search::Match>,
    pub cursor: usize,
    pub want_focus: bool,
    last_key: Option<(String, bool, bool, Vec<String>)>,
}

impl FindState {
    pub fn current(&self) -> Option<Ref> {
        self.matches.get(self.cursor).map(|m| m.target)
    }
    pub fn active(&self) -> bool {
        self.open && !self.query.trim().is_empty()
    }
    fn recompute(&mut self, model: &Model) -> bool {
        let key = (self.query.clone(), self.case_sensitive, self.include_flows, self.categories.clone());
        if self.last_key.as_ref() == Some(&key) {
            return false;
        }
        self.last_key = Some(key);
        self.matches = if self.query.trim().is_empty() {
            vec![]
        } else {
            bpmn::search::search(model, &self.query, &bpmn::search::SearchOptions { case_sensitive: self.case_sensitive, categories: self.categories.clone(), include_flows: self.include_flows })
        };
        self.cursor = 0;
        true
    }
}

/// Modes where the next click on an element means something special (token modification).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickMode {
    /// Pick a target activity; carries a human label for the status hint.
    Target { label: String },
}

#[derive(Debug)]
pub struct DiagramView {
    pub camera: Camera,
    pub selection: Option<Ref>,
    pub multi: HashSet<usize>,
    pub hover: Option<Ref>,
    hover_since: f64,
    hover_pos: Pos2,
    pub find: FindState,
    pub show_minimap: bool,
    pub show_ids: bool,
    pub show_grid: bool,
    pub pick: Option<PickMode>,
    needs_fit: bool,
    pub fit_request: Option<Rect>,
    key_hold_since: Option<f64>,
    drag_velocity: Vec2,
    last_drag_time: f64,
    context_target: Option<Ref>,
    walk_candidates: Vec<(usize, usize)>,
    pub focused: bool,
    want_focus: bool,
    badges: Vec<BadgeHit>,
    /// Zoom-independent label layout cache.
    pub text: super::text::TextCache,
    /// Set by Shift+F; the owning place fits the camera to its tokens and clears it.
    pub fit_tokens_marker: bool,
    id: Id,
}

impl DiagramView {
    pub fn new(id: Id) -> Self {
        Self {
            camera: Camera::default(),
            selection: None,
            multi: HashSet::new(),
            hover: None,
            hover_since: 0.0,
            hover_pos: Pos2::ZERO,
            find: FindState::default(),
            show_minimap: true,
            show_ids: false,
            show_grid: false,
            pick: None,
            needs_fit: true,
            fit_request: None,
            key_hold_since: None,
            drag_velocity: Vec2::ZERO,
            last_drag_time: 0.0,
            context_target: None,
            walk_candidates: vec![],
            focused: false,
            want_focus: true,
            badges: vec![],
            text: Default::default(),
            fit_tokens_marker: false,
            id,
        }
    }

    pub fn request_focus(&mut self) {
        self.want_focus = true;
    }
    pub fn fit_all(&mut self) {
        self.needs_fit = true;
    }
    pub fn fit_rect(&mut self, world: bpmn::Rect) {
        self.fit_request = Some(Rect::from_min_size(Pos2::new(world.x, world.y), Vec2::new(world.w, world.h)));
    }
    pub fn fit_element(&mut self, m: &Model, idx: usize) {
        let b = m.elements[idx].bounds;
        if b.is_valid() {
            self.fit_rect(b.expand(b.w.max(b.h) * 0.9 + 40.0));
        }
    }
    pub fn reveal(&mut self, m: &Model, r: Ref, viewport: Rect) {
        let b = match r {
            Ref::Element(i) => m.elements[i].bounds,
            Ref::Flow(i) => m.flows[i].bounds,
        };
        if !b.is_valid() {
            return;
        }
        let world = Rect::from_min_size(Pos2::new(b.x, b.y), Vec2::new(b.w, b.h));
        if !self.camera.contains_world(viewport, world.expand(40.0)) || self.camera.target.zoom < 0.4 {
            self.camera.target.center = world.center();
            if self.camera.target.zoom < 0.6 {
                self.camera.target.zoom = 0.9;
            }
        }
    }
    pub fn select(&mut self, r: Option<Ref>) {
        self.selection = r;
        self.multi.clear();
        self.walk_candidates.clear();
    }
    pub fn open_find(&mut self) {
        self.find.open = true;
        self.find.want_focus = true;
    }
    pub fn open_find_with(&mut self, q: &str) {
        self.find.open = true;
        self.find.query = q.to_string();
        self.find.want_focus = true;
    }
    pub fn close_find(&mut self) {
        self.find.open = false;
        self.find.query.clear();
        self.find.matches.clear();
        self.find.last_key = None;
    }
    pub fn fit_refs(&mut self, m: &Model, refs: impl Iterator<Item = Ref>) {
        let mut b = bpmn::Rect::EMPTY;
        for r in refs {
            let rb = match r {
                Ref::Element(i) => m.elements[i].bounds,
                Ref::Flow(i) => m.flows[i].bounds,
            };
            if rb.is_valid() {
                b = b.union(&rb);
            }
        }
        if b.is_valid() && b.w >= 0.0 {
            self.fit_rect(b.expand(60.0));
        }
    }
}

#[derive(Debug)]
pub struct DiagramOutput {
    pub clicked: Option<Ref>,
    pub double_clicked: Option<Ref>,
    pub selection_changed: bool,
    pub badge_clicked: Option<(BadgeKind, String)>,
    /// An element was picked while in `PickMode`.
    pub picked: Option<usize>,
    pub enter_pressed: Option<Ref>,
    pub copied: bool,
    pub viewport: Rect,
    pub find_changed: bool,
}

impl Default for DiagramOutput {
    fn default() -> Self {
        Self { clicked: None, double_clicked: None, selection_changed: false, badge_clicked: None, picked: None, enter_pressed: None, copied: false, viewport: Rect::NOTHING, find_changed: false }
    }
}

/// Extra per-element text for the hover tooltip, supplied by the place.
pub type TooltipExtra<'a> = &'a dyn Fn(&mut Ui, &str);

/// A diagram to draw: the model plus the colours and input settings to draw it with.
pub struct Diagram<'a> {
    pub model: &'a Model,
    pub style: &'a DiagramStyle,
    pub settings: &'a DiagramSettings,
}

/// Per-frame timing passed to the key handler.
struct FrameTime {
    dt: f32,
    now: f64,
}

impl Diagram<'_> {
    /// Draw the diagram into the remaining space of `ui`, handling all input.
    pub fn show(&self, ui: &mut Ui, view: &mut DiagramView, overlays: &mut Overlays, tooltip_extra: TooltipExtra<'_>, context_menu: &mut dyn FnMut(&mut Ui, Option<Ref>)) -> DiagramOutput {
        show_impl(ui, view, self.model, overlays, self.style, self.settings, tooltip_extra, context_menu)
    }
}

#[allow(clippy::too_many_arguments)]
fn show_impl(
    ui: &mut Ui,
    view: &mut DiagramView,
    model: &Model,
    overlays: &mut Overlays,
    style: &DiagramStyle,
    settings: &DiagramSettings,
    tooltip_extra: TooltipExtra<'_>,
    context_menu: &mut dyn FnMut(&mut Ui, Option<Ref>),
) -> DiagramOutput {
    let mut out = DiagramOutput::default();
    let rect = ui.available_rect_before_wrap();
    out.viewport = rect;
    let response = ui.interact(rect, view.id, Sense::click_and_drag());
    let ctx = ui.ctx().clone();
    let now = ctx.input(|i| i.time);
    let dt = ctx.input(|i| i.stable_dt).min(0.05);

    if view.want_focus {
        response.request_focus();
        view.want_focus = false;
    }
    if response.clicked() || response.drag_started() || response.secondary_clicked() {
        response.request_focus();
    }
    view.focused = response.has_focus();

    // Fit requests.
    if view.needs_fit && rect.width() > 10.0 {
        let b = model.bounds;
        if b.is_valid() && b.w > 0.0 {
            view.camera.fit(rect, Rect::from_min_size(Pos2::new(b.x, b.y), Vec2::new(b.w, b.h)), 24.0, 1.5);
            view.camera.snap();
        }
        view.needs_fit = false;
    }
    if let Some(r) = view.fit_request.take() {
        view.camera.fit(rect, r, 24.0, 3.0);
    }

    let text_focus = text_input_focused(&ctx, view.id);
    let hovered = response.hovered() || response.dragged();
    let pointer = ctx.input(|i| i.pointer.hover_pos());
    let keys_allowed = (view.focused || hovered) && !text_focus && !view.find_bar_has_focus(&ctx);

    // ---- wheel / pinch ----
    if hovered {
        let (scroll, zoom_delta, mods) = ctx.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.modifiers));
        let anchor = pointer.unwrap_or(rect.center());
        if zoom_delta != 1.0 {
            view.camera.zoom_by(rect, zoom_delta, anchor);
        }
        if scroll != Vec2::ZERO {
            let wheel_zoom = settings.wheel_zooms != mods.ctrl;
            if mods.shift {
                // horizontal pan (egui already maps shift+wheel to x)
                view.camera.pan_screen(Vec2::new(if scroll.x != 0.0 { scroll.x } else { scroll.y }, 0.0));
            } else if wheel_zoom && scroll.y != 0.0 {
                let factor = (scroll.y * 0.0018).exp();
                view.camera.zoom_by(rect, factor, anchor);
                if scroll.x != 0.0 {
                    view.camera.pan_screen(Vec2::new(scroll.x, 0.0));
                }
            } else {
                view.camera.pan_screen(scroll);
            }
        }
    }

    // ---- drag to pan ----
    let space_down = ctx.input(|i| i.key_down(Key::Space));
    let primary_drag = response.dragged_by(egui::PointerButton::Primary);
    let middle_drag = response.dragged_by(egui::PointerButton::Middle);
    if primary_drag || middle_drag {
        // Alt doubles the pan speed: the board moves twice as far as the cursor.
        let boost = if ctx.input(|i| i.modifiers.alt) { 2.0 } else { 1.0 };
        let d = response.drag_delta() * boost;
        view.camera.drag_screen(d);
        let dtt = (now - view.last_drag_time).max(1e-3) as f32;
        let v = -d / dtt;
        view.drag_velocity = if view.last_drag_time == 0.0 { v } else { view.drag_velocity * 0.6 + v * 0.4 };
        view.last_drag_time = now;
        ctx.set_cursor_icon(CursorIcon::Grabbing);
    } else if response.drag_stopped() {
        if settings.inertia && view.drag_velocity.length() > 150.0 && now - view.last_drag_time < 0.08 {
            view.camera.inertia = -view.drag_velocity.clamp(Vec2::splat(-6000.0), Vec2::splat(6000.0));
        }
        view.drag_velocity = Vec2::ZERO;
        view.last_drag_time = 0.0;
    } else if space_down && hovered {
        ctx.set_cursor_icon(CursorIcon::Grab);
    }

    // ---- hover ----
    let zoom = view.camera.zoom();
    let tol = 6.0 / zoom;
    let mut hover_hit: Option<Ref> = None;
    if hovered && !response.dragged() {
        if let Some(p) = pointer {
            if view.badges.iter().any(|b| b.rect.contains(p)) {
                hover_hit = None;
            } else {
                let w = view.camera.screen_to_world(rect, p);
                hover_hit = hit_test(model, w, tol);
            }
            if hover_hit != view.hover || (p - view.hover_pos).length() > 4.0 {
                view.hover_since = now;
                view.hover_pos = p;
            }
            view.hover = hover_hit;
        }
    } else if !response.dragged() {
        view.hover = None;
    }
    if hover_hit.is_some() && view.pick.is_some() {
        ctx.set_cursor_icon(CursorIcon::Crosshair);
    } else if hover_hit.is_some() {
        ctx.set_cursor_icon(CursorIcon::PointingHand);
    }

    // ---- clicks ----
    if response.clicked() {
        if let Some(p) = pointer {
            if let Some(b) = view.badges.iter().find(|b| b.rect.contains(p)) {
                out.badge_clicked = Some((b.kind, b.activity_id.clone()));
            } else {
                let w = view.camera.screen_to_world(rect, p);
                let hit = hit_test(model, w, tol);
                out.clicked = hit;
                if let (Some(_), Some(Ref::Element(i))) = (&view.pick, hit) {
                    out.picked = Some(i);
                } else {
                    let mods = ctx.input(|i| i.modifiers);
                    if mods.ctrl {
                        if let Some(Ref::Element(i)) = hit {
                            if let Some(Ref::Element(s)) = view.selection {
                                view.multi.insert(s);
                            }
                            if !view.multi.remove(&i) {
                                view.multi.insert(i);
                            }
                            view.selection = None;
                            out.selection_changed = true;
                        }
                    } else if hit != view.selection || !view.multi.is_empty() {
                        view.select(hit);
                        out.selection_changed = true;
                    }
                }
            }
        }
    }
    if response.double_clicked() {
        if let Some(p) = pointer {
            let w = view.camera.screen_to_world(rect, p);
            match hit_test(model, w, tol) {
                Some(Ref::Element(i)) => {
                    view.fit_element(model, i);
                    out.double_clicked = Some(Ref::Element(i));
                }
                Some(r) => out.double_clicked = Some(r),
                None => view.needs_fit = true,
            }
        }
    }
    if response.secondary_clicked() {
        if let Some(p) = pointer {
            let w = view.camera.screen_to_world(rect, p);
            view.context_target = hit_test(model, w, tol);
            if let Some(Ref::Element(_)) | Some(Ref::Flow(_)) = view.context_target {
                if view.context_target != view.selection && view.multi.is_empty() {
                    view.selection = view.context_target;
                    out.selection_changed = true;
                }
            }
        }
    }

    // ---- keyboard ----
    if keys_allowed {
        handle_keys(&ctx, view, model, settings, rect, &FrameTime { dt, now }, &mut out);
    } else {
        view.key_hold_since = None;
    }
    if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::F)) && (view.focused || hovered) {
        view.open_find();
    }

    // ---- find bar state ----
    if view.find.open {
        if view.find.recompute(model) {
            out.find_changed = true;
            if let Some(r) = view.find.current() {
                view.reveal(model, r, rect);
            }
        }
    } else if !view.find.matches.is_empty() {
        view.find.matches.clear();
        view.find.last_key = None;
    }

    // ---- animate ----
    if view.camera.tick(dt, settings.inertia) {
        ctx.request_repaint();
    }

    // ---- paint ----
    overlays.selection = view.selection;
    overlays.multi_selection = view.multi.clone();
    overlays.hover = view.hover;
    overlays.show_ids = view.show_ids;
    overlays.show_grid = view.show_grid;
    overlays.related.clear();
    if let Some(Ref::Element(i)) = view.hover.or(view.selection) {
        for &f in model.elements[i].outgoing.iter().chain(model.elements[i].incoming.iter()) {
            overlays.related.insert(Ref::Flow(f));
        }
    }
    if view.find.active() {
        overlays.matches = view.find.matches.iter().map(|m| m.target).collect();
        overlays.current_match = view.find.current();
        overlays.dim_unmatched = true;
    } else {
        overlays.matches.clear();
        overlays.current_match = None;
        overlays.dim_unmatched = false;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let mut renderer = Renderer::new(model, &view.camera, rect, &painter, style, overlays, &mut view.text);
    renderer.paint();
    view.badges = std::mem::take(&mut renderer.badges);

    // pick-mode banner
    if let Some(PickMode::Target { label }) = &view.pick {
        let txt = format!("{label} — click a target element, Esc to cancel");
        let galley = painter.layout_no_wrap(txt, FontId::proportional(13.0), style.badge_text);
        let r = Rect::from_center_size(Pos2::new(rect.center().x, rect.min.y + 26.0), galley.size() + Vec2::new(24.0, 12.0));
        painter.rect_filled(r, 6.0, style.selection);
        painter.galley(r.center() - galley.size() / 2.0, galley, style.badge_text);
    }

    // walk picker
    if !view.walk_candidates.is_empty() {
        for (n, (_, target)) in view.walk_candidates.iter().enumerate().take(9) {
            let b = model.elements[*target].bounds;
            let p = view.camera.world_to_screen(rect, Pos2::new(b.x + b.w / 2.0, b.y));
            let r = Rect::from_center_size(p - Vec2::new(0.0, 14.0), Vec2::splat(20.0));
            painter.rect_filled(r, 4.0, style.selection);
            painter.text(r.center(), Align2::CENTER_CENTER, (n + 1).to_string(), FontId::proportional(13.0), style.badge_text);
        }
    }

    // ---- overlays: find bar, minimap, zoom indicator, tooltip ----
    find_bar(ui, view, model, style, rect, &mut out);
    if view.show_minimap {
        minimap(ui, view, model, overlays, style, rect);
    }
    zoom_indicator(ui, view, model, rect, style);
    if let Some(h) = view.hover {
        if now - view.hover_since >= settings.hover_delay && !response.dragged() && !ctx.input(|i| i.pointer.any_down()) {
            tooltip(ui, view, model, overlays, h, tooltip_extra);
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
    }

    // context menu
    let target = view.context_target;
    response.context_menu(|ui| context_menu(ui, target));

    if view.focused {
        // subtle focus ring on the canvas edge
        painter.rect_stroke(rect.shrink(1.0), 0.0, Stroke::new(1.0, style.selection.gamma_multiply(0.5)), StrokeKind::Inside);
    }
    out
}

fn text_input_focused(ctx: &egui::Context, canvas_id: Id) -> bool {
    match ctx.memory(|m| m.focused()) {
        Some(id) if id != canvas_id => egui::text_edit::TextEditState::load(ctx, id).is_some(),
        _ => false,
    }
}

impl DiagramView {
    fn find_bar_has_focus(&self, ctx: &egui::Context) -> bool {
        self.find.open && ctx.memory(|m| m.has_focus(self.id.with("find")))
    }
}

fn handle_keys(ctx: &egui::Context, view: &mut DiagramView, model: &Model, settings: &DiagramSettings, rect: Rect, time: &FrameTime, out: &mut DiagramOutput) {
    let FrameTime { dt, now } = *time;
    let anchor = match view.selection {
        Some(Ref::Element(i)) => {
            let b = model.elements[i].bounds;
            let c = view.camera.world_to_screen(rect, Pos2::new(b.x + b.w / 2.0, b.y + b.h / 2.0));
            if rect.contains(c) {
                c
            } else {
                rect.center()
            }
        }
        _ => rect.center(),
    };
    // continuous pan
    let mut dir = Vec2::ZERO;
    let (shift, any_mod) = ctx.input(|i| (i.modifiers.shift, i.modifiers.ctrl || i.modifiers.alt || i.modifiers.command));
    if !any_mod {
        ctx.input(|i| {
            if i.key_down(Key::ArrowLeft) || i.key_down(Key::A) || i.key_down(Key::H) {
                dir.x += 1.0;
            }
            if i.key_down(Key::ArrowRight) || i.key_down(Key::D) || i.key_down(Key::L) {
                dir.x -= 1.0;
            }
            if i.key_down(Key::ArrowUp) || i.key_down(Key::W) || i.key_down(Key::K) {
                dir.y += 1.0;
            }
            if i.key_down(Key::ArrowDown) || i.key_down(Key::S) || i.key_down(Key::J) {
                dir.y -= 1.0;
            }
        });
    }
    if dir != Vec2::ZERO {
        let since = *view.key_hold_since.get_or_insert(now);
        let ramp = 1.0 + 1.5 * ((now - since) as f32 / 0.4).min(1.0);
        let speed = settings.pan_speed * ramp * if shift { 3.0 } else { 1.0 };
        view.camera.pan_screen(dir.normalized() * speed * dt);
        ctx.request_repaint();
    } else {
        view.key_hold_since = None;
    }
    ctx.input_mut(|i| {
        let step = settings.zoom_step;
        if i.consume_key(Modifiers::NONE, Key::Plus) || i.consume_key(Modifiers::NONE, Key::Equals) || i.consume_key(Modifiers::SHIFT, Key::Equals) || i.consume_key(Modifiers::SHIFT, Key::Plus) {
            view.camera.zoom_by(rect, step, anchor);
        }
        if i.consume_key(Modifiers::NONE, Key::Minus) {
            view.camera.zoom_by(rect, 1.0 / step, anchor);
        }
        if i.consume_key(Modifiers::NONE, Key::Num0) {
            view.needs_fit = true;
        }
        if i.consume_key(Modifiers::NONE, Key::Num1) {
            view.camera.set_zoom(rect, 1.0, anchor);
        }
        if i.consume_key(Modifiers::NONE, Key::M) {
            view.show_minimap = !view.show_minimap;
        }
        if i.consume_key(Modifiers::NONE, Key::G) {
            view.show_grid = !view.show_grid;
        }
        if i.consume_key(Modifiers::NONE, Key::I) {
            view.show_ids = !view.show_ids;
        }
    });
    let f = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F));
    let shift_f = ctx.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::F));
    if f {
        match view.selection.or(view.hover) {
            Some(Ref::Element(i)) => view.fit_element(model, i),
            Some(Ref::Flow(i)) => {
                let b = model.flows[i].bounds;
                view.fit_rect(b.expand(80.0));
            }
            None => view.needs_fit = true,
        }
    }
    if shift_f {
        // fit all active tokens / incidents — overlay sets are on the renderer side; approximate via selection of running ids
        let _ = shift_f;
        out.enter_pressed = None;
        view.fit_tokens_marker = true;
    }
    // Tab cycling in reading order
    let tab = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab));
    let shift_tab = ctx.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Tab));
    if (tab || shift_tab) && !model.reading_order.is_empty() {
        let ro = &model.reading_order;
        let cur = match view.selection {
            Some(Ref::Element(i)) => ro.iter().position(|&x| x == i),
            _ => None,
        };
        let next = match (cur, tab) {
            (Some(p), true) => ro[(p + 1) % ro.len()],
            (Some(p), false) => ro[(p + ro.len() - 1) % ro.len()],
            (None, true) => ro[0],
            (None, false) => ro[ro.len() - 1],
        };
        view.select(Some(Ref::Element(next)));
        view.reveal(model, Ref::Element(next), rect);
        out.selection_changed = true;
    }
    // Graph walk with Alt+arrows
    let alt_right = ctx.input_mut(|i| i.consume_key(Modifiers::ALT, Key::ArrowRight));
    let alt_left = ctx.input_mut(|i| i.consume_key(Modifiers::ALT, Key::ArrowLeft));
    if alt_right || alt_left {
        if let Some(Ref::Element(i)) = view.selection {
            let cands: Vec<(usize, usize)> = if alt_right { model.successors(i) } else { model.predecessors(i) };
            match cands.len() {
                0 => {}
                1 => {
                    view.select(Some(Ref::Element(cands[0].1)));
                    view.reveal(model, Ref::Element(cands[0].1), rect);
                    out.selection_changed = true;
                }
                _ => view.walk_candidates = cands,
            }
        }
    }
    if !view.walk_candidates.is_empty() {
        let keys = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9];
        for (n, k) in keys.iter().enumerate() {
            if n < view.walk_candidates.len() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, *k)) {
                let t = view.walk_candidates[n].1;
                view.select(Some(Ref::Element(t)));
                view.reveal(model, Ref::Element(t), rect);
                out.selection_changed = true;
                break;
            }
        }
    }
    if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
        out.enter_pressed = view.selection;
    }
    if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::C)) {
        let text = match view.selection {
            Some(Ref::Element(i)) => Some(model.elements[i].id.clone()),
            Some(Ref::Flow(i)) => Some(model.flows[i].id.clone()),
            None => None,
        };
        if let Some(t) = text {
            ctx.copy_text(t);
            out.copied = true;
        }
    }
    if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        if view.pick.is_some() {
            view.pick = None;
        } else if !view.walk_candidates.is_empty() {
            view.walk_candidates.clear();
        } else if view.find.open {
            view.close_find();
        } else if view.selection.is_some() || !view.multi.is_empty() {
            view.select(None);
            out.selection_changed = true;
        }
    }
}

fn find_bar(ui: &mut Ui, view: &mut DiagramView, model: &Model, style: &DiagramStyle, rect: Rect, out: &mut DiagramOutput) {
    if !view.find.open {
        return;
    }
    let bar = Rect::from_min_size(rect.min + Vec2::new(12.0, 10.0), Vec2::new(rect.width().min(560.0) - 24.0, 34.0));
    let id = view.id.with("find");
    let mut close = false;
    let mut go = 0i32;
    ui.scope_builder(egui::UiBuilder::new().max_rect(bar).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
        egui::Frame::new().fill(ui.visuals().window_fill).stroke(ui.visuals().window_stroke).corner_radius(CornerRadius::same(6)).inner_margin(egui::Margin::symmetric(8, 4)).show(ui, |ui| {
            ui.set_min_width(bar.width());
            ui.label(egui::RichText::new("🔍").strong());
            let edit = egui::TextEdit::singleline(&mut view.find.query).id(id).hint_text("Find element: name, id, type, expression…").desired_width(220.0).frame(egui::Frame::NONE);
            let resp = ui.add(edit);
            if view.find.want_focus {
                resp.request_focus();
                view.find.want_focus = false;
            }
            if resp.has_focus() {
                ui.input_mut(|i| {
                    if i.consume_key(Modifiers::NONE, Key::Escape) {
                        close = true;
                    }
                    if i.consume_key(Modifiers::SHIFT, Key::Enter) {
                        go = -1;
                    } else if i.consume_key(Modifiers::NONE, Key::Enter) {
                        go = 1;
                    }
                    if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                        go = 1;
                    }
                    if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                        go = -1;
                    }
                });
            }
            let n = view.find.matches.len();
            let count = if view.find.query.trim().is_empty() {
                String::new()
            } else if n == 0 {
                "no matches".to_string()
            } else {
                format!("{} / {}", view.find.cursor + 1, n)
            };
            ui.add_sized([80.0, 20.0], egui::Label::new(egui::RichText::new(count).color(if n == 0 && !view.find.query.trim().is_empty() { style.incident } else { ui.visuals().weak_text_color() })));
            if ui.small_button("▲").on_hover_text("Previous match (Shift+Enter)").clicked() {
                go = -1;
            }
            if ui.small_button("▼").on_hover_text("Next match (Enter)").clicked() {
                go = 1;
            }
            ui.separator();
            ui.toggle_value(&mut view.find.case_sensitive, "Aa").on_hover_text("Case sensitive");
            ui.toggle_value(&mut view.find.include_flows, "⟶").on_hover_text("Include sequence flows");
            for (cat, label) in [("task", "task"), ("gateway", "gw"), ("event", "ev"), ("call", "call"), ("subprocess", "sub")] {
                let mut on = view.find.categories.iter().any(|c| c == cat);
                if ui.toggle_value(&mut on, label).on_hover_text(format!("Only {label}s")).changed() {
                    if on {
                        view.find.categories.push(cat.to_string());
                    } else {
                        view.find.categories.retain(|c| c != cat);
                    }
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✕").on_hover_text("Close (Esc)").clicked() {
                    close = true;
                }
            });
        });
    });
    if go != 0 && !view.find.matches.is_empty() {
        let n = view.find.matches.len() as i32;
        view.find.cursor = ((view.find.cursor as i32 + go).rem_euclid(n)) as usize;
        if let Some(r) = view.find.current() {
            view.reveal(model, r, rect);
            view.select(Some(r));
            out.selection_changed = true;
        }
    }
    if close {
        view.close_find();
        view.request_focus();
    }
}

fn minimap(ui: &mut Ui, view: &mut DiagramView, model: &Model, overlays: &Overlays, style: &DiagramStyle, rect: Rect) {
    let b = model.bounds;
    if !b.is_valid() || b.w <= 0.0 || b.h <= 0.0 {
        return;
    }
    let world = Rect::from_min_size(Pos2::new(b.x, b.y), Vec2::new(b.w, b.h)).expand(20.0);
    // Hide when everything is visible.
    if view.camera.contains_world(rect, world) {
        return;
    }
    let size = Vec2::new(200.0, (200.0 * world.height() / world.width()).clamp(60.0, 160.0));
    let mm = Rect::from_min_size(rect.max - size - Vec2::new(12.0, 12.0), size);
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect(mm, 4.0, ui.visuals().window_fill.gamma_multiply(0.92), ui.visuals().window_stroke, StrokeKind::Inside);
    let scale = (size.x / world.width()).min(size.y / world.height());
    let origin = mm.min + (size - world.size() * scale) / 2.0;
    let to_mm = |p: Pos2| origin + (p - world.min) * scale;
    for &i in &model.draw_order {
        let e = &model.elements[i];
        if !e.kind.is_flow_node() && !matches!(e.kind, ElementKind::Participant | ElementKind::Lane) {
            continue;
        }
        let r = Rect::from_min_max(to_mm(Pos2::new(e.bounds.x, e.bounds.y)), to_mm(Pos2::new(e.bounds.x + e.bounds.w, e.bounds.y + e.bounds.h)));
        let mut c = style.stroke.gamma_multiply(0.35);
        if matches!(e.kind, ElementKind::Participant | ElementKind::Lane) {
            painter.rect_stroke(r, 0.0, Stroke::new(0.5, style.lane_stroke), StrokeKind::Inside);
            continue;
        }
        if overlays.running.contains_key(&e.id) || overlays.active.contains(&e.id) {
            c = style.running;
        }
        if overlays.incidents.contains_key(&e.id) {
            c = style.incident;
        }
        if overlays.completed.contains_key(&e.id) {
            c = blend(c, style.completed, 0.7);
        }
        painter.rect_filled(r.expand(0.3), 0.0, c);
    }
    let vis = view.camera.visible_world_rect(rect);
    let vr = Rect::from_min_max(to_mm(vis.min), to_mm(vis.max)).intersect(mm.expand(1.0));
    painter.rect(vr, 1.0, style.selection.gamma_multiply(0.12), Stroke::new(1.5, style.selection), StrokeKind::Inside);
    let resp = ui.interact(mm, view.id.with("minimap"), Sense::click_and_drag());
    if resp.dragged() || resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            let w = world.min + (p - origin) / scale;
            view.camera.target.center = w;
            if resp.clicked() {
                // nothing else: smooth animation does the rest
            } else {
                view.camera.current.center = w;
            }
        }
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
    }
}

fn zoom_indicator(ui: &mut Ui, view: &mut DiagramView, model: &Model, rect: Rect, style: &DiagramStyle) {
    let txt = format!("{:.0}%", view.camera.target.zoom * 100.0);
    let r = Rect::from_min_size(Pos2::new(rect.min.x + 12.0, rect.max.y - 34.0), Vec2::new(64.0, 22.0));
    ui.scope_builder(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
        let b = ui.add(egui::Button::new(egui::RichText::new(txt).monospace().size(11.0)).fill(ui.visuals().window_fill.gamma_multiply(0.9)).min_size(r.size()));
        let b = b.on_hover_text("Zoom. Click for presets. Wheel zooms, Shift+wheel pans, 0 fits, 1 = 100%");
        egui::Popup::menu(&b).show(|ui| {
            if ui.button("Fit diagram      0").clicked() {
                view.fit_all();
            }
            if ui.button("Fit selection    F").clicked() {
                if let Some(Ref::Element(i)) = view.selection {
                    view.fit_element(model, i);
                }
            }
            if ui.button("100%             1").clicked() {
                view.camera.set_zoom(rect, 1.0, rect.center());
            }
            for z in [0.25, 0.5, 2.0, 4.0] {
                if ui.button(format!("{:.0}%", z * 100.0)).clicked() {
                    view.camera.set_zoom(rect, z, rect.center());
                }
            }
            ui.separator();
            ui.checkbox(&mut view.show_minimap, "Minimap        M");
            ui.checkbox(&mut view.show_grid, "Grid           G");
            ui.checkbox(&mut view.show_ids, "Element ids    I");
        });
        let _ = style;
    });
}

fn tooltip(ui: &mut Ui, view: &DiagramView, model: &Model, overlays: &Overlays, h: Ref, extra: TooltipExtra<'_>) {
    let ctx = ui.ctx().clone();
    let pos = view.hover_pos + Vec2::new(14.0, 18.0);
    egui::Area::new(view.id.with("tooltip")).order(egui::Order::Tooltip).fixed_pos(pos).interactable(false).show(&ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_max_width(360.0);
            match h {
                Ref::Element(i) => {
                    let e = &model.elements[i];
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(e.display_name()).strong());
                        ui.label(egui::RichText::new(e.kind.label()).weak());
                    });
                    if e.name.is_some() {
                        ui.label(egui::RichText::new(&e.id).monospace().weak());
                    }
                    let mut facts: Vec<String> = vec![];
                    if let Some(n) = overlays.running.get(&e.id) {
                        facts.push(format!("{n} running"));
                    }
                    if let Some(n) = overlays.incidents.get(&e.id) {
                        facts.push(format!("{n} incident{}", if *n == 1 { "" } else { "s" }));
                    }
                    if let Some(n) = overlays.completed.get(&e.id) {
                        facts.push(format!("executed ×{n}"));
                    }
                    if let Some((t, _)) = overlays.timers.get(&e.id) {
                        facts.push(format!("timer {t}"));
                    }
                    if !facts.is_empty() {
                        ui.label(facts.join(" · "));
                    }
                    if let Some(imp) = implementation_summary(e) {
                        ui.label(egui::RichText::new(imp).monospace().size(11.0));
                    }
                    extra(ui, &e.id);
                }
                Ref::Flow(i) => {
                    let f = &model.flows[i];
                    ui.label(egui::RichText::new(f.display_name()).strong());
                    ui.label(egui::RichText::new(f.kind.label()).weak());
                    if let (Some(s), Some(t)) = (f.source, f.target) {
                        ui.label(format!("{} → {}", model.elements[s].display_name(), model.elements[t].display_name()));
                    }
                    if f.is_default {
                        ui.label("default flow");
                    }
                    if let Some(c) = &f.condition {
                        ui.label(egui::RichText::new(&c.text).monospace().size(11.0));
                    }
                }
            }
        });
    });
}

pub fn implementation_summary(e: &bpmn::Element) -> Option<String> {
    for k in ["delegateExpression", "class", "expression", "topic", "calledElement", "decisionRef", "formKey", "assignee", "candidateGroups", "candidateUsers"] {
        if let Some(v) = e.camunda(k).or_else(|| e.attr(k)) {
            return Some(format!("{k} = {v}"));
        }
    }
    for d in &e.event_defs {
        match d {
            bpmn::EventDef::Timer { time_date, time_duration, time_cycle } => {
                let (k, v) = if let Some(v) = time_duration {
                    ("duration", v)
                } else if let Some(v) = time_cycle {
                    ("cycle", v)
                } else if let Some(v) = time_date {
                    ("date", v)
                } else {
                    continue;
                };
                let human = bpmn::text::humanize_timer(v).map(|h| format!(" ({h})")).unwrap_or_default();
                return Some(format!("timer {k} = {v}{human}"));
            }
            bpmn::EventDef::Message { name, message_ref } => return Some(format!("message = {}", name.clone().or(message_ref.clone()).unwrap_or_default())),
            bpmn::EventDef::Signal { name, signal_ref } => return Some(format!("signal = {}", name.clone().or(signal_ref.clone()).unwrap_or_default())),
            bpmn::EventDef::Error { code, name, .. } => return Some(format!("error = {}", code.clone().or(name.clone()).unwrap_or_default())),
            bpmn::EventDef::Conditional { condition: Some(c), .. } => return Some(format!("condition = {}", c.text)),
            _ => {}
        }
    }
    if let Some(s) = &e.script {
        return Some(format!("script: {}", s.lines().next().unwrap_or("")));
    }
    None
}

use super::render::blend;
