//! Hit testing in world space.

use bpmn::{ElementKind, Model, Ref};
use egui::Pos2;

/// Topmost element or flow under a world point. `tolerance` is in world units (≈ 6 px / zoom).
pub fn hit_test(m: &Model, p: Pos2, tolerance: f32) -> Option<Ref> {
    let wp = bpmn::Pos { x: p.x, y: p.y };
    // Flow nodes and artifacts, topmost first (reverse draw order).
    for &i in m.draw_order.iter().rev() {
        let e = &m.elements[i];
        if matches!(e.kind, ElementKind::Participant | ElementKind::Lane | ElementKind::Group) || (e.kind.is_container() && e.is_expanded) {
            continue;
        }
        if !e.bounds.contains(wp) {
            continue;
        }
        if e.kind.is_event() {
            let c = e.bounds.center();
            let r = e.bounds.w / 2.0 + tolerance;
            if (wp.x - c.x).powi(2) + (wp.y - c.y).powi(2) <= r * r {
                return Some(Ref::Element(i));
            }
            continue;
        }
        if e.kind.is_gateway() {
            let c = e.bounds.center();
            let hw = e.bounds.w / 2.0 + tolerance;
            let hh = e.bounds.h / 2.0 + tolerance;
            if ((wp.x - c.x).abs() / hw + (wp.y - c.y).abs() / hh) <= 1.0 {
                return Some(Ref::Element(i));
            }
            continue;
        }
        return Some(Ref::Element(i));
    }
    // Flows.
    let mut best: Option<(f32, usize)> = None;
    for (fi, f) in m.flows.iter().enumerate() {
        if f.waypoints.len() < 2 || !f.bounds.expand(tolerance).contains(wp) {
            continue;
        }
        for w in f.waypoints.windows(2) {
            let d = dist_point_segment(wp, w[0], w[1]);
            if d <= tolerance && best.map(|(bd, _)| d < bd).unwrap_or(true) {
                best = Some((d, fi));
            }
        }
    }
    if let Some((_, fi)) = best {
        return Some(Ref::Flow(fi));
    }
    // Expanded containers, then lanes, then pools (innermost first = reverse draw order).
    for &i in m.draw_order.iter().rev() {
        let e = &m.elements[i];
        let container = (e.kind.is_container() && e.is_expanded) || matches!(e.kind, ElementKind::Lane | ElementKind::Participant | ElementKind::Group);
        if container && e.bounds.contains(wp) {
            // Only count the header strip for lanes/pools so empty space pans.
            if e.swimlane_header().is_some_and(|h| !h.contains(wp)) {
                continue;
            }
            if e.kind.is_container() {
                // header strip of subprocess (top 24 units) only
                if wp.y > e.bounds.y + 24.0 {
                    continue;
                }
            }
            return Some(Ref::Element(i));
        }
    }
    None
}

pub fn dist_point_segment(p: bpmn::Pos, a: bpmn::Pos, b: bpmn::Pos) -> f32 {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let l2 = abx * abx + aby * aby;
    let t = if l2 <= 0.0 { 0.0 } else { (((p.x - a.x) * abx + (p.y - a.y) * aby) / l2).clamp(0.0, 1.0) };
    let cx = a.x + abx * t;
    let cy = a.y + aby * t;
    ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt()
}
