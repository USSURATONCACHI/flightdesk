//! Camera model: a target the user manipulates and a smoothed current camera that is drawn.
//! World units are BPMN DI units; `zoom` = screen pixels per world unit.

use egui::{Pos2, Rect, Vec2};

pub const MIN_ZOOM: f32 = 0.005;
pub const MAX_ZOOM: f32 = 200.0;
/// Smoothing rate: 1 - exp(-dt * RATE). 20 ≈ settles in ~150 ms.
pub const RATE: f32 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraPose {
    /// World point at the centre of the viewport.
    pub center: Pos2,
    pub zoom: f32,
}

impl Default for CameraPose {
    fn default() -> Self {
        Self { center: Pos2::ZERO, zoom: 1.0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Camera {
    pub target: CameraPose,
    pub current: CameraPose,
    /// Residual pan velocity (screen px/s) after a flick; decays exponentially.
    pub inertia: Vec2,
    pub initialized: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self { target: CameraPose::default(), current: CameraPose::default(), inertia: Vec2::ZERO, initialized: false }
    }
}

impl Camera {
    pub fn world_to_screen(&self, viewport: Rect, p: Pos2) -> Pos2 {
        viewport.center() + (p - self.current.center) * self.current.zoom
    }
    pub fn screen_to_world(&self, viewport: Rect, p: Pos2) -> Pos2 {
        self.current.center + (p - viewport.center()) / self.current.zoom
    }
    /// Screen → world using the *target* pose; input must anchor on the target so successive
    /// wheel ticks compose correctly while the animation is still running.
    pub fn screen_to_world_target(&self, viewport: Rect, p: Pos2) -> Pos2 {
        self.target.center + (p - viewport.center()) / self.target.zoom
    }
    pub fn world_rect_to_screen(&self, viewport: Rect, r: Rect) -> Rect {
        Rect::from_min_max(self.world_to_screen(viewport, r.min), self.world_to_screen(viewport, r.max))
    }
    pub fn visible_world_rect(&self, viewport: Rect) -> Rect {
        Rect::from_min_max(self.screen_to_world(viewport, viewport.min), self.screen_to_world(viewport, viewport.max))
    }
    pub fn zoom(&self) -> f32 {
        self.current.zoom
    }

    /// Multiply the target zoom by `factor`, keeping the world point under `anchor` (screen) fixed.
    pub fn zoom_by(&mut self, viewport: Rect, factor: f32, anchor: Pos2) {
        self.inertia = Vec2::ZERO;
        let before = self.screen_to_world_target(viewport, anchor);
        self.target.zoom = (self.target.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let after = self.screen_to_world_target(viewport, anchor);
        self.target.center += before - after;
    }
    pub fn set_zoom(&mut self, viewport: Rect, zoom: f32, anchor: Pos2) {
        let f = zoom.clamp(MIN_ZOOM, MAX_ZOOM) / self.target.zoom;
        self.zoom_by(viewport, f, anchor);
    }
    /// Pan by a screen-space delta (positive = content moves right/down).
    pub fn pan_screen(&mut self, delta: Vec2) {
        self.target.center -= delta / self.target.zoom;
    }
    /// Immediate pan (dragging): move both target and current so the content sticks to the pointer.
    pub fn drag_screen(&mut self, delta: Vec2) {
        self.inertia = Vec2::ZERO;
        let d = delta / self.target.zoom;
        self.target.center -= d;
        self.current.center -= d;
    }
    pub fn fit(&mut self, viewport: Rect, world: Rect, padding_px: f32, max_zoom: f32) {
        self.inertia = Vec2::ZERO;
        if world.width() <= 0.0 || world.height() <= 0.0 || viewport.width() <= 2.0 * padding_px {
            return;
        }
        let zx = (viewport.width() - 2.0 * padding_px) / world.width();
        let zy = (viewport.height() - 2.0 * padding_px) / world.height();
        self.target.zoom = zx.min(zy).min(max_zoom).clamp(MIN_ZOOM, MAX_ZOOM);
        self.target.center = world.center();
    }
    /// Jump without animation (first frame).
    pub fn snap(&mut self) {
        self.current = self.target;
        self.inertia = Vec2::ZERO;
        self.initialized = true;
    }
    /// Advance the animation. Returns true if still moving (caller should request a repaint).
    pub fn tick(&mut self, dt: f32, inertia_enabled: bool) -> bool {
        let dt = dt.clamp(0.0, 0.05);
        if !self.initialized {
            self.snap();
        }
        if inertia_enabled && self.inertia.length() > 1.0 {
            self.target.center -= self.inertia * dt / self.target.zoom;
            self.inertia *= (-dt * 6.0).exp();
            if self.inertia.length() < 20.0 {
                self.inertia = Vec2::ZERO;
            }
        } else {
            self.inertia = Vec2::ZERO;
        }
        let a = 1.0 - (-dt * RATE).exp();
        let lz_cur = self.current.zoom.ln();
        let lz_tgt = self.target.zoom.ln();
        let lz = lz_cur + (lz_tgt - lz_cur) * a;
        self.current.zoom = lz.exp();
        self.current.center = self.current.center + (self.target.center - self.current.center) * a;
        let moving = (lz_tgt - lz).abs() > 1e-4 || (self.target.center - self.current.center).length() * self.current.zoom > 0.1 || self.inertia != Vec2::ZERO;
        if !moving {
            self.current = self.target;
        }
        moving
    }
    pub fn is_animating(&self) -> bool {
        self.current != self.target || self.inertia != Vec2::ZERO
    }
    /// Whether `world` is fully visible with some margin.
    pub fn contains_world(&self, viewport: Rect, world: Rect) -> bool {
        self.visible_world_rect(viewport).contains_rect(world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn vp() -> Rect {
        Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(800.0, 600.0))
    }
    #[test]
    fn zoom_keeps_anchor_fixed() {
        let mut c = Camera { target: CameraPose { center: Pos2::new(500.0, 300.0), zoom: 0.5 }, ..Default::default() };
        c.snap();
        let anchor = Pos2::new(300.0, 200.0);
        let before = c.screen_to_world_target(vp(), anchor);
        c.zoom_by(vp(), 2.0, anchor);
        let after = c.screen_to_world_target(vp(), anchor);
        assert!((before - after).length() < 1e-3);
        assert!((c.target.zoom - 1.0).abs() < 1e-6);
    }
    #[test]
    fn converges_and_stops() {
        let mut c = Camera::default();
        c.snap();
        c.target = CameraPose { center: Pos2::new(1000.0, 0.0), zoom: 4.0 };
        let mut frames = 0;
        while c.tick(1.0 / 60.0, true) {
            frames += 1;
            assert!(frames < 200, "did not converge");
        }
        assert_eq!(c.current, c.target);
        assert!(frames > 5, "should animate over several frames, got {frames}");
    }
    #[test]
    fn fit_contains_world() {
        let mut c = Camera::default();
        let world = Rect::from_min_size(Pos2::new(120.0, 60.0), Vec2::new(1260.0, 520.0));
        c.fit(vp(), world, 20.0, 10.0);
        c.snap();
        assert!(c.contains_world(vp(), world));
        let s = c.world_rect_to_screen(vp(), world);
        assert!(s.width() <= 800.0 && s.height() <= 600.0);
    }
    #[test]
    fn clamp_zoom() {
        let mut c = Camera::default();
        c.snap();
        c.zoom_by(vp(), 1e9, vp().center());
        assert_eq!(c.target.zoom, MAX_ZOOM);
        c.zoom_by(vp(), 1e-12, vp().center());
        assert_eq!(c.target.zoom, MIN_ZOOM);
    }
}
