//! Offline viewer for a local .bpmn file (also used for screenshots and tests).

use crate::app::App;
use crate::inspector::{self, InspectorAction, RuntimeFacts};
use crate::viewport::{self, DiagramView, Overlays};
use bpmn::Model;
use std::sync::Arc;

pub struct FilePlace {
    pub path: std::path::PathBuf,
    pub model: Option<Arc<Model>>,
    pub error: Option<String>,
    pub view: DiagramView,
    pub overlays: Overlays,
    pub dock_tab: usize,
    warned: bool,
}

impl FilePlace {
    pub fn new(path: std::path::PathBuf) -> Self {
        let (model, error) = match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|s| bpmn::parse(&s).map_err(|e| e.to_string())) {
            Ok(m) => (Some(Arc::new(m)), None),
            Err(e) => (None, Some(e)),
        };
        Self { view: DiagramView::new(egui::Id::unique(("file", &path))), path, model, error, overlays: Overlays::default(), dock_tab: 0, warned: false }
    }
    pub fn title(&self) -> String {
        self.path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| "file".into())
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        if !self.warned && self.model.as_ref().map(|m| !m.warnings.is_empty()).unwrap_or(false) {
            self.warned = true;
            let n = self.model.as_ref().map(|m| m.warnings.len()).unwrap_or(0);
            app.toasts.push(crate::widgets::ToastKind::Warning, format!("{n} diagram warning{} (see the Inspector tab)", if n == 1 { "" } else { "s" }));
        }
        let Some(model) = self.model.clone() else {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.label(format!("Cannot open {}: {}", self.path.display(), self.error.clone().unwrap_or_default()));
            });
            return;
        };
        let mut actions = vec![];
        let (main_rect, _) = super::dock(ui, app, "file-dock", &["Inspector", "Matches"], &mut self.dock_tab, |ui, tab, _app| match tab {
            0 => match self.view.selection.or(self.view.hover) {
                Some(r) => actions = inspector::show(ui, &model, r, &RuntimeFacts::default(), false),
                None => {
                    crate::widgets::empty_state(ui, "Select an element to inspect it.\nTab cycles elements, Ctrl+F finds them.");
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new(format!("{} flow nodes · {} flows · {} warnings", model.flow_nodes().count(), model.flows.len(), model.warnings.len())).weak());
                    for w in model.warnings.iter().take(20) {
                        ui.label(egui::RichText::new(w).small().weak());
                    }
                }
            },
            _ => {
                let mut jump = None;
                for (i, m) in self.view.find.matches.iter().enumerate() {
                    let name = match m.target {
                        bpmn::Ref::Element(e) => model.elements[e].display_name().to_string(),
                        bpmn::Ref::Flow(f) => model.flows[f].display_name().to_string(),
                    };
                    if ui.selectable_label(self.view.find.cursor == i, format!("{name}  ·  {}: {}", m.field, crate::util::ellipsize(&m.snippet, 60))).clicked() {
                        jump = Some(i);
                    }
                }
                if let Some(i) = jump {
                    self.view.find.cursor = i;
                    if let Some(r) = self.view.find.current() {
                        self.view.select(Some(r));
                        self.view.fit_refs(&model, std::iter::once(r));
                    }
                }
            }
        });
        let mut main_ui = crate::widgets::split::child_in(ui, main_rect);
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(&mut main_ui, |ui| {
            let style = app.tokens.diagram_style();
            let settings = app.diagram_settings();
            let model2 = model.clone();
            let out = viewport::Diagram { model: &model, style: &style, settings: &settings }.show(ui, &mut self.view, &mut self.overlays, &|_, _| {}, &mut |ui, target| {
                crate::app::diagram_context_menu(ui, &model2, target, &mut actions);
            });
            app.last_diagram_viewport = out.viewport;
            if out.copied {
                app.toasts.push(crate::widgets::ToastKind::Info, "Element id copied");
            }
            if self.view.find.active() {
                self.dock_tab = 1;
            }
        });
        for a in actions {
            if let InspectorAction::Select(r) = a {
                self.view.select(Some(r));
                self.view.fit_refs(&model, std::iter::once(r));
            }
        }
        app.status_hint = "Drag/wheel: pan & zoom · 0 fit · F fit selection · Tab next element · Ctrl+F find · ? help".into();
    }
}
