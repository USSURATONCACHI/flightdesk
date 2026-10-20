//! Connection screen shown until an engine is reachable.

use crate::app::App;
use crate::settings::EngineProfile;
use egui::{RichText, Ui};

#[derive(Default)]
pub struct ConnectScreen {
    pub profile: EngineProfile,
    pub selected: usize,
    pub error: Option<String>,
    pub connecting: bool,
    pub show_password: bool,
    initialised: bool,
}

impl ConnectScreen {
    pub fn load_from(&mut self, settings: &crate::settings::Settings) {
        self.selected = settings.last_engine.min(settings.engines.len().saturating_sub(1));
        self.profile = settings.engines.get(self.selected).cloned().unwrap_or_default();
        self.initialised = true;
    }
}

pub fn ui(ui: &mut egui::Ui, app: &mut App) {
    let ctx = ui.ctx().clone();
    if !app.connect.initialised {
        let s = app.settings.clone();
        app.connect.load_from(&s);
    }
    egui::CentralPanel::default().frame(egui::Frame::new().fill(app.tokens.bg0)).show(ui, |ui| {
        let avail = ui.available_rect_before_wrap();
        let w = 460.0_f32.min(avail.width() - 32.0);
        let rect = egui::Rect::from_center_size(avail.center() - egui::Vec2::new(0.0, 40.0), egui::Vec2::new(w, 420.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            egui::Frame::new().fill(app.tokens.bg2).stroke(egui::Stroke::new(1.0, app.tokens.bg3)).corner_radius(8.0).inner_margin(24.0).show(ui, |ui| {
                ui.set_width(w - 48.0);
                ui.label(RichText::new("Flightdeck").size(22.0).strong());
                ui.label(RichText::new("Connect to a Camunda 7 engine").weak());
                ui.add_space(14.0);
                form(ui, app);
            });
        });
        // Keyboard: Enter connects when not connecting.
        if !app.connect.connecting && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !ctx.wants_keyboard_input_except_enter() {
            start_connect(app);
        }
    });
}

trait EnterCheck {
    fn wants_keyboard_input_except_enter(&self) -> bool;
}
impl EnterCheck for egui::Context {
    fn wants_keyboard_input_except_enter(&self) -> bool {
        // Text fields consume Enter only for multiline; our fields are single line, so allow.
        false
    }
}

fn form(ui: &mut Ui, app: &mut App) {
    let n = app.settings.engines.len();
    if n > 1 {
        ui.horizontal(|ui| {
            ui.label("Saved");
            let mut sel = app.connect.selected;
            egui::ComboBox::from_id_salt("engine-combo").selected_text(app.settings.engines.get(sel).map(|e| e.name.clone()).unwrap_or_default()).show_ui(ui, |ui| {
                for (i, e) in app.settings.engines.iter().enumerate() {
                    ui.selectable_value(&mut sel, i, format!("{}  —  {}", e.name, e.url));
                }
            });
            if sel != app.connect.selected {
                app.connect.selected = sel;
                app.connect.profile = app.settings.engines[sel].clone();
            }
            if ui.small_button("Delete").on_hover_text("Remove this saved connection").clicked() && n > 1 {
                app.settings.engines.remove(app.connect.selected);
                app.connect.selected = 0;
                app.connect.profile = app.settings.engines[0].clone();
                app.settings_dirty = true;
            }
        });
        ui.add_space(6.0);
    }
    let p = &mut app.connect.profile;
    egui::Grid::new("connect-grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        let l = ui.label("Name");
        ui.add(egui::TextEdit::singleline(&mut p.name).desired_width(f32::INFINITY)).labelled_by(l.id);
        ui.end_row();
        let l = ui.label("REST URL");
        let r = ui.add(egui::TextEdit::singleline(&mut p.url).hint_text("http://host:8080/engine-rest  (a Cockpit URL works too)").desired_width(f32::INFINITY)).labelled_by(l.id);
        if !app.connect.initialised || p.url.is_empty() {
            r.request_focus();
        }
        ui.end_row();
        let l = ui.label("User");
        ui.add(egui::TextEdit::singleline(&mut p.user).hint_text("optional (basic auth)").desired_width(f32::INFINITY)).labelled_by(l.id);
        ui.end_row();
        let l = ui.label("Password");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut p.password).password(!app.connect.show_password).desired_width(ui.available_width() - 40.0)).labelled_by(l.id);
            ui.toggle_value(&mut app.connect.show_password, "👁").on_hover_text("Show password");
        });
        ui.end_row();
        ui.label("");
        ui.horizontal(|ui| {
            ui.checkbox(&mut p.remember_password, "Remember password").on_hover_text("Stored in plain text in the settings file");
            ui.checkbox(&mut p.accept_invalid_certs, "Allow self-signed TLS");
        });
        ui.end_row();
    });
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        let btn = egui::Button::new(RichText::new(if app.connect.connecting { "Connecting…" } else { "Connect  ↵" }).color(egui::Color32::WHITE)).fill(app.tokens.accent).min_size(egui::Vec2::new(130.0, 30.0));
        if ui.add_enabled(!app.connect.connecting, btn).clicked() {
            start_connect(app);
        }
        if app.connect.connecting {
            ui.spinner();
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Open a .bpmn file").on_hover_text("Offline viewer: no engine needed").clicked() {
                if let Some(path) = rfd_pick() {
                    app.navigate(crate::state::Nav::File { path }, crate::state::NavMode::NewTab);
                    app.offline = true;
                }
            }
        });
    });
    if let Some(e) = &app.connect.error {
        ui.add_space(8.0);
        ui.label(RichText::new(e).color(app.tokens.incident));
    }
    ui.add_space(10.0);
    ui.label(RichText::new("The URL is normalised: `host:8080`, `…/camunda/app/cockpit/…` and `…/engine-rest` all work.").weak().small());
}

fn rfd_pick() -> Option<std::path::PathBuf> {
    // No file-dialog crate: accept a path typed in an env var or the CLI; keep the UI honest.
    std::env::var("FLIGHTDECK_BPMN").ok().map(std::path::PathBuf::from)
}

pub fn start_connect(app: &mut App) {
    app.connect.error = None;
    app.connect.connecting = true;
    let mut profile = app.connect.profile.clone();
    profile.url = camunda::normalize_base_url(&profile.url);
    app.connect.profile.url = profile.url.clone();
    app.connect_to(profile);
}
