//! Vector icons for BPMN element types and markers. Everything is drawn in a unit box
//! `rect` (screen space) with a stroke width `w`, no icon fonts involved.

use egui::epaint::{CubicBezierShape, PathShape, PathStroke};
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};
use std::f32::consts::TAU;

fn p(r: &Rect, x: f32, y: f32) -> Pos2 {
    Pos2::new(r.min.x + r.width() * x, r.min.y + r.height() * y)
}

fn line(painter: &Painter, r: &Rect, pts: &[(f32, f32)], stroke: Stroke) {
    let v: Vec<Pos2> = pts.iter().map(|&(x, y)| p(r, x, y)).collect();
    painter.add(Shape::line(v, PathStroke::from(stroke)));
}

fn closed(painter: &Painter, r: &Rect, pts: &[(f32, f32)], fill: Color32, stroke: Stroke) {
    let v: Vec<Pos2> = pts.iter().map(|&(x, y)| p(r, x, y)).collect();
    painter.add(Shape::convex_polygon(v, fill, PathStroke::from(stroke)));
}

pub fn user(painter: &Painter, r: Rect, color: Color32, w: f32) {
    let s = Stroke::new(w, color);
    // head
    painter.circle_stroke(p(&r, 0.5, 0.33), r.width() * 0.17, s);
    // shoulders
    let pts = vec![p(&r, 0.12, 0.95), p(&r, 0.14, 0.7), p(&r, 0.3, 0.6), p(&r, 0.7, 0.6), p(&r, 0.86, 0.7), p(&r, 0.88, 0.95)];
    painter.add(Shape::line(pts, PathStroke::from(s)));
    painter.line_segment([p(&r, 0.12, 0.95), p(&r, 0.88, 0.95)], s);
    painter.line_segment([p(&r, 0.32, 0.62), p(&r, 0.32, 0.95)], s);
    painter.line_segment([p(&r, 0.68, 0.62), p(&r, 0.68, 0.95)], s);
}

pub fn gear(painter: &Painter, r: Rect, color: Color32, w: f32) {
    let s = Stroke::new(w, color);
    let c = p(&r, 0.5, 0.5);
    let ro = r.width() * 0.42;
    let ri = r.width() * 0.3;
    let mut pts = Vec::with_capacity(32);
    let teeth = 8;
    for i in 0..teeth * 4 {
        let a = TAU * i as f32 / (teeth * 4) as f32 - TAU / 16.0;
        let rad = if (i / 2) % 2 == 0 { ro } else { ri };
        pts.push(c + Vec2::angled(a) * rad);
    }
    painter.add(Shape::closed_line(pts, PathStroke::from(s)));
    painter.circle_stroke(c, r.width() * 0.12, s);
}

pub fn script(painter: &Painter, r: Rect, color: Color32, w: f32) {
    let s = Stroke::new(w, color);
    line(painter, &r, &[(0.3, 0.1), (0.9, 0.1), (0.75, 0.5), (0.9, 0.9), (0.3, 0.9), (0.15, 0.5), (0.3, 0.1)], s);
    for &y in &[0.32, 0.5, 0.68] {
        painter.line_segment([p(&r, 0.35, y), p(&r, 0.7, y)], s);
    }
}

pub fn envelope(painter: &Painter, r: Rect, color: Color32, w: f32, filled: bool) {
    let s = Stroke::new(w, color);
    let rr = Rect::from_min_max(p(&r, 0.08, 0.2), p(&r, 0.92, 0.8));
    if filled {
        painter.rect_filled(rr, 1.0, color);
        let inv = Color32::WHITE;
        let si = Stroke::new(w, inv);
        painter.add(Shape::line(vec![rr.left_top(), rr.center(), rr.right_top()], PathStroke::from(si)));
    } else {
        painter.rect_stroke(rr, 1.0, s, StrokeKind::Middle);
        painter.add(Shape::line(vec![rr.left_top(), rr.center(), rr.right_top()], PathStroke::from(s)));
    }
}

pub fn table(painter: &Painter, r: Rect, color: Color32, w: f32) {
    let s = Stroke::new(w, color);
    let rr = Rect::from_min_max(p(&r, 0.1, 0.2), p(&r, 0.9, 0.8));
    painter.rect_stroke(rr, 0.0, s, StrokeKind::Middle);
    painter.rect_filled(Rect::from_min_max(rr.min, Pos2::new(rr.max.x, rr.min.y + rr.height() * 0.3)), 0.0, color);
    painter.line_segment([Pos2::new(rr.min.x, rr.center().y + rr.height() * 0.15), Pos2::new(rr.max.x, rr.center().y + rr.height() * 0.15)], s);
    painter.line_segment([Pos2::new(rr.min.x + rr.width() * 0.3, rr.min.y), Pos2::new(rr.min.x + rr.width() * 0.3, rr.max.y)], s);
}

