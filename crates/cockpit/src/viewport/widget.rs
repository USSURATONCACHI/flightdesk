//! The interactive diagram widget: camera input (mouse, wheel, keys, pinch), selection,
//! hover tooltips, the find bar, the minimap and the zoom indicator.

use super::camera::Camera;
use super::hit::hit_test;
use super::render::{BadgeHit, BadgeKind, DiagramStyle, Overlays, Renderer};
use bpmn::{ElementKind, Model, Ref};
use egui::{Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Key, Modifiers, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub struct DiagramSettings {
    /// true: wheel zooms (Ctrl+wheel pans); false: wheel pans (Ctrl+wheel zooms).
    pub wheel_zooms: bool,
    pub inertia: bool,
    /// Unlimited drags: teleport or lock the pointer at the view edge.
    pub wrap: super::wrap::WrapMode,
    pub pan_speed: f32,
    pub zoom_step: f32,
    pub hover_delay: f64,
}

impl Default for DiagramSettings {
    fn default() -> Self {
        Self { wheel_zooms: true, inertia: true, wrap: Default::default(), pan_speed: 600.0, zoom_step: 1.25, hover_delay: 0.25 }
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
    /// Running instances matching the query, supplied by the place (business key / id search).
    pub instance_hits: Vec<crate::history::InstanceHit>,
    pub instance_hits_loading: bool,
    /// Whether the place searches instances at all (settings); shown as a chip.
    pub instances_enabled: bool,
    pub cursor: usize,
    pub want_focus: bool,
    last_key: Option<(String, bool, bool, Vec<String>)>,
}

impl FindState {
    /// Everything Enter cycles through: element matches, then activities with matching instances.
    pub fn targets(&self, model: &Model) -> Vec<Ref> {
        let mut v: Vec<Ref> = self.matches.iter().map(|m| m.target).collect();
        let mut seen = std::collections::HashSet::new();
        for h in &self.instance_hits {
            if seen.insert(h.activity_id.as_str()) {
                if let Some(i) = model.element_idx(&h.activity_id) {
                    let r = Ref::Element(i);
                    if !v.contains(&r) {
                        v.push(r);
                    }
                }
            }
        }
        v
    }
    pub fn current(&self) -> Option<Ref> {
        self.matches.get(self.cursor).map(|m| m.target)
    }
    pub fn current_in(&self, model: &Model) -> Option<Ref> {
        self.targets(model).get(self.cursor).copied()
    }
    pub fn active(&self) -> bool {
        self.open && !self.query.trim().is_empty()
    }
    /// Activities with at least one matching instance.
    pub fn instance_match_refs(&self, model: &Model) -> std::collections::HashSet<Ref> {
        self.instance_hits.iter().filter_map(|h| model.element_idx(&h.activity_id).map(Ref::Element)).collect()
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
    /// Pointer wrapping for canvas pans and for the zoom dial.
    pan_wrap: super::wrap::PointerWrap,
    dial_wrap: super::wrap::PointerWrap,
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
            pan_wrap: Default::default(),
            dial_wrap: Default::default(),
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
        self.find.instance_hits.clear();
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
    // Alt doubles every movement: pan, wheel/pinch zoom, key zoom and the dial.
    let boost = if ctx.input(|i| i.modifiers.alt) { 2.0 } else { 1.0 };
    if hovered {
        let (scroll, zoom_delta, mods) = ctx.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.modifiers));
        let anchor = pointer.unwrap_or(rect.center());
        if zoom_delta != 1.0 {
            view.camera.zoom_by(rect, zoom_delta.powf(boost), anchor);
        }
        if scroll != Vec2::ZERO {
            let wheel_zoom = settings.wheel_zooms != mods.ctrl;
            if mods.shift {
                // horizontal pan (egui already maps shift+wheel to x)
                view.camera.pan_screen(Vec2::new(if scroll.x != 0.0 { scroll.x } else { scroll.y }, 0.0) * boost);
            } else if wheel_zoom && scroll.y != 0.0 {
                let factor = (scroll.y * 0.0018 * boost).exp();
                view.camera.zoom_by(rect, factor, anchor);
                if scroll.x != 0.0 {
                    view.camera.pan_screen(Vec2::new(scroll.x, 0.0) * boost);
                }
            } else {
                view.camera.pan_screen(scroll * boost);
            }
        }
    }

    // ---- drag to pan ----
    let space_down = ctx.input(|i| i.key_down(Key::Space));
    let primary_drag = response.dragged_by(egui::PointerButton::Primary);
    let middle_drag = response.dragged_by(egui::PointerButton::Middle);
    if response.drag_started() {
        view.pan_wrap.begin(settings.wrap);
    }
    if primary_drag || middle_drag {
        let d = view.pan_wrap.delta(&ctx, response.drag_delta(), pointer, rect, (true, true)) * boost;
        view.camera.drag_screen(d);
        let dtt = (now - view.last_drag_time).max(1e-3) as f32;
        let v = -d / dtt;
        view.drag_velocity = if view.last_drag_time == 0.0 { v } else { view.drag_velocity * 0.6 + v * 0.4 };
        view.last_drag_time = now;
        ctx.set_cursor_icon(CursorIcon::Grabbing);
    } else if response.drag_stopped() {
        view.pan_wrap.end();
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
            if let Some(r) = view.find.current_in(model) {
                view.reveal(model, r, rect);
            }
        }
    } else if !view.find.matches.is_empty() || !view.find.instance_hits.is_empty() {
        view.find.matches.clear();
        view.find.instance_hits.clear();
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
        overlays.instance_matches = view.find.instance_match_refs(model);
        overlays.current_match = view.find.current_in(model);
        overlays.dim_unmatched = true;
    } else {
        overlays.matches.clear();
        overlays.instance_matches.clear();
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
    find_legend(ui, view, model, style, rect);
    if view.show_minimap {
        minimap(ui, view, model, overlays, style, rect);
    }
    zoom_indicator(ui, view, model, rect, style, settings);
    if let Some(h) = view.hover {
        if now - view.hover_since >= settings.hover_delay && !response.dragged() && !ctx.input(|i| i.pointer.any_down()) {
            tooltip(ui, view, model, overlays, style, h, tooltip_extra);
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
    }

    alt_badge(&ctx, view, style, rect, hovered || response.dragged(), pointer);

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
        if i.consume_key(Modifiers::ALT, Key::Plus) || i.consume_key(Modifiers::ALT, Key::Equals) || i.consume_key(Modifiers::ALT.plus(Modifiers::SHIFT), Key::Equals) {
            view.camera.zoom_by(rect, step * step, anchor);
        }
        if i.consume_key(Modifiers::NONE, Key::Minus) {
            view.camera.zoom_by(rect, 1.0 / step, anchor);
        }
        if i.consume_key(Modifiers::ALT, Key::Minus) {
            view.camera.zoom_by(rect, 1.0 / (step * step), anchor);
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
    let width = (rect.width() - 24.0).clamp(260.0, 640.0);
    let bar = Rect::from_min_size(rect.min + Vec2::new(12.0, 10.0), Vec2::new(width, 64.0));
    let id = view.id.with("find");
    let mut close = false;
    let mut go = 0i32;
    let targets = view.find.targets(model);
    let n_total = targets.len();
    ui.scope_builder(egui::UiBuilder::new().max_rect(bar).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
        egui::Frame::new().fill(ui.visuals().window_fill).stroke(ui.visuals().window_stroke).corner_radius(CornerRadius::same(6)).inner_margin(egui::Margin::symmetric(8, 5)).show(ui, |ui| {
            ui.set_width(width - 16.0);
            // row 1: input · counter · prev/next · close
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔍").strong());
                let edit_w = (ui.available_width() - 190.0).max(80.0);
                let edit = egui::TextEdit::singleline(&mut view.find.query).id(id).hint_text("Find element, business key or instance id…").desired_width(edit_w).frame(egui::Frame::NONE);
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
                let count = if view.find.query.trim().is_empty() {
                    String::new()
                } else if n_total == 0 && !view.find.instance_hits_loading {
                    "no matches".to_string()
                } else {
                    format!("{} / {}", (view.find.cursor + 1).min(n_total.max(1)), n_total)
                };
                let count_color = if n_total == 0 && !view.find.query.trim().is_empty() && !view.find.instance_hits_loading { style.incident } else { ui.visuals().weak_text_color() };
                ui.add_sized([70.0, 20.0], egui::Label::new(egui::RichText::new(count).color(count_color)));
                if ui.small_button("▲").on_hover_text("Previous match (Shift+Enter)").clicked() {
                    go = -1;
                }
                if ui.small_button("▼").on_hover_text("Next match (Enter)").clicked() {
                    go = 1;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("✕").on_hover_text("Close (Esc)").clicked() {
                        close = true;
                    }
                });
            });
            // row 2: options and what matched
            ui.horizontal_wrapped(|ui| {
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
                if view.find.instances_enabled && !view.find.query.trim().is_empty() {
                    ui.separator();
                    let n_el = view.find.matches.len();
                    let n_inst = view.find.instance_hits.len();
                    let n_act = n_total.saturating_sub(n_el);
                    let mut txt = format!("{n_el} element{}", if n_el == 1 { "" } else { "s" });
                    if view.find.instance_hits_loading {
                        txt.push_str(" · searching instances…");
                    } else if n_inst > 0 {
                        txt.push_str(&format!(" · {n_inst} instance{} at {n_act} activit{}", if n_inst == 1 { "" } else { "s" }, if n_act == 1 { "y" } else { "ies" }));
                    }
                    ui.label(egui::RichText::new(txt).weak().small());
                    if n_inst > 0 {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 4.0, style.external);
                        ui.label(egui::RichText::new("instance match").weak().small());
                    }
                }
            });
        });
    });
    if go != 0 && n_total > 0 {
        view.find.cursor = ((view.find.cursor as i32 + go).rem_euclid(n_total as i32)) as usize;
        if let Some(r) = targets.get(view.find.cursor).copied() {
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

fn zoom_indicator(ui: &mut Ui, view: &mut DiagramView, model: &Model, rect: Rect, style: &DiagramStyle, settings: &DiagramSettings) {
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
    });
    zoom_dial(ui, view, Rect::from_min_size(Pos2::new(r.max.x + 6.0, r.min.y), Vec2::new(150.0, 22.0)), rect, style, settings);
}

