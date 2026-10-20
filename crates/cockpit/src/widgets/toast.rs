use egui::{Align2, Color32, Vec2};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

pub struct Toast {
    pub kind: ToastKind,
    pub text: String,
    pub details: Option<String>,
    pub created: Instant,
    pub ttl: Option<f32>,
    pub undo: Option<super::AppAction>,
    /// Label of the action button ("Undo" for undo handlers, e.g. "Fix" for repairs).
    pub action_label: String,
    pub id: u64,
    pub hovered: bool,
    pub expanded: bool,
}

#[derive(Default)]
pub struct Toasts {
    pub items: Vec<Toast>,
    next_id: u64,
}

pub enum ToastAction {
    Undo(super::AppAction),
}

impl Toasts {
    pub fn push(&mut self, kind: ToastKind, text: impl Into<String>) -> u64 {
        self.push_full(kind, text, None, None)
    }
    pub fn error(&mut self, text: impl Into<String>, details: Option<String>) -> u64 {
        self.push_full(ToastKind::Error, text, details, None)
    }
    pub fn push_full(&mut self, kind: ToastKind, text: impl Into<String>, details: Option<String>, undo: Option<super::AppAction>) -> u64 {
        self.push_action_inner(kind, text, details, crate::t!("common-undo"), undo, true)
    }

    /// A toast with a custom action button; actions keep the toast until clicked or dismissed.
    pub fn push_with_action(&mut self, kind: ToastKind, text: impl Into<String>, details: Option<String>, action_label: impl Into<String>, action: Option<super::AppAction>) -> u64 {
        self.push_action_inner(kind, text, details, action_label.into(), action, false)
    }

    fn push_action_inner(&mut self, kind: ToastKind, text: impl Into<String>, details: Option<String>, action_label: String, action: Option<super::AppAction>, is_undo: bool) -> u64 {
        let undo = action;
        self.next_id += 1;
        let ttl = match kind {
            ToastKind::Error => None,
            ToastKind::Warning => if is_undo { Some(10.0) } else { None },
            _ => Some(if undo.is_some() { 8.0 } else { 4.5 }),
        };
        self.items.push(Toast { kind, text: text.into(), details, created: Instant::now(), ttl, undo, action_label, id: self.next_id, hovered: false, expanded: false });
        if self.items.len() > 6 {
            self.items.remove(0);
        }
        self.next_id
    }

    /// Draw the stack; returns actions requested by the user (undo).
    pub fn ui(&mut self, ctx: &egui::Context, tokens: &crate::theme::Tokens) -> Vec<ToastAction> {
        let mut actions = vec![];
        self.items.retain(|t| match t.ttl {
            Some(ttl) => t.hovered || t.created.elapsed().as_secs_f32() < ttl,
            None => true,
        });
        if self.items.is_empty() {
            return actions;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
        let mut remove: Vec<u64> = vec![];
        egui::Area::new(egui::Id::unique("toasts")).order(egui::Order::Foreground).anchor(Align2::RIGHT_BOTTOM, Vec2::new(-16.0, -34.0)).interactable(true).show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.vertical(|ui| {
                for t in self.items.iter_mut().rev() {
                    let (bar, icon) = match t.kind {
                        ToastKind::Info => (tokens.running, "ℹ"),
                        ToastKind::Success => (tokens.completed, "✓"),
                        ToastKind::Warning => (tokens.suspended, "⚠"),
                        ToastKind::Error => (tokens.incident, "✕"),
                    };
                    let frame = egui::Frame::popup(ui.style()).fill(tokens.bg2).inner_margin(egui::Margin::symmetric(12, 8));
                    let resp = frame.show(ui, |ui| {
                        ui.set_min_width(300.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(icon).color(bar).strong());
                            ui.vertical(|ui| {
                                ui.add(egui::Label::new(&t.text).wrap());
                                if let Some(d) = &t.details {
                                    if t.expanded {
                                        ui.add(egui::Label::new(egui::RichText::new(d).monospace().size(11.0)).wrap());
                                    }
                                }
                                ui.horizontal(|ui| {
                                    if t.undo.is_some() && ui.small_button(t.action_label.clone()).clicked() {
                                        if let Some(u) = t.undo.take() {
                                            actions.push(ToastAction::Undo(u));
                                        }
                                        remove.push(t.id);
                                    }
                                    if t.details.is_some() {
                                        if ui.small_button(if t.expanded { crate::t!("toast-hide-details") } else { crate::t!("common-details") }).clicked() {
                                            t.expanded = !t.expanded;
                                        }
                                        if ui.small_button(crate::t!("common-copy")).clicked() {
                                            let text = format!("{}\n{}", t.text, t.details.clone().unwrap_or_default());
                                            ui.ctx().copy_text(text);
                                        }
                                    }
                                });
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                if ui.small_button("✕").clicked() {
                                    remove.push(t.id);
                                }
                            });
                        });
                    });
                    let r = resp.response.rect;
                    ui.painter().rect_filled(egui::Rect::from_min_size(r.min, Vec2::new(3.0, r.height())), 2.0, bar);
                    t.hovered = ui.rect_contains_pointer(r);
                    let _ = Color32::TRANSPARENT;
                }
            });
        });
        self.items.retain(|t| !remove.contains(&t.id));
        actions
    }
}
