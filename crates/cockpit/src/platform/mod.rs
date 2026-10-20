//! Windowing-system specific capabilities, detected from the window's raw handles.
//!
//! Today this is one capability: moving the pointer ("warping"), used for unlimited drags.
//! X11, Windows and macOS do it through winit; Wayland needs the `wp_pointer_warp_v1`
//! protocol, spoken directly to the compositor on the window's own connection (KWin ≥ 6.4).
//! Where neither is available the feature reports itself as unsupported and stays off.

#[cfg(target_os = "linux")]
pub mod wayland;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Wayland,
    X11,
    Windows,
    MacOs,
    Other,
}

impl Platform {
    pub fn from_display_handle(h: &RawDisplayHandle) -> Self {
        match h {
            RawDisplayHandle::Wayland(_) => Platform::Wayland,
            RawDisplayHandle::Xlib(_) | RawDisplayHandle::Xcb(_) => Platform::X11,
            RawDisplayHandle::Windows(_) => Platform::Windows,
            RawDisplayHandle::AppKit(_) => Platform::MacOs,
            _ => Platform::Other,
        }
    }
    /// Whether winit itself can move the pointer on this platform.
    pub fn winit_can_warp(self) -> bool {
        matches!(self, Platform::X11 | Platform::Windows | Platform::MacOs)
    }
    pub fn label(self) -> &'static str {
        match self {
            Platform::Wayland => "Wayland",
            Platform::X11 => "X11",
            Platform::Windows => "Windows",
            Platform::MacOs => "macOS",
            Platform::Other => "unknown platform",
        }
    }
}

struct State {
    platform: Platform,
    #[cfg(target_os = "linux")]
    wayland: Option<wayland::WaylandWarp>,
    wayland_error: Option<String>,
}

fn state() -> &'static Mutex<Option<State>> {
    static S: OnceLock<Mutex<Option<State>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(None))
}

/// Detect the platform from the window and, on Wayland, bind the pointer-warp protocol.
/// Call once after the window exists (eframe's `CreationContext` implements both traits).
pub fn init(window: &dyn HasWindowHandle, display: &dyn HasDisplayHandle) {
    let dh = display.display_handle().map(|h| h.as_raw());
    let wh = window.window_handle().map(|h| h.as_raw());
    let platform = dh.as_ref().map(Platform::from_display_handle).unwrap_or(Platform::Other);
    let mut st = State {
        platform,
        #[cfg(target_os = "linux")]
        wayland: None,
        wayland_error: None,
    };
    #[cfg(target_os = "linux")]
    if let (Ok(RawDisplayHandle::Wayland(d)), Ok(RawWindowHandle::Wayland(w))) = (&dh, &wh) {
        // Safety: both pointers come from winit's live window and outlive this process's windows.
        match unsafe { wayland::WaylandWarp::new(d.display.as_ptr(), w.surface.as_ptr()) } {
            Ok(w) => {
                log::info!("wayland: pointer warp {}", if w.supported() { "available (wp_pointer_warp_v1)" } else { "not offered by this compositor" });
                st.wayland = Some(w);
            }
            Err(e) => {
                log::warn!("wayland: pointer warp unavailable: {e}");
                st.wayland_error = Some(e);
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = wh;
    log::info!("platform: {}", st.platform.label());
    *state().lock().unwrap() = Some(st);
}

pub fn platform() -> Platform {
    state().lock().unwrap().as_ref().map(|s| s.platform).unwrap_or(Platform::Other)
}

/// Can the pointer be moved programmatically on this platform right now?
pub fn warp_supported() -> bool {
    let guard = state().lock().unwrap();
    let Some(s) = guard.as_ref() else { return false };
    match s.platform {
        #[cfg(target_os = "linux")]
        Platform::Wayland => s.wayland.as_ref().map(|w| w.supported()).unwrap_or(false),
        p => p.winit_can_warp(),
    }
}

/// Move the pointer to `pos` (window points). Returns false when unsupported.
pub fn warp_pointer(ctx: &egui::Context, pos: egui::Pos2) -> bool {
    let mut guard = state().lock().unwrap();
    let Some(s) = guard.as_mut() else { return false };
    match s.platform {
        #[cfg(target_os = "linux")]
        Platform::Wayland => s.wayland.as_mut().map(|w| w.warp(pos.x as f64, pos.y as f64)).unwrap_or(false),
        p if p.winit_can_warp() => {
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorPosition(pos));
            true
        }
        _ => false,
    }
}

/// Per-frame housekeeping (Wayland: dispatch our protocol events).
pub fn pump() {
    #[cfg(target_os = "linux")]
    if let Some(s) = state().lock().unwrap().as_mut() {
        if let Some(w) = s.wayland.as_mut() {
            w.pump();
        }
    }
}

/// One-line status for the settings page.
pub fn describe() -> String {
    let guard = state().lock().unwrap();
    let Some(s) = guard.as_ref() else { return "not initialised".into() };
    match s.platform {
        #[cfg(target_os = "linux")]
        Platform::Wayland => match (&s.wayland, &s.wayland_error) {
            (Some(w), _) if w.supported() => "Wayland · pointer warp via wp_pointer_warp_v1".into(),
            (Some(_), _) => "Wayland · this compositor does not offer wp_pointer_warp_v1, unlimited drag is unavailable".into(),
            (None, Some(e)) => format!("Wayland · unavailable ({e})"),
            (None, None) => "Wayland · unavailable".into(),
        },
        p if p.winit_can_warp() => format!("{} · pointer warp via the window system", p.label()),
        p => format!("{} · pointer warp unsupported", p.label()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn platform_from_handles() {
        assert_eq!(Platform::from_display_handle(&RawDisplayHandle::Xlib(raw_window_handle::XlibDisplayHandle::new(None, 0))), Platform::X11);
        assert_eq!(Platform::from_display_handle(&RawDisplayHandle::Windows(raw_window_handle::WindowsDisplayHandle::new())), Platform::Windows);
        assert!(Platform::X11.winit_can_warp() && !Platform::Other.winit_can_warp() && !Platform::Wayland.winit_can_warp());
        // before init nothing is supported and nothing panics
        assert!(!warp_supported());
        assert!(!warp_pointer(&egui::Context::default(), egui::Pos2::ZERO));
        pump();
    }
}