/// A scrubbable dial: drag left/right to zoom continuously, without limit (the pointer wraps at
/// the dial's edges). One octave of zoom is `PX_PER_OCTAVE` pixels of travel.
fn zoom_dial(ui: &mut Ui, view: &mut DiagramView, dial: Rect, viewport: Rect, style: &DiagramStyle, settings: &DiagramSettings) {
    const PX_PER_OCTAVE: f32 = 96.0;
    let ctx = ui.ctx().clone();
    let resp = ui.interact(dial, view.id.with("zoom-dial"), Sense::click_and_drag());
    let anchor = viewport.center();
    if resp.drag_started() {
        view.dial_wrap.begin(settings.wrap);
    }
    if resp.dragged() {
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let d = view.dial_wrap.delta(&ctx, resp.drag_delta(), pointer, dial.expand2(Vec2::new(0.0, 40.0)), (true, false));
        if d.x != 0.0 {
            let boost = if ctx.input(|i| i.modifiers.alt) { 2.0 } else { 1.0 };
            view.camera.zoom_by(viewport, (d.x * boost / PX_PER_OCTAVE).exp2(), anchor);
        }
        ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
    } else if resp.drag_stopped() {
        view.dial_wrap.end();
    }
    if resp.hovered() {
        ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
        let scroll = ctx.input(|i| i.smooth_scroll_delta);
        if scroll != Vec2::ZERO {
            view.camera.zoom_by(viewport, ((scroll.x + scroll.y) / PX_PER_OCTAVE).exp2(), anchor);
        }
    }
    resp.on_hover_text("Drag left or right to zoom, as far as you like");
    // paint: a ruler that scrolls with log2(zoom)
    let painter = ui.painter().with_clip_rect(dial);
    let fill = ui.visuals().window_fill.gamma_multiply(0.9);
    painter.rect(dial, 6.0, fill, ui.visuals().window_stroke, StrokeKind::Inside);
    let tick_spacing = PX_PER_OCTAVE / 8.0;
    let offset = view.camera.target.zoom.log2() * PX_PER_OCTAVE;
    let cx = dial.center().x;
    let first = ((dial.min.x - cx + offset) / tick_spacing).floor() as i32;
    let last = ((dial.max.x - cx + offset) / tick_spacing).ceil() as i32;
    let tick_color = style.text_muted.gamma_multiply(0.7);
    for n in first..=last {
        let x = cx + n as f32 * tick_spacing - offset;
        if x < dial.min.x + 3.0 || x > dial.max.x - 3.0 {
            continue;
        }
        let octave = n.rem_euclid(8) == 0;
        let h = if octave { 10.0 } else if n.rem_euclid(4) == 0 { 7.0 } else { 4.0 };
        let fade = 1.0 - ((x - cx).abs() / (dial.width() / 2.0)).powi(2) * 0.8;
        painter.line_segment([Pos2::new(x, dial.center().y - h / 2.0), Pos2::new(x, dial.center().y + h / 2.0)], Stroke::new(1.0, tick_color.gamma_multiply(fade)));
    }
    painter.line_segment([Pos2::new(cx, dial.min.y + 3.0), Pos2::new(cx, dial.max.y - 3.0)], Stroke::new(2.0, style.selection));
}

