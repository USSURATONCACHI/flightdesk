//! Paints a BPMN model through a camera with level of detail and runtime overlays.

use super::camera::Camera;
use super::icons;
use bpmn::{Element, ElementKind, EventDef, Flow, FlowKind, Model, Ref};
use egui::epaint::{PathShape, PathStroke, TextShape};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};
use std::collections::{HashMap, HashSet};

/// Colours used by the renderer; filled from the app theme.
#[derive(Clone, Debug)]
pub struct DiagramStyle {
    pub canvas: Color32,
    pub grid: Color32,
    pub fill: Color32,
    pub stroke: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub lane_fill: Color32,
    pub lane_stroke: Color32,
    pub flow: Color32,
    pub running: Color32,
    pub incident: Color32,
    pub completed: Color32,
    pub suspended: Color32,
    pub selection: Color32,
    pub hover: Color32,
    pub matched: Color32,
    pub current_match: Color32,
    pub external: Color32,
    pub badge_text: Color32,
    pub dark: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BadgeKind {
    Running,
    Incident,
    Completed,
    Timer,
}

#[derive(Clone, Debug, Default)]
pub struct Overlays {
    pub running: HashMap<String, u32>,
    pub incidents: HashMap<String, u32>,
    /// Historic visits per activity id (finished activity instances).
    pub completed: HashMap<String, u32>,
    /// Sequence flows inferred as traversed.
    pub traversed_flows: HashSet<usize>,
    /// Activities currently active in the instance (glow).
    pub active: HashSet<String>,
    /// Cancelled historic activities.
    pub canceled: HashSet<String>,
    /// Timer label per activity id ("in 2 h 13 m", "overdue").
    pub timers: HashMap<String, (String, bool)>,
    /// Heat 0..1 per activity id.
    pub heat: HashMap<String, f32>,
    pub matches: HashSet<Ref>,
    pub current_match: Option<Ref>,
    /// When true, everything not matched is dimmed.
    pub dim_unmatched: bool,
    pub selection: Option<Ref>,
    pub multi_selection: HashSet<usize>,
    pub hover: Option<Ref>,
    pub show_ids: bool,
    pub show_grid: bool,
    pub suspended: bool,
    /// Elements to highlight softly (e.g. neighbours of the hovered element).
    pub related: HashSet<Ref>,
}

/// A clickable badge drawn this frame (screen rect).
#[derive(Clone, Debug)]
pub struct BadgeHit {
    pub rect: Rect,
    pub kind: BadgeKind,
    pub activity_id: String,
}

pub struct Renderer<'a> {
    pub model: &'a Model,
    pub camera: &'a Camera,
    pub viewport: Rect,
    pub painter: &'a Painter,
    pub style: &'a DiagramStyle,
    pub overlays: &'a Overlays,
    pub badges: Vec<BadgeHit>,
    zoom: f32,
    visible: Rect,
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
enum Lod {
    Full,
    Medium,
    Low,
    Blocks,
}

const BPMN_FONT: f32 = 12.0;

impl<'a> Renderer<'a> {
    pub fn new(model: &'a Model, camera: &'a Camera, viewport: Rect, painter: &'a Painter, style: &'a DiagramStyle, overlays: &'a Overlays) -> Self {
        let zoom = camera.zoom();
        let visible = camera.visible_world_rect(viewport).expand(60.0 / zoom.max(0.01));
        Self { model, camera, viewport, painter, style, overlays, badges: vec![], zoom, visible }
    }

    fn lod(&self) -> Lod {
        let z = self.zoom;
        if z >= 0.6 {
            Lod::Full
        } else if z >= 0.3 {
            Lod::Medium
        } else if z >= 0.12 {
            Lod::Low
        } else {
            Lod::Blocks
        }
    }

    #[inline]
    fn ws(&self, p: bpmn::Pos) -> Pos2 {
        self.camera.world_to_screen(self.viewport, Pos2::new(p.x, p.y))
    }
    #[inline]
    fn wr(&self, r: bpmn::Rect) -> Rect {
        let min = self.ws(r.min());
        Rect::from_min_size(min, Vec2::new(r.w * self.zoom, r.h * self.zoom))
    }
    #[inline]
    fn px(&self, world: f32) -> f32 {
        world * self.zoom
    }
    fn visible_world(&self, r: &bpmn::Rect) -> bool {
        r.is_valid() && r.x <= self.visible.max.x && r.x + r.w >= self.visible.min.x && r.y <= self.visible.max.y && r.y + r.h >= self.visible.min.y
    }

    fn alpha_for(&self, r: Ref) -> f32 {
        if self.overlays.dim_unmatched && !self.overlays.matches.contains(&r) {
            0.22
        } else {
            1.0
        }
    }

