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

/// A small painted "copy" icon button (two overlapping sheets); copies `text` when clicked and
/// flashes a tick for a moment.
pub fn copy_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let size = egui::Vec2::splat(16.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let resp = resp.on_hover_text(format!("Copy\n{}", crate::util::ellipsize(text, 120)));
    let flash_id = resp.id.with("copied-at");
    if resp.clicked() {
        ui.ctx().copy_text(text.to_string());
        ui.ctx().data_mut(|d| d.insert_temp(flash_id, ui.input(|i| i.time)));
    }
    let now = ui.input(|i| i.time);
    let flashed = ui.ctx().data(|d| d.get_temp::<f64>(flash_id)).map(|t| now - t < 1.0).unwrap_or(false);
    let visuals = ui.style().interact(&resp);
    let color = if flashed { crate::theme::COMPLETED } else if resp.hovered() { visuals.fg_stroke.color } else { ui.visuals().weak_text_color() };
    let p = ui.painter();
    if flashed {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
        let c = rect.center();
        p.line_segment([c + egui::vec2(-4.0, 0.0), c + egui::vec2(-1.0, 3.0)], egui::Stroke::new(1.8, color));
        p.line_segment([c + egui::vec2(-1.0, 3.0), c + egui::vec2(4.5, -3.5)], egui::Stroke::new(1.8, color));
    } else {
        let stroke = egui::Stroke::new(1.2, color);
        let back = egui::Rect::from_min_size(rect.min + egui::vec2(3.0, 2.0), egui::vec2(7.5, 9.0));
        let front = egui::Rect::from_min_size(rect.min + egui::vec2(5.5, 4.5), egui::vec2(7.5, 9.0));
        // back sheet: only the edges not hidden by the front sheet
        p.line_segment([back.left_bottom(), back.left_top()], stroke);
        p.line_segment([back.left_top(), back.right_top()], stroke);
        p.line_segment([back.left_bottom(), egui::pos2(front.min.x, back.max.y)], stroke);
        p.line_segment([back.right_top(), egui::pos2(back.max.x, front.min.y)], stroke);
        p.rect_stroke(front, 1.5, stroke, egui::StrokeKind::Middle);
    }
    resp
}

/// An identifier cell: a copy button plus truncated monospace text; copies the full value.
/// Used for uuid and business-key columns.
pub fn id_cell(ui: &mut egui::Ui, full: &str, shown: &str) -> egui::Response {
    let r = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        copy_button(ui, full);
        ui.add(egui::Label::new(egui::RichText::new(shown).monospace().size(11.5)).truncate()).on_hover_text(full)
    });
    r.inner
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
