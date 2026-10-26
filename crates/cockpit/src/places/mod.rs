pub mod batches;
pub mod connect;
pub mod definition;
pub mod definitions;
pub mod diff;
pub mod file;
pub mod incidents;
pub mod instance;
pub mod search;
pub mod settings;
pub mod source;
pub mod xml;

pub use batches::BatchesPlace;
pub use definition::DefinitionPlace;
pub use definitions::DefinitionsPlace;
pub use diff::DiffPlace;
pub use file::FilePlace;
pub use incidents::IncidentsPlace;
pub use instance::InstancePlace;
pub use search::SearchPlace;
pub use settings::SettingsPlace;
pub use xml::XmlPlace;

use crate::app::App;
use crate::widgets::split::{child_in, split, Side};
use egui::Ui;

/// Right-hand dock with a tab strip. Returns the rect left for the main area and the content's
/// result (`None` when the dock is hidden). The dock has an exact width, so fill-width widgets
/// inside it never push it wider (see `widgets::split`).
pub fn dock<R>(ui: &mut Ui, app: &mut App, id: &str, tabs: &[&str], current: &mut usize, content: impl FnOnce(&mut Ui, usize, &mut App) -> R) -> (egui::Rect, Option<R>) {
    if !app.dock_visible {
        return (ui.available_rect_before_wrap(), None);
    }
    let before = app.settings.dock_width;
    let sp = split(ui, egui::Id::unique(("dock-split", id)), Side::Right, &mut app.settings.dock_width, 280.0, 0.6);
    if (app.settings.dock_width - before).abs() > 0.5 {
        app.settings_dirty = true;
    }
    ui.painter().rect_filled(sp.pane, 0.0, app.tokens.bg1);
    let mut pane = child_in(ui, sp.pane);
    let out = egui::Frame::new().inner_margin(egui::Margin { left: 8, right: 8, top: 6, bottom: 2 }).show(&mut pane, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (i, t) in tabs.iter().enumerate() {
                let r = ui.selectable_label(*current == i, crate::i18n::dock_tab(t));
                let r = if i < 9 { r.on_hover_text(format!("Ctrl+{}", i + 1)) } else { r };
                if r.clicked() {
                    *current = i;
                }
            }
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
        let w = ui.available_width();
        egui::ScrollArea::vertical().id_salt((id, *current)).auto_shrink([false, false]).show(ui, |ui| {
            ui.set_width(w);
            egui::Frame::new().inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| content(ui, *current, app)).inner
        }).inner
    });
    (sp.main, Some(out.inner))
}

/// Shows loading / error status for a loader at the top of a panel.
pub fn loader_status<T>(ui: &mut Ui, l: &crate::net::Loader<T>, tokens: &crate::theme::Tokens) {
    if l.is_loading() && l.data.is_none() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(egui::RichText::new(crate::t!("common-loading")).weak());
        });
    }
    if let Some(e) = &l.error {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("⚠").color(tokens.incident));
            ui.label(egui::RichText::new(e.short()).color(tokens.incident).small());
        });
    }
}