    fn element_colors(&self, idx: usize, e: &Element) -> (Color32, Color32, Color32) {
        let st = self.style;
        let mut fill = st.fill;
        let mut stroke = st.stroke;
        let a = self.alpha_for(Ref::Element(idx));
        if let Some(h) = self.overlays.heat.get(&e.id) {
            fill = blend(fill, st.running, 0.08 + 0.55 * h);
        }
        if self.overlays.completed.contains_key(&e.id) {
            fill = blend(fill, st.completed, if st.dark { 0.28 } else { 0.18 });
            stroke = blend(stroke, st.completed, 0.6);
        }
        if self.overlays.canceled.contains(&e.id) {
            stroke = blend(stroke, st.incident, 0.5);
        }
        if self.overlays.active.contains(&e.id) || self.overlays.running.contains_key(&e.id) {
            stroke = st.running;
            fill = blend(fill, st.running, if st.dark { 0.22 } else { 0.12 });
        }
        if self.overlays.incidents.contains_key(&e.id) {
            stroke = st.incident;
            fill = blend(fill, st.incident, if st.dark { 0.22 } else { 0.1 });
        }
        if self.overlays.suspended {
            stroke = blend(stroke, st.suspended, 0.5);
        }
        let text = st.text;
        (fill.gamma_multiply(a), stroke.gamma_multiply(a), text.gamma_multiply(a))
    }

    pub fn paint(&mut self) {
        self.painter.rect_filled(self.viewport, 0.0, self.style.canvas);
        if self.overlays.show_grid {
            self.paint_grid();
        }
        let lod = self.lod();
        let m = self.model;
        // Pass 1: pools, lanes, groups, expanded containers.
        let mut nodes: Vec<usize> = Vec::with_capacity(m.draw_order.len());
        let mut artifacts: Vec<usize> = vec![];
        for &i in &m.draw_order {
            let e = &m.elements[i];
            if !self.visible_world(&e.bounds) {
                continue;
            }
            match e.kind {
                ElementKind::Participant => self.paint_pool(i, e, lod),
                ElementKind::Lane => self.paint_lane(i, e, lod),
                ElementKind::Group => self.paint_group(i, e),
                _ if e.kind.is_container() && e.is_expanded => self.paint_subprocess(i, e, lod),
                ElementKind::TextAnnotation | ElementKind::DataObjectReference | ElementKind::DataStoreReference | ElementKind::DataInput | ElementKind::DataOutput => artifacts.push(i),
                _ => nodes.push(i),
            }
        }
        // Pass 2: flows.
        for (fi, f) in m.flows.iter().enumerate() {
            if f.waypoints.len() < 2 || !self.visible_world(&f.bounds) {
                continue;
            }
            self.paint_flow(fi, f, lod);
        }
        // Pass 3: nodes, then artifacts, then highlights, badges.
        for &i in &nodes {
            self.paint_node(i, &m.elements[i], lod);
        }
        for &i in &artifacts {
            self.paint_artifact(i, &m.elements[i], lod);
        }
        if lod <= Lod::Medium {
            for (fi, f) in m.flows.iter().enumerate() {
                if f.waypoints.len() >= 2 && self.visible_world(&f.bounds) {
                    self.paint_flow_label(fi, f);
                }
            }
        }
        for &i in m.draw_order.iter() {
            let e = &m.elements[i];
            if self.visible_world(&e.bounds) && (e.kind.is_flow_node() || e.kind.is_artifact()) {
                self.paint_highlights(i, e);
            }
        }
        for &i in m.draw_order.iter() {
            let e = &m.elements[i];
            if self.visible_world(&e.bounds) && e.kind.is_flow_node() {
                self.paint_badges(e);
            }
        }
    }

    fn paint_grid(&self) {
        let step_world = [10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0].into_iter().find(|s| s * self.zoom >= 24.0).unwrap_or(2000.0);
        let vis = self.camera.visible_world_rect(self.viewport);
        let x0 = (vis.min.x / step_world).floor() * step_world;
        let y0 = (vis.min.y / step_world).floor() * step_world;
        let mut x = x0;
        let col = self.style.grid;
        while x <= vis.max.x {
            let mut y = y0;
            while y <= vis.max.y {
                let p = self.camera.world_to_screen(self.viewport, Pos2::new(x, y));
                self.painter.circle_filled(p, 1.0, col);
                y += step_world;
            }
            x += step_world;
        }
    }

