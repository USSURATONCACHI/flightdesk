//! Offline viewer for a local .bpmn file (also used for screenshots and tests).

use crate::app::App;
use crate::inspector::{self, InspectorAction, RuntimeFacts};
use crate::viewport::{self, DiagramView, Overlays};
use bpmn::Model;
use std::sync::Arc;

/// Dock tab names (also the MCP identifiers).
pub const DOCK_TABS: [&str; 2] = ["Inspector", "Matches"];

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
            app.toasts.push(crate::widgets::ToastKind::Warning, crate::t!("file-diagram-warnings", n = n));
        }
        let Some(model) = self.model.clone() else {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.label(crate::t!("file-open-failed", path = self.path.display().to_string(), error = self.error.clone().unwrap_or_default()));
            });
            return;
        };
        let mut actions = vec![];
        let (main_rect, _) = super::dock(ui, app, "file-dock", &DOCK_TABS, &mut self.dock_tab, |ui, tab, _app| match tab {
            0 => match self.view.selection.or(self.view.hover) {
                Some(r) => actions = inspector::show(ui, &model, r, &RuntimeFacts::default(), false),
                None => {
                    crate::widgets::empty_state(ui, &crate::t!("inspector-empty-file"));
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new(format!("{} flow nodes · {} flows · {} warnings", model.flow_nodes().count(), model.flows.len(), model.warnings.len())).weak());
                    for w in model.warnings.iter().take(20) {
                        ui.label(egui::RichText::new(w).small().weak());
                    }
                    ui.add_space(8.0);
                    for p in &model.processes {
                        crate::widgets::section(ui, ("file-process", &p.id), "Process", true, |ui| {
                            crate::widgets::kv_mono(ui, "id", &p.id);
                            if let Some(n) = &p.name {
                                crate::widgets::kv(ui, "name", n);
                            }
                            crate::widgets::kv(ui, "executable", &if p.is_executable { crate::t!("common-yes") } else { crate::t!("common-no") });
                            for (k, v) in &p.attrs {
                                if let Some(c) = k.strip_prefix("camunda:") {
                                    crate::widgets::kv(ui, c, v);
                                }
                            }
                            if let Some(d) = &p.documentation {
                                crate::widgets::doc_block(ui, ("process-doc", &p.id), "documentation", d);
                            }
                        });
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
            let xml_editor = !app.settings.xml_editor.trim().is_empty();
            let out = viewport::Diagram { model: &model, style: &style, settings: &settings }.show(ui, &mut self.view, &mut self.overlays, &|_, _| {}, &mut |ui, target| {
                crate::app::diagram_context_menu(ui, &model2, target, xml_editor, &mut actions);
            });
            app.last_diagram_viewport = out.viewport;
            if out.copied {
                app.toasts.push(crate::widgets::ToastKind::Info, crate::t!("toast-element-id-copied"));
            }
            if self.view.find.active() {
                self.dock_tab = 1;
            }
        });
        let source = crate::places::source::DocSource::File(self.path.clone());
        for a in actions {
            match a {
                InspectorAction::Select(r) => {
                    self.view.select(Some(r));
                    self.view.fit_refs(&model, std::iter::once(r));
                }
                InspectorAction::FitDiagram => self.view.fit_all(),
                InspectorAction::Compare => app.request_compare(),
                InspectorAction::ViewXml(_) | InspectorAction::XmlExternal(..) => {
                    crate::places::xml::handle_action(app, crate::places::xml::XmlDoc::Single(source.clone()), &[(source.clone(), model.clone())], &a);
                }
                InspectorAction::OpenDefinitionKey(_) | InspectorAction::InstancesAt(_) | InspectorAction::ExecutedAt(_) | InspectorAction::IncidentsAt(_) => {
                    app.toasts.push(crate::widgets::ToastKind::Info, crate::t!("file-needs-engine"));
                }
            }
        }
        app.status_hint = crate::t!("hint-file");
    }
}
