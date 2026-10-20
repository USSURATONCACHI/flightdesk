pub mod confirm;
pub mod kv;
pub mod split;
pub mod toast;

pub use confirm::Confirm;
pub use kv::*;
pub use toast::{ToastKind, Toasts};

/// A deferred action on the app (undo handlers, confirmed actions).
pub type AppAction = Box<dyn FnOnce(&mut crate::app::App) + Send>;

/// A table cell label that never widens its column: long text is truncated with an ellipsis
/// and shown in full on hover.
pub fn cell(ui: &mut egui::Ui, text: impl Into<egui::RichText>) -> egui::Response {
    let text: egui::RichText = text.into();
    let full = text.text().to_string();
    let r = ui.add(egui::Label::new(text).truncate());
    if full.len() > 12 {
        r.on_hover_text(full)
    } else {
        r
    }
}

/// Host a table: the table keeps its natural minimum width and the area scrolls horizontally
/// when the pane is narrower than that.
pub fn table_area<R>(ui: &mut egui::Ui, id: impl std::hash::Hash + std::fmt::Debug, min_width: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let outer = ui.available_width();
    egui::ScrollArea::horizontal().id_salt(id).auto_shrink([false, false]).show(ui, |ui| {
        ui.set_width(outer.max(min_width));
        add(ui)
    }).inner
}

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