    fn paint_pool(&mut self, idx: usize, e: &Element, lod: Lod) {
        let r = self.wr(e.bounds);
        let a = self.alpha_for(Ref::Element(idx));
        let st = self.style;
        self.painter.rect(r, 0.0, st.lane_fill.gamma_multiply(a), Stroke::new(self.px(1.5).clamp(1.0, 3.0), st.lane_stroke.gamma_multiply(a)), StrokeKind::Inside);
        let header = self.px(30.0);
        if e.is_horizontal {
            self.painter.line_segment([Pos2::new(r.min.x + header, r.min.y), Pos2::new(r.min.x + header, r.max.y)], Stroke::new(self.px(1.0).clamp(0.5, 2.0), st.lane_stroke.gamma_multiply(a)));
            if lod <= Lod::Low {
                self.rotated_label(e.display_name(), Rect::from_min_max(r.min, Pos2::new(r.min.x + header, r.max.y)), st.text.gamma_multiply(a), true);
            }
        } else {
            self.painter.line_segment([Pos2::new(r.min.x, r.min.y + header), Pos2::new(r.max.x, r.min.y + header)], Stroke::new(self.px(1.0).clamp(0.5, 2.0), st.lane_stroke.gamma_multiply(a)));
            if lod <= Lod::Low {
                self.label_in_rect(e.display_name(), Rect::from_min_max(r.min, Pos2::new(r.max.x, r.min.y + header)), st.text.gamma_multiply(a), true, Align2::CENTER_CENTER);
            }
        }
    }

    fn paint_lane(&mut self, idx: usize, e: &Element, lod: Lod) {
        let r = self.wr(e.bounds);
        let a = self.alpha_for(Ref::Element(idx));
        let st = self.style;
        self.painter.rect_stroke(r, 0.0, Stroke::new(self.px(1.0).clamp(0.5, 2.0), st.lane_stroke.gamma_multiply(a)), StrokeKind::Inside);
        let header = self.px(30.0);
        if e.is_horizontal {
            self.painter.line_segment([Pos2::new(r.min.x + header, r.min.y), Pos2::new(r.min.x + header, r.max.y)], Stroke::new(self.px(1.0).clamp(0.5, 2.0), st.lane_stroke.gamma_multiply(a)));
            if lod <= Lod::Low {
                self.rotated_label(e.display_name(), Rect::from_min_max(r.min, Pos2::new(r.min.x + header, r.max.y)), st.text.gamma_multiply(a), false);
            }
        } else {
            self.painter.line_segment([Pos2::new(r.min.x, r.min.y + header), Pos2::new(r.max.x, r.min.y + header)], Stroke::new(self.px(1.0).clamp(0.5, 2.0), st.lane_stroke.gamma_multiply(a)));
            if lod <= Lod::Low {
                self.label_in_rect(e.display_name(), Rect::from_min_max(r.min, Pos2::new(r.max.x, r.min.y + header)), st.text.gamma_multiply(a), false, Align2::CENTER_CENTER);
            }
        }
    }

    fn paint_group(&mut self, idx: usize, e: &Element) {
        let r = self.wr(e.bounds);
        let a = self.alpha_for(Ref::Element(idx));
        let s = Stroke::new(self.px(1.5).clamp(1.0, 3.0), self.style.stroke.gamma_multiply(a * 0.7));
        self.dashed_rect(r, self.px(8.0), self.px(4.0), s, self.px(10.0));
        if let Some(n) = &e.name {
            self.label_in_rect(n, Rect::from_min_size(r.min + Vec2::new(self.px(6.0), self.px(4.0)), Vec2::new(r.width(), self.px(16.0))), self.style.text_muted.gamma_multiply(a), false, Align2::LEFT_TOP);
        }
    }

    fn paint_subprocess(&mut self, idx: usize, e: &Element, lod: Lod) {
        let r = self.wr(e.bounds);
        let (fill, stroke, text) = self.element_colors(idx, e);
        let fill = blend(fill, self.style.canvas, 0.35);
        let w = self.px(1.5).clamp(1.0, 3.0);
        let cr = self.px(10.0);
        if e.triggered_by_event {
            self.painter.rect_filled(r, cr, fill);
            self.dashed_rect(r, self.px(3.0), self.px(3.0), Stroke::new(w, stroke), cr);
        } else {
            self.painter.rect(r, cr, fill, Stroke::new(w, stroke), StrokeKind::Inside);
        }
        if lod <= Lod::Low {
            let header = Rect::from_min_size(r.min + Vec2::new(self.px(8.0), self.px(4.0)), Vec2::new(r.width() - self.px(16.0), self.px(18.0)));
            self.label_in_rect(e.display_name(), header, text, true, Align2::LEFT_TOP);
        }
        if lod <= Lod::Medium {
            self.paint_activity_markers(e, r, stroke, false);
        }
    }

