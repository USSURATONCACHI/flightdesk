//! Inline/modal confirmation for destructive actions. Focus defaults to Cancel; Enter confirms
//! only once Confirm is focused or `y` is typed.

use egui::{Key, Modifiers};

pub struct Confirm {
    pub title: String,
    pub body: String,
    pub confirm_label: String,
    pub danger: bool,
    pub action: Option<super::AppAction>,
    first_frame: bool,
}

impl Confirm {
    pub fn new(title: impl Into<String>, body: impl Into<String>, confirm_label: impl Into<String>, action: impl FnOnce(&mut crate::app::App) + Send + 'static) -> Self {
        Self { title: title.into(), body: body.into(), confirm_label: confirm_label.into(), danger: true, action: Some(Box::new(action)), first_frame: true }
    }

    /// Returns Some(true) to run, Some(false) to cancel, None while open.
    pub fn ui(&mut self, ctx: &egui::Context, tokens: &crate::theme::Tokens) -> Option<bool> {
        let mut result = None;
        let modal = egui::Modal::new(egui::Id::unique("confirm-modal")).show(ctx, |ui| {
            ui.set_width(440.0);
            ui.heading(&self.title);
            ui.add_space(6.0);
            ui.add(egui::Label::new(&self.body).wrap());
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let b = egui::Button::new(egui::RichText::new(&self.confirm_label).color(egui::Color32::WHITE)).fill(if self.danger { tokens.incident } else { tokens.accent });
                    let confirm = ui.add(b);
                    let cancel = ui.button(crate::t!("common-cancel"));
                    if self.first_frame {
                        cancel.request_focus();
                        self.first_frame = false;
                    }
                    if confirm.clicked() || (confirm.has_focus() && ui.input(|i| i.key_pressed(Key::Enter))) {
                        result = Some(true);
                    }
                    if cancel.clicked() || (cancel.has_focus() && ui.input(|i| i.key_pressed(Key::Enter))) {
                        result = Some(false);
                    }
                    ui.label(egui::RichText::new(crate::t!("confirm-hint")).weak().small());
                });
            });
            ctx.input_mut(|i| {
                if i.consume_key(Modifiers::NONE, Key::Escape) {
                    result = Some(false);
                }
                if i.consume_key(Modifiers::NONE, Key::Y) {
                    result = Some(true);
                }
            });
        });
        if modal.should_close() && result.is_none() {
            result = Some(false);
        }
        result
    }
}
