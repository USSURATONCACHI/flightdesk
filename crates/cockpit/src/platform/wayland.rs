//! `wp_pointer_warp_v1` on the window's existing Wayland connection.
//!
//! winit owns the connection and reads its socket; we attach a second event queue to the same
//! `wl_display` (libwayland allows this), bind our own `wl_pointer` to learn the enter serial
//! the protocol demands, and send warp requests with it. Events for our queue are read by winit
//! and dispatched when `pump` runs each frame.

use std::ffi::c_void;
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_pointer, wl_registry, wl_seat, wl_surface};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::wp::pointer_warp::v1::client::wp_pointer_warp_v1::WpPointerWarpV1;

#[derive(Default)]
struct Data {
    pointer: Option<wl_pointer::WlPointer>,
    /// Serial of the most recent pointer enter on any of our surfaces.
    enter_serial: Option<u32>,
}

pub struct WaylandWarp {
    conn: Connection,
    queue: EventQueue<Data>,
    data: Data,
    warp: Option<WpPointerWarpV1>,
    surface: wl_surface::WlSurface,
}

impl WaylandWarp {
    /// # Safety
    /// `display` must be the live `wl_display*` and `surface` the live `wl_surface*` of the window.
    pub unsafe fn new(display: *mut c_void, surface: *mut c_void) -> Result<Self, String> {
        let backend = unsafe { Backend::from_foreign_display(display.cast()) };
        let conn = Connection::from_backend(backend);
        let (globals, mut queue) = registry_queue_init::<Data>(&conn).map_err(|e| format!("registry: {e}"))?;
        let qh = queue.handle();
        let warp: Option<WpPointerWarpV1> = globals.bind(&qh, 1..=1, ()).ok();
        let seat: wl_seat::WlSeat = globals.bind(&qh, 1..=9, ()).map_err(|e| format!("wl_seat: {e}"))?;
        let surface_id = unsafe { ObjectId::from_ptr(wl_surface::WlSurface::interface(), surface.cast()) }.map_err(|e| format!("surface: {e}"))?;
        let surface = wl_surface::WlSurface::from_id(&conn, surface_id).map_err(|e| format!("surface: {e}"))?;
        let mut data = Data::default();
        // Receive the seat capabilities (creates our wl_pointer) before returning.
        queue.roundtrip(&mut data).map_err(|e| format!("roundtrip: {e}"))?;
        let _ = seat;
        Ok(Self { conn, queue, data, warp, surface })
    }

    pub fn supported(&self) -> bool {
        self.warp.is_some()
    }

    pub fn pump(&mut self) {
        let _ = self.queue.dispatch_pending(&mut self.data);
    }

    /// Ask the compositor to move the pointer to surface-local `(x, y)`. False when the protocol
    /// is missing or we have not seen a pointer enter yet (the request would be rejected).
    pub fn warp(&mut self, x: f64, y: f64) -> bool {
        self.pump();
        let (Some(warp), Some(pointer), Some(serial)) = (&self.warp, &self.data.pointer, self.data.enter_serial) else {
            return false;
        };
        warp.warp_pointer(&self.surface, pointer, x, y, serial);
        self.conn.flush().is_ok()
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Data {
    fn event(_: &mut Self, _: &wl_registry::WlRegistry, _: wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<wl_seat::WlSeat, ()> for Data {
    fn event(data: &mut Self, seat: &wl_seat::WlSeat, event: wl_seat::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        if let wl_seat::Event::Capabilities { capabilities: wayland_client::WEnum::Value(caps) } = event {
            let has_pointer = caps.contains(wl_seat::Capability::Pointer);
            if has_pointer && data.pointer.is_none() {
                data.pointer = Some(seat.get_pointer(qh, ()));
            } else if !has_pointer {
                if let Some(p) = data.pointer.take() {
                    p.release();
                }
                data.enter_serial = None;
            }
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for Data {
    fn event(data: &mut Self, _: &wl_pointer::WlPointer, event: wl_pointer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        match event {
            wl_pointer::Event::Enter { serial, .. } => data.enter_serial = Some(serial),
            wl_pointer::Event::Leave { .. } => data.enter_serial = None,
            _ => {}
        }
    }
}

impl Dispatch<WpPointerWarpV1, ()> for Data {
    fn event(_: &mut Self, _: &WpPointerWarpV1, _: <WpPointerWarpV1 as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