    fn paint_node(&mut self, idx: usize, e: &Element, lod: Lod) {
        let r = self.wr(e.bounds);
        let (fill, stroke, text) = self.element_colors(idx, e);
        if lod == Lod::Blocks {
            let c = if e.kind.is_event() { blend(stroke, fill, 0.3) } else { blend(stroke, fill, 0.55) };
            if e.kind.is_gateway() {
                self.diamond(r, c, Stroke::NONE);
            } else if e.kind.is_event() {
                self.painter.circle_filled(r.center(), r.width() / 2.0, c);
            } else {
                self.painter.rect_filled(r, self.px(10.0), c);
            }
            return;
        }
        let w = self.px(1.5).clamp(1.0, 3.0);
        match &e.kind {
            k if k.is_task() || *k == ElementKind::CallActivity || (k.is_container() && !e.is_expanded) => {
                let cr = self.px(10.0);
                self.painter.rect_filled(r, cr, fill);
                let bw = if *k == ElementKind::CallActivity { self.px(4.0).clamp(2.0, 6.0) } else { w };
                self.painter.rect_stroke(r, cr, Stroke::new(bw, stroke), StrokeKind::Inside);
                if lod <= Lod::Medium {
                    let icon = Rect::from_min_size(r.min + Vec2::splat(self.px(5.0)), Vec2::splat(self.px(16.0)));
                    self.paint_task_icon(k, e, icon, stroke);
                    self.paint_activity_markers(e, r, stroke, k.is_container());
                    let inner = r.shrink2(Vec2::new(self.px(6.0), self.px(6.0)));
                    let name = e.display_name();
                    self.label_in_rect(name, inner, text, false, Align2::CENTER_CENTER);
                    if self.overlays.show_ids {
                        self.label_in_rect(&e.id, Rect::from_min_max(Pos2::new(inner.min.x, inner.max.y - self.px(12.0)), inner.max), self.style.text_muted.gamma_multiply(text.a() as f32 / 255.0), false, Align2::CENTER_BOTTOM);
                    }
                }
            }
            k if k.is_gateway() => {
                self.diamond(r, fill, Stroke::new(w, stroke));
                if lod <= Lod::Medium {
                    let inner = r.shrink(r.width() * 0.1);
                    let mw = self.px(2.5).clamp(1.5, 5.0);
                    match k {
                        ElementKind::ExclusiveGateway => {
                            if e.marker_visible {
                                icons::gw_exclusive(self.painter, inner, stroke, mw);
                            }
                        }
                        ElementKind::ParallelGateway => icons::gw_parallel(self.painter, inner, stroke, mw),
                        ElementKind::InclusiveGateway => icons::gw_inclusive(self.painter, inner, stroke, mw),
                        ElementKind::EventBasedGateway => icons::gw_event_based(self.painter, inner, stroke, mw, e.attr("instantiate") == Some("true")),
                        ElementKind::ComplexGateway => icons::gw_complex(self.painter, inner, stroke, mw),
                        _ => {}
                    }
                }
                if lod <= Lod::Medium {
                    self.external_label(e, r, text);
                }
            }
            k if k.is_event() => {
                let c = r.center();
                let rad = r.width() / 2.0;
                let throwing = e.is_throwing();
                let end = *k == ElementKind::EndEvent;
                let intermediate = matches!(k, ElementKind::IntermediateCatchEvent | ElementKind::IntermediateThrowEvent | ElementKind::BoundaryEvent);
                self.painter.circle_filled(c, rad, fill);
                let bw = if end { self.px(4.0).clamp(2.0, 6.0) } else { w };
                if e.interrupting {
                    self.painter.circle_stroke(c, rad - bw / 2.0, Stroke::new(bw, stroke));
                    if intermediate {
                        self.painter.circle_stroke(c, rad - bw / 2.0 - self.px(3.0), Stroke::new(w, stroke));
                    }
                } else {
                    self.dashed_circle(c, rad - bw / 2.0, Stroke::new(bw, stroke));
                    if intermediate {
                        self.dashed_circle(c, rad - bw / 2.0 - self.px(3.0), Stroke::new(w, stroke));
                    }
                }
                if lod <= Lod::Low {
                    let icon = Rect::from_center_size(c, Vec2::splat(rad * 1.1));
                    self.paint_event_icon(e, icon, stroke, throwing);
                }
                if lod <= Lod::Medium {
                    self.external_label(e, r, text);
                }
            }
            _ => {
                self.painter.rect(r, 0.0, fill, Stroke::new(w, stroke), StrokeKind::Inside);
            }
        }
    }

    fn paint_task_icon(&self, k: &ElementKind, e: &Element, icon: Rect, color: Color32) {
        let w = self.px(1.3).clamp(1.0, 2.5);
        match k {
            ElementKind::UserTask => icons::user(self.painter, icon, color, w),
            ElementKind::ServiceTask => {
                if e.camunda("type") == Some("external") {
                    icons::gear(self.painter, icon, self.style.external, w);
                } else {
                    icons::gear(self.painter, icon, color, w);
                }
            }
            ElementKind::ScriptTask => icons::script(self.painter, icon, color, w),
            ElementKind::SendTask => icons::envelope(self.painter, icon, color, w, true),
            ElementKind::ReceiveTask => icons::receive(self.painter, icon, color, w),
            ElementKind::ManualTask => icons::hand(self.painter, icon, color, w),
            ElementKind::BusinessRuleTask => icons::table(self.painter, icon, color, w),
            _ => {}
        }
    }