fn tooltip(ui: &mut Ui, view: &DiagramView, model: &Model, overlays: &Overlays, style: &DiagramStyle, h: Ref, extra: TooltipExtra<'_>) {
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
                    find_explanation(ui, view, model, style, h);
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
                    find_explanation(ui, view, model, style, h);
                }
            }
        });
    });
}

fn dot(ui: &mut Ui, color: egui::Color32) {
    let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
    ui.painter().circle_filled(r.center(), 4.0, color);
}

/// Striped swatch for the "both" legend entry.
fn striped_dot(ui: &mut Ui, a: egui::Color32, b: egui::Color32) {
    let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
    let p = ui.painter().with_clip_rect(r);
    p.rect_filled(r, 2.0, a);
    let mut x = r.min.x - r.height();
    while x < r.max.x {
        p.line_segment([Pos2::new(x, r.min.y), Pos2::new(x + r.height(), r.max.y)], Stroke::new(2.0, b));
        x += 4.0;
    }
}

/// Colour-coded lines in the hover tooltip saying why a highlighted element matched the find query.
fn find_explanation(ui: &mut Ui, view: &DiagramView, model: &Model, style: &DiagramStyle, h: Ref) {
    if !view.find.active() {
        return;
    }
    let el = view.find.matches.iter().find(|m| m.target == h);
    let hits: Vec<&crate::history::InstanceHit> = match h {
        Ref::Element(i) => view.find.instance_hits.iter().filter(|x| x.activity_id == model.elements[i].id).collect(),
        Ref::Flow(_) => vec![],
    };
    let is_current = view.find.current_in(model) == Some(h);
    if el.is_none() && hits.is_empty() {
        return;
    }
    ui.separator();
    if let Some(m) = el {
        ui.horizontal(|ui| {
            dot(ui, style.matched);
            ui.label(egui::RichText::new(format!("Element matches: {} “{}”", m.field, crate::util::ellipsize(&m.snippet, 60))).size(11.5));
        });
    }
    if !hits.is_empty() {
        ui.horizontal(|ui| {
            dot(ui, style.external);
            let n = hits.len();
            ui.label(egui::RichText::new(format!("{n} running instance{} here match{}:", if n == 1 { "" } else { "s" }, if n == 1 { "es" } else { "" })).size(11.5));
        });
        for x in hits.iter().take(4) {
            let what = match &x.business_key {
                Some(bk) => format!("business key {bk}"),
                None => format!("id {}", x.process_instance_id),
            };
            ui.label(egui::RichText::new(format!("    {what}")).monospace().size(11.0));
        }
        if hits.len() > 4 {
            ui.label(egui::RichText::new(format!("    … and {} more", hits.len() - 4)).weak().size(11.0));
        }
    }
    if el.is_some() && !hits.is_empty() {
        ui.horizontal(|ui| {
            striped_dot(ui, style.matched, style.external);
            ui.label(egui::RichText::new("striped: both the element and an instance match").weak().size(11.0));
        });
    }
    if is_current {
        ui.horizontal(|ui| {
            dot(ui, style.current_match);
            let n = view.find.targets(model).len();
            ui.label(egui::RichText::new(format!("Current match {} of {n} (Enter / Shift+Enter cycle)", view.find.cursor + 1)).size(11.5));
        });
    }
}