pub fn hand(painter: &Painter, r: Rect, color: Color32, w: f32) {
    let s = Stroke::new(w, color);
    line(painter, &r, &[(0.15, 0.55), (0.15, 0.85), (0.8, 0.85), (0.9, 0.65), (0.9, 0.45), (0.55, 0.45)], s);
    for &y in &[0.2, 0.33, 0.46] {
        line(painter, &r, &[(0.15, y + 0.1), (0.25, y), (0.8, y), (0.8, y + 0.1)], s);
    }
}

pub fn receive(painter: &Painter, r: Rect, color: Color32, w: f32) {
    envelope(painter, r, color, w, false);
}

pub fn clock(painter: &Painter, c: Pos2, radius: f32, color: Color32, w: f32) {
    let s = Stroke::new(w, color);
    painter.circle_stroke(c, radius, s);
    for i in 0..12 {
        let a = TAU * i as f32 / 12.0;
        let l = if i % 3 == 0 { 0.25 } else { 0.12 };
        painter.line_segment([c + Vec2::angled(a) * radius * (1.0 - l), c + Vec2::angled(a) * radius * 0.92], s);
    }
    painter.line_segment([c, c + Vec2::angled(-TAU / 4.0) * radius * 0.55], s);
    painter.line_segment([c, c + Vec2::angled(TAU / 12.0) * radius * 0.7], s);
}

pub fn lightning(painter: &Painter, r: Rect, color: Color32, w: f32, filled: bool) {
    let pts = [(0.3, 0.1), (0.55, 0.1), (0.42, 0.45), (0.7, 0.45), (0.38, 0.92), (0.45, 0.58), (0.22, 0.58)];
    closed(painter, &r, &pts, if filled { color } else { Color32::TRANSPARENT }, Stroke::new(w, color));
}

pub fn triangle(painter: &Painter, r: Rect, color: Color32, w: f32, filled: bool) {
    closed(painter, &r, &[(0.5, 0.1), (0.92, 0.85), (0.08, 0.85)], if filled { color } else { Color32::TRANSPARENT }, Stroke::new(w, color));
}

pub fn chevron_up(painter: &Painter, r: Rect, color: Color32, w: f32, filled: bool) {
    closed(painter, &r, &[(0.5, 0.1), (0.85, 0.8), (0.5, 0.6), (0.15, 0.8)], if filled { color } else { Color32::TRANSPARENT }, Stroke::new(w, color));
}

pub fn conditional(painter: &Painter, r: Rect, color: Color32, w: f32) {
    let s = Stroke::new(w, color);
    let rr = Rect::from_min_max(p(&r, 0.2, 0.12), p(&r, 0.8, 0.88));
    painter.rect_stroke(rr, 0.0, s, StrokeKind::Middle);
    for i in 0..4 {
        let y = rr.min.y + rr.height() * (0.2 + 0.2 * i as f32);
        painter.line_segment([Pos2::new(rr.min.x + 3.0 * w, y), Pos2::new(rr.max.x - 3.0 * w, y)], s);
    }
}

pub fn link_arrow(painter: &Painter, r: Rect, color: Color32, w: f32, filled: bool) {
    closed(painter, &r, &[(0.1, 0.35), (0.55, 0.35), (0.55, 0.15), (0.92, 0.5), (0.55, 0.85), (0.55, 0.65), (0.1, 0.65)], if filled { color } else { Color32::TRANSPARENT }, Stroke::new(w, color));
}

pub fn compensation(painter: &Painter, r: Rect, color: Color32, w: f32, filled: bool) {
    let f = if filled { color } else { Color32::TRANSPARENT };
    closed(painter, &r, &[(0.5, 0.15), (0.5, 0.85), (0.1, 0.5)], f, Stroke::new(w, color));
    closed(painter, &r, &[(0.9, 0.15), (0.9, 0.85), (0.5, 0.5)], f, Stroke::new(w, color));
}

pub fn terminate(painter: &Painter, c: Pos2, radius: f32, color: Color32) {
    painter.circle_filled(c, radius, color);
}

pub fn cancel_x(painter: &Painter, r: Rect, color: Color32, w: f32, filled: bool) {
    let s = Stroke::new(w, color);
    if filled {
        let pts = [(0.3, 0.1), (0.5, 0.3), (0.7, 0.1), (0.9, 0.3), (0.7, 0.5), (0.9, 0.7), (0.7, 0.9), (0.5, 0.7), (0.3, 0.9), (0.1, 0.7), (0.3, 0.5), (0.1, 0.3)];
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| p(&r, x, y)).collect();
        painter.add(PathShape::convex_polygon(v, color, PathStroke::from(s)));
    } else {
        line(painter, &r, &[(0.15, 0.15), (0.85, 0.85)], Stroke::new(w * 1.6, color));
        line(painter, &r, &[(0.85, 0.15), (0.15, 0.85)], Stroke::new(w * 1.6, color));
    }
}

