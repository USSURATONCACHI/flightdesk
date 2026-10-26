//! Windowing-system specific capabilities, detected from the window's raw handles.
//!
//! Today this is one capability: moving the pointer ("warping"), used for unlimited drags.
//! X11, Windows and macOS do it through winit; Wayland needs the `wp_pointer_warp_v1`
//! protocol, spoken directly to the compositor on the window's own connection (KWin ≥ 6.4).
//! Where neither is available the feature reports itself as unsupported and stays off.

pub mod files;
#[cfg(target_os = "linux")]
pub mod wayland;
#[cfg(target_os = "linux")]
pub mod x11;

#[cfg(target_os = "linux")]
use raw_window_handle::RawWindowHandle;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle};
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
    #[cfg(target_os = "linux")]
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
    #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
    let mut st = State {
        platform,
        #[cfg(target_os = "linux")]
        wayland: None,
        #[cfg(target_os = "linux")]
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

/// Keep a not-yet-mapped window from taking focus when it appears. Windows and macOS handle
/// this through `ViewportBuilder::with_active(false)`; on X11 winit ignores that, so the window
/// gets the EWMH "do not focus" hint here. Wayland has no such request: callers that need it
/// run through Xwayland (see `no_focus_prefers_x11`). Call before the first frame.
pub fn suppress_initial_focus(window: &dyn HasWindowHandle) {
    #[cfg(target_os = "linux")]
    {
        let xid = match window.window_handle().map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Xlib(w)) => Some(w.window as u32),
            Ok(RawWindowHandle::Xcb(w)) => Some(w.window.get()),
            _ => None,
        };
        if let Some(xid) = xid {
            match x11::suppress_initial_focus(xid) {
                Ok(()) => log::info!("x11: window opens without focus (_NET_WM_USER_TIME=0)"),
                Err(e) => log::warn!("x11: could not set the no-focus hint: {e}"),
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = window;
}

/// Whether a no-focus start should use X11 (Xwayland) instead of Wayland: Wayland offers no way
/// for a client to decline focus, so with both displays available X11 is the only reliable path.
pub fn no_focus_prefers_x11(wayland_display: Option<&str>, x_display: Option<&str>) -> bool {
    cfg!(target_os = "linux") && wayland_display.is_some_and(|s| !s.is_empty()) && x_display.is_some_and(|s| !s.is_empty())
}

fn last_occluded_skip() -> &'static Mutex<Option<std::time::Instant>> {
    static S: OnceLock<Mutex<Option<std::time::Instant>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(None))
}

/// Record that the GPU refused a frame because the window is covered (wgpu `Occluded`).
/// Such frames are skipped, and any screenshot waiting on them is never delivered.
pub fn note_occluded_frame() {
    *last_occluded_skip().lock().unwrap() = Some(std::time::Instant::now());
}

/// Why a screenshot requested at `since` has not arrived: used for errors and logs.
pub fn screenshot_stall_reason(since: std::time::Instant) -> String {
    let covered = last_occluded_skip().lock().unwrap().is_some_and(|t| t >= since);
    if covered {
        "the window is covered by another window, so the GPU skips every frame and there is nothing \
         to capture; uncover it, or start Flightdeck with --on-top / FLIGHTDECK_ON_TOP=1"
            .into()
    } else {
        "no frame was painted (is the window minimized or on a hidden workspace?)".into()
    }
}

/// Drive a pending screenshot to completion. The image is read back from the GPU
/// asynchronously and only arrives once a later frame polls the device, and an idle window
/// that nobody interacts with (unfocused, no pointer over it) paints no later frame. So this
/// keeps requesting repaints until `done` is set, and calls `on_stall` with an explanation if
/// that has not happened within `timeout`.
pub fn screenshot_wait(ctx: egui::Context, done: std::sync::Arc<std::sync::atomic::AtomicBool>, timeout: std::time::Duration, on_stall: impl FnOnce(String) + Send + 'static) {
    let since = std::time::Instant::now();
    std::thread::spawn(move || {
        while since.elapsed() < timeout {
            if done.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            ctx.request_repaint();
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if !done.load(std::sync::atomic::Ordering::SeqCst) {
            on_stall(format!("screenshot timed out after {} s: {}", timeout.as_secs(), screenshot_stall_reason(since)));
        }
    });
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

    #[test]
    fn no_focus_uses_x11_only_when_both_displays_exist() {
        let linux = cfg!(target_os = "linux");
        assert_eq!(no_focus_prefers_x11(Some("wayland-0"), Some(":0")), linux);
        assert!(!no_focus_prefers_x11(Some("wayland-0"), None), "no Xwayland: stay on Wayland");
        assert!(!no_focus_prefers_x11(Some("wayland-0"), Some("")));
        assert!(!no_focus_prefers_x11(None, Some(":0")), "plain X11 session needs no switch");
        assert!(!no_focus_prefers_x11(Some(""), Some(":0")));
    }

    fn wait(done: bool, timeout_ms: u64) -> Option<String> {
        use std::sync::{atomic::AtomicBool, mpsc, Arc};
        let (tx, rx) = mpsc::channel();
        let flag = Arc::new(AtomicBool::new(done));
        screenshot_wait(egui::Context::default(), flag, std::time::Duration::from_millis(timeout_ms), move |why| {
            let _ = tx.send(why);
        });
        rx.recv_timeout(std::time::Duration::from_millis(timeout_ms + 2000)).ok()
    }

    /// A pending screenshot must end one way or the other: never a silent, endless wait.
    #[test]
    fn screenshot_wait_reports_stalls_and_stays_quiet_when_done() {
        assert_eq!(wait(true, 100), None, "a delivered screenshot reports nothing");
        let why = wait(false, 100).expect("a stalled screenshot is reported");
        // The reason itself depends on global state another test may touch concurrently.
        assert!(why.starts_with("screenshot timed out after"), "{why}");
    }

    #[test]
    fn stall_reason_names_a_covered_window() {
        let since = std::time::Instant::now();
        note_occluded_frame();
        assert!(screenshot_stall_reason(since).contains("covered"));
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(!screenshot_stall_reason(std::time::Instant::now()).contains("covered"), "older skips do not count");
    }
}