    fn paint_event_icon(&self, e: &Element, icon: Rect, color: Color32, throwing: bool) {
        let w = self.px(1.2).clamp(0.8, 2.5);
        let Some(def) = e.event_defs.first() else { return };
        match def {
            EventDef::Timer { .. } => icons::clock(self.painter, icon.center(), icon.width() * 0.42, color, w),
            EventDef::Message { .. } => icons::envelope(self.painter, icon.shrink(icon.width() * 0.08), color, w, throwing),
            EventDef::Signal { .. } => icons::triangle(self.painter, icon.shrink(icon.width() * 0.1), color, w, throwing),
            EventDef::Error { .. } => icons::lightning(self.painter, icon.shrink(icon.width() * 0.05), color, w, throwing),
            EventDef::Escalation { .. } => icons::chevron_up(self.painter, icon.shrink(icon.width() * 0.1), color, w, throwing),
            EventDef::Conditional { .. } => icons::conditional(self.painter, icon.shrink(icon.width() * 0.1), color, w),
            EventDef::Link { .. } => icons::link_arrow(self.painter, icon.shrink(icon.width() * 0.1), color, w, throwing),
            EventDef::Compensate { .. } => icons::compensation(self.painter, icon.shrink(icon.width() * 0.15), color, w, throwing),
            EventDef::Terminate => icons::terminate(self.painter, icon.center(), icon.width() * 0.3, color),
            EventDef::Cancel => icons::cancel_x(self.painter, icon.shrink(icon.width() * 0.15), color, w, throwing),
        }
    }

    fn paint_activity_markers(&self, e: &Element, r: Rect, color: Color32, collapsed_container: bool) {
        let size = self.px(12.0);
        let mut markers: Vec<fn(&Painter, Rect, Color32, f32)> = vec![];
        if collapsed_container {
            markers.push(icons::marker_collapsed);
        }
        if e.standard_loop {
            markers.push(icons::marker_loop);
        }
        if let Some(mi) = &e.multi_instance {
            markers.push(if mi.sequential { icons::marker_mi_sequential } else { icons::marker_mi_parallel });
        }
        if e.is_for_compensation {
            markers.push(icons::marker_compensation);
        }
        if e.kind == ElementKind::AdHocSubProcess {
            markers.push(icons::marker_adhoc);
        }
        if markers.is_empty() {
            return;
        }
        let total = markers.len() as f32 * size + (markers.len() as f32 - 1.0) * self.px(3.0);
        let mut x = r.center().x - total / 2.0;
        let y = r.max.y - size - self.px(4.0);
        let w = self.px(1.3).clamp(1.0, 2.5);
        for m in markers {
            m(self.painter, Rect::from_min_size(Pos2::new(x, y), Vec2::splat(size)), color, w);
            x += size + self.px(3.0);
        }
    }

    fn paint_artifact(&mut self, idx: usize, e: &Element, lod: Lod) {
        let r = self.wr(e.bounds);
        let (fill, stroke, text) = self.element_colors(idx, e);
        let w = self.px(1.2).clamp(0.8, 2.5);
        match e.kind {
            ElementKind::TextAnnotation => {
                let bw = self.px(10.0);
                self.painter.add(Shape::line(vec![Pos2::new(r.min.x + bw, r.min.y), r.min, Pos2::new(r.min.x, r.max.y), Pos2::new(r.min.x + bw, r.max.y)], PathStroke::from(Stroke::new(w, stroke))));
                if lod <= Lod::Medium {
                    self.label_in_rect(e.display_name(), r.shrink2(Vec2::new(self.px(6.0), self.px(3.0))), text, false, Align2::LEFT_TOP);
                }
            }
            ElementKind::DataStoreReference => {
                let cr = self.px(6.0);
                self.painter.rect(r, cr, fill, Stroke::new(w, stroke), StrokeKind::Inside);
                for i in 1..3 {
                    let y = r.min.y + r.height() * 0.15 * i as f32;
                    self.painter.line_segment([Pos2::new(r.min.x, y), Pos2::new(r.max.x, y)], Stroke::new(w, stroke));
                }
                self.external_label(e, r, text);
            }
            _ => {
                let fold = self.px(10.0);
                let pts = vec![r.min, Pos2::new(r.max.x - fold, r.min.y), Pos2::new(r.max.x, r.min.y + fold), r.max, Pos2::new(r.min.x, r.max.y)];
                self.painter.add(PathShape::convex_polygon(pts, fill, PathStroke::from(Stroke::new(w, stroke))));
                self.external_label(e, r, text);
            }
        }
    }