/// Gateway markers inside a diamond of rect `r`.
pub fn gw_exclusive(painter: &Painter, r: Rect, color: Color32, w: f32) {
    line(painter, &r, &[(0.32, 0.32), (0.68, 0.68)], Stroke::new(w, color));
    line(painter, &r, &[(0.68, 0.32), (0.32, 0.68)], Stroke::new(w, color));
}
pub fn gw_parallel(painter: &Painter, r: Rect, color: Color32, w: f32) {
    line(painter, &r, &[(0.5, 0.22), (0.5, 0.78)], Stroke::new(w, color));
    line(painter, &r, &[(0.22, 0.5), (0.78, 0.5)], Stroke::new(w, color));
}
pub fn gw_inclusive(painter: &Painter, r: Rect, color: Color32, w: f32) {
    painter.circle_stroke(r.center(), r.width() * 0.22, Stroke::new(w, color));
}
pub fn gw_event_based(painter: &Painter, r: Rect, color: Color32, w: f32, instantiating: bool) {
    let c = r.center();
    let s = Stroke::new(w * 0.8, color);
    painter.circle_stroke(c, r.width() * 0.26, s);
    if !instantiating {
        painter.circle_stroke(c, r.width() * 0.21, s);
    }
    let pts: Vec<Pos2> = (0..5).map(|i| c + Vec2::angled(-TAU / 4.0 + TAU * i as f32 / 5.0) * r.width() * 0.14).collect();
    painter.add(Shape::closed_line(pts, PathStroke::from(s)));
}
pub fn gw_complex(painter: &Painter, r: Rect, color: Color32, w: f32) {
    let s = Stroke::new(w, color);
    line(painter, &r, &[(0.5, 0.22), (0.5, 0.78)], s);
    line(painter, &r, &[(0.22, 0.5), (0.78, 0.5)], s);
    line(painter, &r, &[(0.3, 0.3), (0.7, 0.7)], s);
    line(painter, &r, &[(0.7, 0.3), (0.3, 0.7)], s);
}

/// Activity markers (bottom centre): loop, parallel/sequential multi-instance, compensation, ad-hoc, collapsed.
pub fn marker_loop(painter: &Painter, r: Rect, color: Color32, w: f32) {
    let c = r.center();
    let rad = r.width() * 0.38;
    let pts: Vec<Pos2> = (0..20).map(|i| c + Vec2::angled(TAU * 0.1 + TAU * 0.8 * i as f32 / 19.0) * rad).collect();
    painter.add(Shape::line(pts, PathStroke::from(Stroke::new(w, color))));
    let tip = c + Vec2::angled(TAU * 0.1) * rad;
    painter.add(Shape::convex_polygon(vec![tip + Vec2::new(-w * 2.5, -w * 0.5), tip + Vec2::new(w * 1.0, -w * 2.5), tip + Vec2::new(w * 1.5, w * 1.5)], color, PathStroke::NONE));
}
pub fn marker_mi_parallel(painter: &Painter, r: Rect, color: Color32, w: f32) {
    for i in 0..3 {
        let x = 0.3 + 0.2 * i as f32;
        line(painter, &r, &[(x, 0.15), (x, 0.85)], Stroke::new(w * 1.2, color));
    }
}
pub fn marker_mi_sequential(painter: &Painter, r: Rect, color: Color32, w: f32) {
    for i in 0..3 {
        let y = 0.3 + 0.2 * i as f32;
        line(painter, &r, &[(0.15, y), (0.85, y)], Stroke::new(w * 1.2, color));
    }
}
pub fn marker_adhoc(painter: &Painter, r: Rect, color: Color32, w: f32) {
    let s = PathStroke::from(Stroke::new(w, color));
    let a = p(&r, 0.1, 0.55);
    let b = p(&r, 0.9, 0.45);
    painter.add(CubicBezierShape::from_points_stroke([a, p(&r, 0.3, 0.1), p(&r, 0.7, 0.9), b], false, Color32::TRANSPARENT, s));
}
pub fn marker_collapsed(painter: &Painter, r: Rect, color: Color32, w: f32) {
    painter.rect_stroke(r, 0.0, Stroke::new(w, color), StrokeKind::Middle);
    line(painter, &r, &[(0.5, 0.2), (0.5, 0.8)], Stroke::new(w, color));
    line(painter, &r, &[(0.2, 0.5), (0.8, 0.5)], Stroke::new(w, color));
}
pub fn marker_compensation(painter: &Painter, r: Rect, color: Color32, w: f32) {
    compensation(painter, r, color, w, false);
}