/// Legend in the top-right corner of the canvas while a find query has matches; lists only
/// the colours currently on screen.
fn find_legend(ui: &mut Ui, view: &DiagramView, model: &Model, style: &DiagramStyle, rect: Rect) {
    if !view.find.active() {
        return;
    }
    let el_set: std::collections::HashSet<Ref> = view.find.matches.iter().map(|m| m.target).collect();
    let inst_set = view.find.instance_match_refs(model);
    let both = el_set.intersection(&inst_set).count();
    let el_only = el_set.len() - both;
    let inst_only = inst_set.len() - both;
    let current = view.find.current_in(model);
    if el_only == 0 && inst_only == 0 && both == 0 {
        return;
    }
    let size = Vec2::new(236.0, 0.0);
    let pos = Pos2::new(rect.max.x - size.x - 12.0, rect.min.y + 10.0);
    let legend_rect = Rect::from_min_size(pos, Vec2::new(size.x, 120.0));
    ui.scope_builder(egui::UiBuilder::new().max_rect(legend_rect).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
        egui::Frame::new().fill(ui.visuals().window_fill.gamma_multiply(0.94)).stroke(ui.visuals().window_stroke).corner_radius(CornerRadius::same(6)).inner_margin(egui::Margin::symmetric(8, 6)).show(ui, |ui| {
            ui.set_width(size.x - 16.0);
            ui.spacing_mut().item_spacing.y = 3.0;
            let row = |ui: &mut Ui, text: String| {
                ui.label(egui::RichText::new(text).size(11.0));
            };
            if el_only > 0 {
                ui.horizontal(|ui| {
                    dot(ui, style.matched);
                    row(ui, format!("element text matches ({el_only})"));
                });
            }
            if inst_only > 0 {
                ui.horizontal(|ui| {
                    dot(ui, style.external);
                    row(ui, format!("a running instance here matches ({inst_only})"));
                });
            }
            if both > 0 {
                ui.horizontal(|ui| {
                    striped_dot(ui, style.matched, style.external);
                    row(ui, format!("both ({both})"));
                });
            }
            if current.is_some() {
                ui.horizontal(|ui| {
                    dot(ui, style.current_match);
                    row(ui, "current match · Enter cycles".to_string());
                });
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

/// A small "Alt ×2" pill that fades in and slowly orbits the cursor while Alt is held over the
/// diagram, so the doubled speed is never a surprise.
fn alt_badge(ctx: &egui::Context, view: &DiagramView, style: &DiagramStyle, rect: Rect, over_canvas: bool, pointer: Option<Pos2>) {
    let alt = ctx.input(|i| i.modifiers.alt) && over_canvas && pointer.is_some();
    let t = ctx.animate_bool_with_time(view.id.with("alt-badge"), alt, 0.18);
    if t <= 0.0 {
        return;
    }
    let Some(p) = pointer else { return };
    ctx.request_repaint(); // keep orbiting
    let time = ctx.input(|i| i.time) as f32;
    let eased = 1.0 - (1.0 - t) * (1.0 - t);
    let radius = 30.0 - 10.0 * eased; // settles from 30 px to 20 px
    let angle = time * 1.1; // ~10 s per orbit, counter-clockwise
    let center = p + Vec2::angled(-angle) * radius;
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, view.id.with("alt-badge-layer"))).with_clip_rect(rect);
    let font = FontId::proportional(10.5);
    let galley = painter.layout_no_wrap("Alt ×2".to_string(), font, style.badge_text);
    let size = galley.size() + Vec2::new(12.0, 6.0);
    let pill = Rect::from_center_size(center, size);
    let fill = style.selection.gamma_multiply(0.92 * eased);
    painter.rect_filled(pill.expand(1.5), 9.0, Color32::from_black_alpha((40.0 * eased) as u8));
    painter.rect_filled(pill, 8.0, fill);
    painter.galley(pill.center() - galley.size() / 2.0, galley, style.badge_text.gamma_multiply(eased));
}
