//! Unlimited dragging ("torus" pointer): when a drag reaches the edge of its area the pointer is
//! teleported to the opposite edge and the drag continues (Okular-style). Moving the pointer is a
//! platform capability (see `crate::platform`); where it is unavailable drags simply stop at the
//! edge.

use egui::{Context, Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WrapMode {
    /// Wrap whenever the platform can move the pointer.
    #[default]
    Auto,
    /// Plain drags that stop at the edge.
    Off,
}

/// Where to teleport a pointer that left `rect`, and the displacement that teleport adds to the
/// next pointer delta (so it can be subtracted). `axes` selects horizontal / vertical wrapping.
pub fn plan_warp(pointer: Pos2, rect: Rect, axes: (bool, bool)) -> Option<(Pos2, Vec2)> {
    const INSET: f32 = 2.0;
    let mut target = pointer;
    if axes.0 {
        if pointer.x <= rect.min.x {
            target.x = rect.max.x - INSET;
        } else if pointer.x >= rect.max.x - 1.0 {
            target.x = rect.min.x + INSET;
        }
    }
    if axes.1 {
        if pointer.y <= rect.min.y {
            target.y = rect.max.y - INSET;
        } else if pointer.y >= rect.max.y - 1.0 {
            target.y = rect.min.y + INSET;
        }
    }
    if target == pointer {
        None
    } else {
        Some((target, target - pointer))
    }
}

/// Remove a teleport's displacement from a pointer delta once it shows up: the delta jumps by
/// about the wrap distance along the warped axis.
pub fn correct_delta(raw: Vec2, pending: Vec2) -> Option<Vec2> {
    let jumped = |d: f32, w: f32| w != 0.0 && d.signum() == w.signum() && d.abs() >= w.abs() * 0.5;
    if jumped(raw.x, pending.x) || jumped(raw.y, pending.y) {
        Some(raw - pending)
    } else {
        None
    }
}

/// Per-widget wrap state. Call `begin` on drag start, `delta` every dragged frame, `end` on stop.
#[derive(Debug, Default)]
pub struct PointerWrap {
    active: bool,
    pending: Option<(Vec2, u32)>,
}

impl PointerWrap {
    pub fn begin(&mut self, mode: WrapMode) {
        self.active = mode == WrapMode::Auto && crate::platform::warp_supported();
        self.pending = None;
    }

    pub fn end(&mut self) {
        self.active = false;
        self.pending = None;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// The movement to apply this frame, given egui's drag delta, the pointer position and the
    /// area the pointer should stay in. Issues the teleport when the pointer leaves the area.
    pub fn delta(&mut self, ctx: &Context, raw: Vec2, pointer: Option<Pos2>, rect: Rect, axes: (bool, bool)) -> Vec2 {
        if !self.active {
            return raw;
        }
        let mut d = raw;
        if let Some((w, age)) = self.pending {
            match correct_delta(raw, w) {
                Some(fixed) => {
                    d = fixed;
                    self.pending = None;
                }
                None if age > 12 => self.pending = None, // the warp never arrived
                None => self.pending = Some((w, age + 1)),
            }
        }
        if self.pending.is_none() {
            if let Some(p) = pointer {
                if let Some((target, w)) = plan_warp(p, rect, axes) {
                    if crate::platform::warp_pointer(ctx, target) {
                        self.pending = Some((w, 0));
                    }
                }
            }
        }
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn r() -> Rect {
        Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(800.0, 600.0))
    }
    #[test]
    fn warps_to_the_opposite_edge_only_on_enabled_axes() {
        let (t, w) = plan_warp(Pos2::new(899.5, 300.0), r(), (true, true)).unwrap();
        assert_eq!(t, Pos2::new(102.0, 300.0));
        assert_eq!(w, Vec2::new(-797.5, 0.0));
        let (t, _) = plan_warp(Pos2::new(100.0, 50.0), r(), (true, true)).unwrap();
        assert_eq!(t, Pos2::new(898.0, 648.0), "both axes can wrap at once");
        assert!(plan_warp(Pos2::new(100.0, 300.0), r(), (false, true)).is_none(), "x wrapping disabled");
        assert!(plan_warp(Pos2::new(500.0, 300.0), r(), (true, true)).is_none());
    }
    #[test]
    fn teleport_displacement_is_removed_from_the_next_jump_only() {
        let w = Vec2::new(-797.5, 0.0);
        assert_eq!(correct_delta(Vec2::new(-790.0, 3.0), w), Some(Vec2::new(7.5, 3.0)));
        assert_eq!(correct_delta(Vec2::new(-12.0, 0.0), w), None, "ordinary movement is left alone");
        assert_eq!(correct_delta(Vec2::new(790.0, 0.0), w), None, "a jump the other way is not ours");
    }
    #[test]
    fn inactive_without_platform_support_passes_deltas_through() {
        let mut pw = PointerWrap::default();
        pw.begin(WrapMode::Auto); // no platform initialised in tests → inactive
        assert!(!pw.is_active());
        let ctx = Context::default();
        assert_eq!(pw.delta(&ctx, Vec2::new(5.0, -3.0), Some(Pos2::new(100.0, 300.0)), r(), (true, true)), Vec2::new(5.0, -3.0));
        pw.begin(WrapMode::Off);
        assert!(!pw.is_active());
    }
}