    fn paint_flow(&mut self, fi: usize, f: &Flow, lod: Lod) {
        let a = self.alpha_for(Ref::Flow(fi));
        let st = self.style;
        let traversed = self.overlays.traversed_flows.contains(&fi);
        let mut color = if traversed { st.completed } else { st.flow };
        let mut width = if traversed { self.px(3.0).clamp(1.5, 5.0) } else { self.px(1.5).clamp(0.8, 3.0) };
        let selected = self.overlays.selection == Some(Ref::Flow(fi));
        let hovered = self.overlays.hover == Some(Ref::Flow(fi)) || self.overlays.related.contains(&Ref::Flow(fi));
        if selected {
            color = st.selection;
            width *= 1.6;
        } else if hovered {
            color = st.hover;
            width *= 1.4;
        }
        if self.overlays.matches.contains(&Ref::Flow(fi)) {
            color = if self.overlays.current_match == Some(Ref::Flow(fi)) { st.current_match } else { st.matched };
            width *= 1.5;
        }
        let color = color.gamma_multiply(a);
        let pts: Vec<Pos2> = f.waypoints.iter().map(|p| self.ws(*p)).collect();
        let stroke = Stroke::new(width, color);
        match f.kind {
            FlowKind::SequenceFlow => {
                self.painter.add(Shape::line(pts.clone(), PathStroke::from(stroke)));
                if lod <= Lod::Low {
                    self.arrow_head(&pts, color, true, width);
                    if f.is_default {
                        self.default_marker(&pts, stroke);
                    } else if f.condition.is_some() && f.source.map(|s| !self.model.elements[s].kind.is_gateway()).unwrap_or(false) {
                        self.conditional_marker(&pts, color, stroke);
                    }
                }
            }
            FlowKind::MessageFlow => {
                self.dashed_polyline(&pts, stroke, self.px(8.0), self.px(5.0));
                if lod <= Lod::Low {
                    self.arrow_head(&pts, color, false, width);
                    self.painter.circle(pts[0], self.px(4.0), st.canvas, stroke);
                }
            }
            FlowKind::Association => {
                self.dashed_polyline(&pts, stroke, self.px(1.5), self.px(4.0));
            }
            FlowKind::DataInputAssociation | FlowKind::DataOutputAssociation => {
                self.dashed_polyline(&pts, stroke, self.px(1.5), self.px(4.0));
                if lod <= Lod::Low {
                    self.arrow_head(&pts, color, false, width);
                }
            }
        }
    }

    fn paint_flow_label(&mut self, fi: usize, f: &Flow) {
        let Some(name) = f.name.as_deref().filter(|n| !n.trim().is_empty()) else { return };
        let a = self.alpha_for(Ref::Flow(fi));
        let color = self.style.text_muted.gamma_multiply(a);
        let rect = if let Some(lb) = f.label_bounds {
            self.wr(lb)
        } else {
            let mid = polyline_midpoint(&f.waypoints);
            Rect::from_center_size(self.ws(mid) + Vec2::new(0.0, -self.px(10.0)), Vec2::new(self.px(90.0), self.px(14.0)))
        };
        self.label_free(name, rect, color, Align2::CENTER_CENTER);
    }

    fn paint_highlights(&mut self, idx: usize, e: &Element) {
        let r = self.wr(e.bounds);
        let st = self.style;
        let rref = Ref::Element(idx);
        let mut ring: Option<(Color32, f32)> = None;
        if self.overlays.multi_selection.contains(&idx) {
            ring = Some((st.selection, 3.0));
        }
        if self.overlays.selection == Some(rref) {
            ring = Some((st.selection, 3.0));
        } else if self.overlays.hover == Some(rref) {
            ring = Some((st.hover, 2.0));
        } else if self.overlays.related.contains(&rref) {
            ring = Some((st.hover.gamma_multiply(0.6), 2.0));
        }
        if self.overlays.matches.contains(&rref) {
            let cur = self.overlays.current_match == Some(rref);
            ring = Some((if cur { st.current_match } else { st.matched }, if cur { 4.0 } else { 2.5 }));
        }
        if let Some((color, w)) = ring {
            let pad = w + 2.0;
            let rr = r.expand(pad);
            if e.kind.is_event() {
                self.painter.circle_stroke(r.center(), r.width() / 2.0 + pad, Stroke::new(w, color));
            } else if e.kind.is_gateway() {
                self.diamond(rr, Color32::TRANSPARENT, Stroke::new(w, color));
            } else {
                self.painter.rect_stroke(rr, self.px(10.0) + pad, Stroke::new(w, color), StrokeKind::Middle);
            }
        }
        if self.overlays.active.contains(&e.id) && !self.overlays.incidents.contains_key(&e.id) {
            // soft glow for live tokens
            for i in 1..=3 {
                let g = st.running.gamma_multiply(0.10 / i as f32);
                let rr = r.expand(i as f32 * 3.0);
                if e.kind.is_event() {
                    self.painter.circle_stroke(r.center(), r.width() / 2.0 + i as f32 * 3.0, Stroke::new(3.0, g));
                } else {
                    self.painter.rect_stroke(rr, self.px(10.0) + i as f32 * 3.0, Stroke::new(3.0, g), StrokeKind::Middle);
                }
            }
        }
    }

