pub mod batches;
pub mod connect;
pub mod definition;
pub mod definitions;
pub mod file;
pub mod incidents;
pub mod instance;
pub mod search;
pub mod settings;

pub use batches::BatchesPlace;
pub use definition::DefinitionPlace;
pub use definitions::DefinitionsPlace;
pub use file::FilePlace;
pub use incidents::IncidentsPlace;
pub use instance::InstancePlace;
pub use search::SearchPlace;
pub use settings::SettingsPlace;

use crate::app::App;
use egui::Ui;

/// Right-hand dock frame with tab strip. Returns the inner ui result.
pub fn dock<R>(ui: &mut Ui, app: &mut App, id: &str, tabs: &[&str], current: &mut usize, content: impl FnOnce(&mut Ui, usize, &mut App) -> R) -> Option<R> {
    if !app.dock_visible {
        return None;
    }
    let mut out = None;
    let max_w = (ui.available_width() * 0.55).max(300.0);
    egui::Panel::right("dock").resizable(true).default_size(app.settings.dock_width).min_size(280.0).max_size(max_w).frame(egui::Frame::new().fill(app.tokens.bg1)).show(ui, |ui| {
        let w = ui.available_width();
        ui.set_max_width(w);
        // The panel width itself is persisted by egui (eframe storage); settings.dock_width is only the first-run default.
        egui::Frame::new().inner_margin(egui::Margin { left: 8, right: 8, top: 6, bottom: 2 }).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (i, t) in tabs.iter().enumerate() {
                    let selected = *current == i;
                    let label = if i < 9 { format!("{t}") } else { t.to_string() };
                    let r = ui.selectable_label(selected, label);
                    let r = if i < 9 { r.on_hover_text(format!("Ctrl+{}", i + 1)) } else { r };
                    if r.clicked() {
                        *current = i;
                    }
                }
            });
        });
        ui.separator();
        // Ctrl+1..9 switch dock tabs
        ui.input_mut(|inp| {
            let keys = [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3, egui::Key::Num4, egui::Key::Num5, egui::Key::Num6, egui::Key::Num7, egui::Key::Num8, egui::Key::Num9];
            for (i, k) in keys.iter().enumerate() {
                if i < tabs.len() && inp.consume_key(egui::Modifiers::CTRL, *k) {
                    *current = i;
                }
            }
        });
        egui::ScrollArea::vertical().id_salt((id, *current)).auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::new().inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
                out = Some(content(ui, *current, app));
            });
        });
    });
    out
}

/// Shows loading / error status for a loader at the top of a panel.
pub fn loader_status<T>(ui: &mut Ui, l: &crate::net::Loader<T>, tokens: &crate::theme::Tokens) {
    if l.is_loading() && l.data.is_none() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(egui::RichText::new("Loading…").weak());
        });
    }
    if let Some(e) = &l.error {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("⚠").color(tokens.incident));
            ui.label(egui::RichText::new(e.short()).color(tokens.incident).small());
        });
    }
}
