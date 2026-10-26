//! Key/value rows, collapsible sections, badges, small reusable bits.

use egui::{Color32, CornerRadius, RichText, Ui, Vec2};

/// Collapsible section with remembered open state.
pub fn section<R>(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, title: &str, default_open: bool, content: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    let title = crate::i18n::section_title(title);
    let id = ui.make_persistent_id(id);
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, default_open);
    let header = ui.horizontal(|ui| {
        let icon = if state.is_open() { "▾" } else { "▸" };
        let r = ui.add(egui::Label::new(RichText::new(format!("{icon}  {title}")).strong().size(13.5)).sense(egui::Sense::click()));
        if r.clicked() {
            state.toggle(ui);
        }
        r
    });
    let _ = header;
    let out = state.show_body_unindented(ui, |ui| {
        ui.add_space(2.0);
        egui::Frame::new().inner_margin(egui::Margin { left: 10, right: 0, top: 0, bottom: 4 }).show(ui, content).inner
    });
    ui.add_space(4.0);
    out.map(|o| o.inner)
}

fn looks_code(v: &str) -> bool {
    v.contains("${") || v.contains("#{") || v.contains('.') && !v.contains(' ') && v.len() > 6 || v.starts_with('{') || v.starts_with('[') || v.chars().all(|c| c.is_ascii_hexdigit() || c == '-') && v.len() >= 32
}

/// Two-column row; the value is selectable text with a copy button in front of it.
pub fn kv(ui: &mut Ui, key: &str, value: &str) -> egui::Response {
    kv_opt(ui, key, value, looks_code(value))
}

pub fn kv_mono(ui: &mut Ui, key: &str, value: &str) -> egui::Response {
    kv_opt(ui, key, value, true)
}

pub fn kv_opt(ui: &mut Ui, key: &str, value: &str, mono: bool) -> egui::Response {
    let key = crate::i18n::kv_label(key);
    let resp = ui.horizontal_top(|ui| {
        ui.add_sized([120.0, 18.0], egui::Label::new(RichText::new(key).color(ui.visuals().weak_text_color()).size(12.0)).halign(egui::Align::Max).truncate());
        ui.spacing_mut().item_spacing.x = 4.0;
        if !value.is_empty() {
            crate::widgets::copy_button(ui, value);
        }
        let text = if mono { RichText::new(value).monospace().size(12.0) } else { RichText::new(value) };
        // Plain selectable text: drag to select part of it, the button copies all of it.
        ui.add(egui::Label::new(text).wrap().selectable(true))
    });
    resp.inner
}

/// Documentation or a description: a labelled block rendered as Markdown-lite (line breaks kept,
/// see `widgets::markdown`) at the full width of the panel, folded when long. Viewers place it
/// after every other field so a long text never pushes the short fields out of sight.
pub fn doc_block(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, key: &str, text: &str) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    let key = crate::i18n::kv_label(key);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(RichText::new(key).color(ui.visuals().weak_text_color()).size(12.0));
        crate::widgets::copy_button(ui, text);
    });
    let id = ui.make_persistent_id(("doc-block", id));
    let expanded = ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    const FOLD_AT: usize = 14;
    let long = super::markdown::weight(text) > FOLD_AT + 4;
    let shown = if long && !expanded { fold(text, FOLD_AT) } else { text.to_string() };
    egui::Frame::new().fill(ui.visuals().faint_bg_color).corner_radius(4.0).inner_margin(egui::Margin { left: 8, right: 6, top: 5, bottom: 5 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        super::markdown::show(ui, &shown);
        if long {
            let hidden = super::markdown::weight(text).saturating_sub(super::markdown::weight(&shown));
            let label = if expanded { crate::t!("doc-show-less") } else { crate::t!("doc-show-all", n = hidden) };
            if ui.link(RichText::new(label).small()).clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(id, !expanded));
            }
        }
    });
}

