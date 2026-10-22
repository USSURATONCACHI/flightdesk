//! X11: open the window without taking keyboard focus.
//!
//! winit ignores `with_active(false)` on X11, so we use the EWMH convention instead: a
//! `_NET_WM_USER_TIME` of 0 tells the window manager the window must not be focused when it is
//! first mapped (KWin, Mutter, Openbox and others honour it). The property has to be set while
//! the window is still unmapped; eframe creates it hidden and maps it after the first frame,
//! so the app-creator callback is the right moment.

use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, PropMode};
use x11rb::wrapper::ConnectionExt as _;

/// Set `_NET_WM_USER_TIME = 0` on `window`. Uses its own connection to `$DISPLAY` and waits for
/// the server to process the request, so the property is in place before winit maps the window.
pub fn suppress_initial_focus(window: u32) -> Result<(), String> {
    let (conn, _) = x11rb::connect(None).map_err(|e| format!("connect: {e}"))?;
    let atom = conn
        .intern_atom(false, b"_NET_WM_USER_TIME")
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?
        .atom;
    conn.change_property32(PropMode::REPLACE, window, atom, AtomEnum::CARDINAL, &[0])
        .map_err(|e| e.to_string())?;
    // A round trip guarantees the change is applied before anything else touches the window.
    conn.get_input_focus().map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?;
    Ok(())
}
