pub mod confirm;
pub mod kv;
pub mod toast;

pub use confirm::Confirm;
pub use kv::*;
pub use toast::{ToastKind, Toasts};

/// Compact status (spinner or error) for a loader inside a toolbar row.
pub fn loader_status_inline<T>(ui: &mut egui::Ui, l: &crate::net::Loader<T>, tokens: &crate::theme::Tokens) {
    if l.is_loading() {
        ui.spinner();
    }
    if let Some(e) = &l.error {
        ui.label(egui::RichText::new(format!("⚠ {}", e.short())).color(tokens.incident).small()).on_hover_text(e.to_string());
    }
}

/// Tooltip-styled popup at the pointer (for custom-painted widgets).
pub fn tooltip_at_pointer(ui: &mut egui::Ui, id: egui::Id, content: impl FnOnce(&mut egui::Ui)) {
    let Some(pos) = ui.ctx().input(|i| i.pointer.hover_pos()) else { return };
    let ctx = ui.ctx().clone();
    egui::Area::new(id).order(egui::Order::Tooltip).fixed_pos(pos + egui::Vec2::new(14.0, 18.0)).interactable(false).show(&ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_max_width(380.0);
            content(ui);
        });
    });
}