/// The first display lines of `text` (cut at a line or word boundary), for folded documentation.
pub fn fold(text: &str, max_weight: usize) -> String {
    let mut out = String::new();
    let mut w = 0;
    for l in text.lines() {
        let lw = 1 + l.chars().count() / 70;
        if w + lw > max_weight {
            let room = (max_weight.saturating_sub(w)) * 70;
            if room > 0 {
                let cut: String = l.chars().take(room).collect();
                let cut = cut.rsplit_once(' ').map(|(a, _)| a.to_string()).unwrap_or(cut);
                out.push_str(&cut);
            }
            out.push_str(" …");
            return out;
        }
        out.push_str(l);
        out.push('\n');
        w += lw;
    }
    out
}

/// A row whose value is a link (clickable, accent coloured).
pub fn kv_link(ui: &mut Ui, key: &str, value: &str) -> bool {
    let key = crate::i18n::kv_label(key);
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.add_sized([120.0, 18.0], egui::Label::new(RichText::new(key).color(ui.visuals().weak_text_color()).size(12.0)).halign(egui::Align::Max).truncate());
        ui.spacing_mut().item_spacing.x = 4.0;
        crate::widgets::copy_button(ui, value);
        if ui.link(RichText::new(value).monospace().size(12.0)).on_hover_text(crate::t!("common-open")).clicked() {
            clicked = true;
        }
    });
    clicked
}

pub fn badge(ui: &mut Ui, text: &str, color: Color32) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(11.0), Color32::WHITE);
    let size = Vec2::new((galley.size().x + 10.0).max(18.0), 18.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    ui.painter().rect_filled(rect, 9.0, color);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, Color32::WHITE);
    resp
}

pub fn pill(ui: &mut Ui, text: &str, color: Color32) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(11.0), color);
    let size = Vec2::new(galley.size().x + 12.0, 18.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    ui.painter().rect(rect, CornerRadius::same(9), color.gamma_multiply(0.15), egui::Stroke::new(1.0, color.gamma_multiply(0.6)), egui::StrokeKind::Inside);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
    resp
}

/// A state label coloured by instance state.
pub fn state_color(state: &str, tokens: &crate::theme::Tokens) -> Color32 {
    match state {
        "ACTIVE" => tokens.running,
        "SUSPENDED" => tokens.suspended,
        "COMPLETED" => tokens.completed,
        "EXTERNALLY_TERMINATED" | "INTERNALLY_TERMINATED" => tokens.incident,
        _ => tokens.fg1,
    }
}

pub fn state_label(state: &str) -> &str {
    match state {
        "ACTIVE" => "active",
        "SUSPENDED" => "suspended",
        "COMPLETED" => "completed",
        "EXTERNALLY_TERMINATED" => "terminated",
        "INTERNALLY_TERMINATED" => "terminated (internal)",
        s => s,
    }
}

/// Thin indeterminate progress line at the top of a rect.
pub fn progress_line(ui: &mut Ui, rect: egui::Rect, color: Color32) {
    let t = ui.input(|i| i.time) as f32;
    let w = rect.width();
    let x = ((t * 0.8) % 1.0) * (w + 120.0) - 120.0;
    let r = egui::Rect::from_min_size(egui::Pos2::new(rect.min.x + x.max(0.0), rect.min.y), Vec2::new((x + 120.0).min(w) - x.max(0.0), 2.0));
    ui.painter().rect_filled(r, 0.0, color);
    ui.ctx().request_repaint();
}

/// Filter text field with `/` hint; returns true when its content changed.
pub fn filter_box(ui: &mut Ui, id: egui::Id, text: &mut String, hint: &str, want_focus: &mut bool) -> egui::Response {
    let r = ui.add(egui::TextEdit::singleline(text).id(id).hint_text(format!("{hint}  (/)")).desired_width(f32::INFINITY));
    if *want_focus {
        r.request_focus();
        *want_focus = false;
    }
    if r.has_focus() && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        if text.is_empty() {
            r.surrender_focus();
        } else {
            text.clear();
        }
    }
    r
}

pub fn heading_row(ui: &mut Ui, title: &str, subtitle: Option<&str>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(16.0).strong());
        if let Some(s) = subtitle {
            ui.label(RichText::new(s).weak());
        }
    });
}

pub fn empty_state(ui: &mut Ui, text: &str) {
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(text).weak());
    });
}