    fn paint_badges(&mut self, e: &Element) {
        let r = self.wr(e.bounds);
        let id = &e.id;
        let st = self.style;
        if let Some(&n) = self.overlays.running.get(id) {
            let rect = self.badge(Pos2::new(r.max.x, r.min.y), &n.to_string(), st.running, Align2::CENTER_CENTER);
            self.badges.push(BadgeHit { rect, kind: BadgeKind::Running, activity_id: id.clone() });
        }
        if let Some(&n) = self.overlays.incidents.get(id) {
            let rect = self.badge(Pos2::new(r.min.x, r.min.y), &n.to_string(), st.incident, Align2::CENTER_CENTER);
            self.badges.push(BadgeHit { rect, kind: BadgeKind::Incident, activity_id: id.clone() });
        }
        if let Some(&n) = self.overlays.completed.get(id) {
            if n > 1 {
                let rect = self.badge(Pos2::new(r.max.x, r.max.y), &format!("×{n}"), st.completed, Align2::CENTER_CENTER);
                self.badges.push(BadgeHit { rect, kind: BadgeKind::Completed, activity_id: id.clone() });
            }
        }
        if let Some((label, overdue)) = self.overlays.timers.get(id) {
            let color = if *overdue { st.incident } else { st.suspended };
            let anchor = Pos2::new(r.center().x, r.max.y + 6.0);
            let rect = self.badge(anchor, label, color, Align2::CENTER_TOP);
            self.badges.push(BadgeHit { rect, kind: BadgeKind::Timer, activity_id: id.clone() });
        }
    }

    /// Constant screen-size pill with text; returns its rect.
    fn badge(&self, anchor: Pos2, text: &str, color: Color32, align: Align2) -> Rect {
        let font = FontId::proportional(11.0);
        let galley = self.painter.layout_no_wrap(text.to_string(), font, self.style.badge_text);
        let size = Vec2::new((galley.size().x + 10.0).max(18.0), 18.0);
        let rect = align.anchor_size(anchor, size);
        self.painter.rect_filled(rect, 9.0, color);
        self.painter.galley(rect.center() - galley.size() / 2.0, galley, self.style.badge_text);
        rect
    }

    // ---------- primitives ----------

    fn diamond(&self, r: Rect, fill: Color32, stroke: Stroke) {
        let c = r.center();
        let pts = vec![Pos2::new(c.x, r.min.y), Pos2::new(r.max.x, c.y), Pos2::new(c.x, r.max.y), Pos2::new(r.min.x, c.y)];
        self.painter.add(PathShape::convex_polygon(pts, fill, PathStroke::from(stroke)));
    }

    fn dashed_circle(&self, c: Pos2, rad: f32, stroke: Stroke) {
        let n = 64;
        let pts: Vec<Pos2> = (0..=n).map(|i| c + Vec2::angled(std::f32::consts::TAU * i as f32 / n as f32) * rad).collect();
        self.dashed_polyline(&pts, stroke, self.px(3.0).max(2.0), self.px(3.0).max(2.0));
    }

    fn dashed_rect(&self, r: Rect, dash: f32, gap: f32, stroke: Stroke, _cr: f32) {
        let pts = vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
        self.dashed_polyline(&pts, stroke, dash.max(2.0), gap.max(2.0));
    }

    fn dashed_polyline(&self, pts: &[Pos2], stroke: Stroke, dash: f32, gap: f32) {
        if dash + gap < 1.5 {
            self.painter.add(Shape::line(pts.to_vec(), PathStroke::from(stroke)));
            return;
        }
        self.painter.add(Shape::dashed_line(pts, stroke, dash, gap));
    }

    fn arrow_head(&self, pts: &[Pos2], color: Color32, filled: bool, width: f32) {
        let n = pts.len();
        if n < 2 {
            return;
        }
        let tip = pts[n - 1];
        let prev = pts[n - 2];
        let dir = (tip - prev).normalized();
        if !dir.is_finite() || dir == Vec2::ZERO {
            return;
        }
        let len = self.px(11.0).clamp(4.0, 24.0);
        let half = self.px(5.0).clamp(2.0, 11.0);
        let base = tip - dir * len;
        let n = dir.rot90();
        let a = base + n * half;
        let b = base - n * half;
        if filled {
            self.painter.add(PathShape::convex_polygon(vec![tip, a, b], color, PathStroke::NONE));
        } else {
            self.painter.add(PathShape::convex_polygon(vec![tip, a, b], self.style.canvas, PathStroke::from(Stroke::new(width, color))));
        }
    }

    fn default_marker(&self, pts: &[Pos2], stroke: Stroke) {
        let dir = (pts[1] - pts[0]).normalized();
        if !dir.is_finite() {
            return;
        }
        let at = pts[0] + dir * self.px(10.0);
        let n = dir.rot90();
        let l = self.px(5.0);
        self.painter.line_segment([at - dir * l * 0.6 + n * l, at + dir * l * 0.6 - n * l], stroke);
    }

    fn conditional_marker(&self, pts: &[Pos2], color: Color32, stroke: Stroke) {
        let dir = (pts[1] - pts[0]).normalized();
        if !dir.is_finite() {
            return;
        }
        let l = self.px(7.0);
        let c = pts[0] + dir * (l + 1.0);
        let n = dir.rot90();
        let poly = vec![c - dir * l, c + n * l * 0.5, c + dir * l, c - n * l * 0.5];
        let _ = color;
        self.painter.add(PathShape::convex_polygon(poly, self.style.canvas, PathStroke::from(stroke)));
    }

    /// Wrapped label centred in `rect`, shrinking the font when it does not fit.
    fn label_in_rect(&self, text: &str, rect: Rect, color: Color32, bold: bool, align: Align2) {
        if rect.width() < 4.0 || rect.height() < 4.0 {
            return;
        }
        let mut size = quantize(self.px(BPMN_FONT));
        if size < 3.5 {
            return;
        }
        let min_size = (size * 0.6).max(3.5);
        let text = text.trim();
        loop {
            let font = if bold { FontId::proportional(size) } else { FontId::proportional(size) };
            let galley = self.painter.layout(text.to_string(), font, color, rect.width());
            if galley.size().y <= rect.height() + 0.5 || size <= min_size {
                // Truncate rows that do not fit.
                let pos = align.align_size_within_rect(galley.size(), rect).min;
                let clip = rect.expand(1.0);
                let p = self.painter.with_clip_rect(clip.intersect(self.painter.clip_rect()));
                p.galley(pos, galley, color);
                return;
            }
            size = (size - 1.0).max(min_size);
        }
    }

    /// Label outside the shape (events, gateways): DI label bounds or below the element.
    fn external_label(&self, e: &Element, r: Rect, color: Color32) {
        let Some(name) = e.name.as_deref().filter(|n| !n.trim().is_empty()) else { return };
        if let Some(lb) = e.label_bounds {
            let lr = self.wr(lb);
            // DI label bounds are often too tight; allow growth around the centre.
            let lr = Rect::from_center_size(lr.center(), Vec2::new(lr.width().max(self.px(90.0)), lr.height().max(self.px(14.0))));
            self.label_free(name, lr, color, Align2::CENTER_TOP);
        } else {
            let lr = Rect::from_center_size(Pos2::new(r.center().x, r.max.y + self.px(12.0)), Vec2::new(self.px(100.0), self.px(28.0)));
            self.label_free(name, lr, color, Align2::CENTER_TOP);
        }
    }

    fn label_free(&self, text: &str, rect: Rect, color: Color32, align: Align2) {
        let size = quantize(self.px(BPMN_FONT));
        if size < 3.5 {
            return;
        }
        let galley = self.painter.layout(text.trim().to_string(), FontId::proportional(size), color, rect.width().max(self.px(60.0)));
        let pos = align.align_size_within_rect(galley.size(), rect).min;
        self.painter.galley(pos, galley, color);
    }

    fn rotated_label(&self, text: &str, strip: Rect, color: Color32, bold: bool) {
        let size = quantize(self.px(BPMN_FONT) * if bold { 1.1 } else { 1.0 });
        if size < 3.5 {
            return;
        }
        let galley = self.painter.layout(text.trim().to_string(), FontId::proportional(size), color, strip.height() - self.px(10.0));
        let gsize = galley.size();
        // Rotate -90°: text runs bottom→top; anchor is the galley's top-left after rotation.
        let center = strip.center();
        let pos = Pos2::new(center.x - gsize.y / 2.0, center.y + gsize.x / 2.0);
        let shape = TextShape::new(pos, galley, color).with_angle(-std::f32::consts::FRAC_PI_2);
        self.painter.add(shape);
    }
}

fn quantize(px: f32) -> f32 {
    (px * 2.0).round() / 2.0
}

pub fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

pub fn polyline_midpoint(pts: &[bpmn::Pos]) -> bpmn::Pos {
    if pts.is_empty() {
        return bpmn::Pos::default();
    }
    let total: f32 = pts.windows(2).map(|w| ((w[1].x - w[0].x).powi(2) + (w[1].y - w[0].y).powi(2)).sqrt()).sum();
    let mut acc = 0.0;
    for w in pts.windows(2) {
        let l = ((w[1].x - w[0].x).powi(2) + (w[1].y - w[0].y).powi(2)).sqrt();
        if acc + l >= total / 2.0 {
            let t = if l > 0.0 { (total / 2.0 - acc) / l } else { 0.0 };
            return bpmn::Pos { x: w[0].x + (w[1].x - w[0].x) * t, y: w[0].y + (w[1].y - w[0].y) * t };
        }
        acc += l;
    }
    pts[pts.len() / 2]
}
