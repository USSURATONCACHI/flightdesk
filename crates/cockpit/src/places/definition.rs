//! Definition place: the diagram with live counts, plus instances / incidents / versions docks.

use crate::app::App;
use crate::inspector::{self, InspectorAction, RuntimeFacts};
use crate::net::Loader;
use crate::state::{Nav, NavMode};
use crate::viewport::{self, BadgeKind, DiagramView, Overlays};
use crate::widgets::{badge, filter_box, loader_status_inline, pill, progress_line, state_color, state_label};
use camunda::{ActivityStatistics, HistoricProcessInstance, HistoricProcessInstanceQuery, Incident, ProcessDefinition};
use egui::{Key, Modifiers, RichText};
use egui_extras::{Column, TableBuilder};
use std::sync::Arc;
use std::time::Duration;

pub struct DefinitionPlace {
    pub def_id: String,
    pub def: Loader<ProcessDefinition>,
    pub versions: Loader<Vec<ProcessDefinition>>,
    pub stats: Loader<Vec<ActivityStatistics>>,
    pub instances: Loader<Vec<HistoricProcessInstance>>,
    pub instances_count: Loader<i64>,
    pub incidents: Loader<Vec<Incident>>,
    pub view: DiagramView,
    pub overlays: Overlays,
    pub dock_tab: usize,
    pub activity_filter: Option<String>,
    pub instance_filter: String,
    pub filter_focus: bool,
    pub all_versions: bool,
    pub row_cursor: usize,
    pub page: usize,
    seen_epoch: u64,
    instances_key: String,
}

pub const DOCK_TABS: [&str; 4] = ["Inspector", "Instances", "Incidents", "Versions"];

impl DefinitionPlace {
    pub fn new(def_id: String) -> Self {
        Self {
            view: DiagramView::new(egui::Id::new(("def", &def_id))),
            def_id,
            def: Loader::default(),
            versions: Loader::default(),
            stats: Loader::default(),
            instances: Loader::default(),
            instances_count: Loader::default(),
            incidents: Loader::default(),
            overlays: Overlays::default(),
            dock_tab: 0,
            activity_filter: None,
            instance_filter: String::new(),
            filter_focus: false,
            all_versions: false,
            row_cursor: 0,
            page: 0,
            seen_epoch: 0,
            instances_key: String::new(),
        }
    }

    pub fn title(&self) -> String {
        match &self.def.data {
            Some(d) => format!("{} v{}", crate::util::ellipsize(d.display_name(), 22), d.version),
            None => "definition".into(),
        }
    }

    fn poll(&mut self, app: &mut App) {
        if self.seen_epoch != app.refresh_epoch {
            self.seen_epoch = app.refresh_epoch;
            self.stats.invalidate();
            self.instances.invalidate();
            self.instances_count.invalidate();
            self.incidents.invalidate();
            self.def.invalidate();
        }
        let Some(net) = app.net.as_ref() else { return };
        let id = self.def_id.clone();
        let refresh = Duration::from_secs(app.settings.refresh_list_secs);
        let auto = app.auto_refresh_allowed();
        let due = |l_age: Option<Duration>| l_age.is_none() || auto;
        if self.def.stale(Duration::from_secs(60)) && due(self.def.age()) {
            let id2 = id.clone();
            net.fetch(&mut self.def, move |c| async move { c.process_definition(&id2).await });
        }
        if self.def.poll() {
            if let Some(d) = &self.def.data {
                app.cache.remember_def(d.clone());
            }
        }
        if self.stats.stale(refresh) && due(self.stats.age()) {
            let id2 = id.clone();
            net.fetch(&mut self.stats, move |c| async move { c.activity_statistics(&id2).await });
        }
        if self.stats.poll() {
            self.rebuild_overlays();
        }
        if let Some(d) = &self.def.data {
            if self.versions.stale(Duration::from_secs(30)) && due(self.versions.age()) {
                let key = d.key.clone();
                net.fetch(&mut self.versions, move |c| async move { c.process_definition_versions(&key).await });
            }
        }
        self.versions.poll();
        // Instances list: only when the dock shows it, or when a filter is active.
        if self.dock_tab == 1 || self.activity_filter.is_some() {
            let key = format!("{}|{}|{}|{}|{}", self.def_id, self.all_versions, self.activity_filter.clone().unwrap_or_default(), self.instance_filter.trim(), self.page);
            if key != self.instances_key {
                self.instances_key = key;
                self.instances.invalidate();
                self.instances_count.invalidate();
                self.row_cursor = 0;
            }
            if self.instances.stale(refresh) && due(self.instances.age()) {
                let q = self.instance_query(app);
                let first = self.page * app.settings.page_size;
                let max = app.settings.page_size;
                let q2 = q.clone();
                net.fetch(&mut self.instances, move |c| async move { c.history_process_instances(&q2, first, max).await });
                net.fetch(&mut self.instances_count, move |c| async move { c.history_process_instances_count(&q).await });
            }
        }
        self.instances.poll();
        self.instances_count.poll();
        if self.dock_tab == 2 && self.incidents.stale(refresh) && due(self.incidents.age()) {
            let id2 = id.clone();
            net.fetch(&mut self.incidents, move |c| async move { c.incidents(&[("processDefinitionId", id2)]).await });
        }
        self.incidents.poll();
    }

    fn instance_query(&self, app: &App) -> HistoricProcessInstanceQuery {
        let mut q = HistoricProcessInstanceQuery { unfinished: Some(true), sorting: Some(vec![camunda::Sorting { sort_by: "startTime".into(), sort_order: "desc".into() }]), ..Default::default() };
        if self.all_versions {
            q.process_definition_key = self.def.data.as_ref().map(|d| d.key.clone()).or_else(|| app.cache.def_by_id(&self.def_id).map(|d| d.key.clone()));
            if q.process_definition_key.is_none() {
                q.process_definition_id = Some(self.def_id.clone());
            }
        } else {
            q.process_definition_id = Some(self.def_id.clone());
        }
        if let Some(a) = &self.activity_filter {
            q.active_activity_id_in = Some(vec![a.clone()]);
        }
        let f = self.instance_filter.trim();
        if !f.is_empty() {
            if crate::util::is_uuid(f) {
                q.process_instance_id = Some(f.to_string());
            } else {
                q.process_instance_business_key_like = Some(format!("%{}%", f.replace('*', "%")));
            }
        }
        q
    }

    fn rebuild_overlays(&mut self) {
        self.overlays.running.clear();
        self.overlays.incidents.clear();
        if let Some(stats) = &self.stats.data {
            for s in stats {
                if s.instances > 0 {
                    self.overlays.running.insert(s.id.clone(), s.instances as u32);
                }
                let inc = s.incident_count();
                if inc > 0 {
                    self.overlays.incidents.insert(s.id.clone(), inc as u32);
                }
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        self.poll(app);
        let model = app.diagram(&self.def_id);
        let tokens = app.tokens.clone();

        // ---- toolbar ----
        egui::Panel::top(("def-toolbar", &self.def_id)).frame(egui::Frame::new().fill(tokens.bg1).inner_margin(egui::Margin::symmetric(12, 6))).show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.link(RichText::new("Definitions").weak()).clicked() {
                    app.navigate(Nav::Definitions, NavMode::Here);
                }
                ui.label(RichText::new("›").weak());
                match &self.def.data {
                    Some(d) => {
                        ui.label(RichText::new(d.display_name()).strong().size(15.0));
                        ui.label(RichText::new(&d.key).monospace().weak());
                        // version switcher
                        let versions = self.versions.data.clone().unwrap_or_default();
                        let cur = format!("v{}", d.version);
                        egui::ComboBox::from_id_salt("version-switch").selected_text(cur).width(90.0).show_ui(ui, |ui| {
                            for v in &versions {
                                let selected = v.id == self.def_id;
                                let label = format!("v{}{}", v.version, v.version_tag.as_ref().map(|t| format!(" ({t})")).unwrap_or_default());
                                if ui.selectable_label(selected, label).clicked() && !selected {
                                    app.navigate(Nav::Definition { id: v.id.clone() }, NavMode::Replace);
                                }
                            }
                        });
                        if d.suspended {
                            pill(ui, "suspended", tokens.suspended);
                        }
                        let running: i64 = self.stats.data.as_ref().map(|s| s.iter().map(|x| x.instances).sum()).unwrap_or(0);
                        let incidents: i64 = self.stats.data.as_ref().map(|s| s.iter().map(|x| x.incident_count()).sum()).unwrap_or(0);
                        if badge(ui, &format!("{running} running"), tokens.running).on_hover_text("Show instances").clicked() {
                            self.dock_tab = 1;
                            app.dock_visible = true;
                        }
                        if incidents > 0 && badge(ui, &format!("{incidents} incidents"), tokens.incident).clicked() {
                            self.dock_tab = 2;
                            app.dock_visible = true;
                        }
                    }
                    None => {
                        ui.label(RichText::new(crate::util::short_uuid(&self.def_id)).monospace());
                        if let Some(e) = &self.def.error {
                            ui.label(RichText::new(e.short()).color(tokens.incident));
                        }
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    app.refresh_controls(ui, self.stats.age(), self.stats.is_loading());
                    if ui.button("🔍 Find element").on_hover_text("Ctrl+F").clicked() {
                        self.view.open_find();
                    }
                    if let Some(d) = &self.def.data {
                        if ui.button("Search instances").on_hover_text("Open the instance search for this definition").clicked() {
                            app.navigate(Nav::Search { query: format!("def:{}", d.key) }, NavMode::NewTab);
                        }
                    }
                });
            });
        });

        let Some(model) = model else {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(tokens.bg0)).show(ui, |ui| {
                let r = ui.max_rect();
                progress_line(ui, egui::Rect::from_min_size(r.min, egui::Vec2::new(r.width(), 2.0)), tokens.accent);
                ui.centered_and_justified(|ui| {
                    if let Some(e) = app.cache.diagram_error(&self.def_id) {
                        ui.label(RichText::new(format!("Cannot load diagram: {e}")).color(tokens.incident));
                    } else {
                        ui.label(RichText::new("Loading diagram…").weak());
                    }
                });
            });
            return;
        };

        // ---- dock ----
        let mut actions: Vec<InspectorAction> = vec![];
        let mut dock_tab = self.dock_tab;
        let selection = self.view.selection.or(self.view.hover.filter(|_| ctx.input(|i| i.modifiers.alt)));
        let model_dock = model.clone();
        let mut open_instance: Option<(String, NavMode)> = None;
        let mut select_from_dock: Option<bpmn::Ref> = None;
        super::dock(ui, app, "def-dock", &DOCK_TABS, &mut dock_tab, |ui, tab, app| match tab {
            0 => match selection {
                Some(r) => {
                    let mut facts = RuntimeFacts::default();
                    if let bpmn::Ref::Element(i) = r {
                        let id = &model_dock.elements[i].id;
                        facts.running = Some(self.overlays.running.get(id).copied().unwrap_or(0));
                        facts.incidents = Some(self.overlays.incidents.get(id).copied().unwrap_or(0));
                    }
                    actions = inspector::show(ui, &model_dock, r, &facts, false);
                }
                None => {
                    crate::widgets::empty_state(ui, "Select an element to inspect it.\nTab cycles elements · Ctrl+F finds · hold Alt to inspect on hover.");
                    if let Some(d) = &self.def.data {
                        ui.add_space(16.0);
                        crate::widgets::section(ui, "def-meta", "Definition", true, |ui| {
                            crate::widgets::kv_mono(ui, "id", &d.id);
                            crate::widgets::kv_mono(ui, "key", &d.key);
                            crate::widgets::kv(ui, "version", &format!("{}{}", d.version, d.version_tag.as_ref().map(|t| format!(" ({t})")).unwrap_or_default()));
                            if let Some(r) = &d.resource {
                                crate::widgets::kv(ui, "resource", r);
                            }
                            if let Some(dep) = &d.deployment_id {
                                crate::widgets::kv_mono(ui, "deployment", dep);
                            }
                            if let Some(t) = d.history_time_to_live {
                                crate::widgets::kv(ui, "history TTL", &format!("{t} days"));
                            }
                            if let Some(desc) = &d.description {
                                crate::widgets::kv(ui, "description", desc);
                            }
                            for p in model_dock.executable_processes() {
                                for (k, v) in &p.attrs {
                                    if k.starts_with("camunda:") {
                                        crate::widgets::kv(ui, k.trim_start_matches("camunda:"), v);
                                    }
                                }
                            }
                        });
                    }
                }
            },
            1 => {
                self.instances_dock(ui, app, &model_dock, &mut open_instance, &mut select_from_dock);
            }
            2 => {
                self.incidents_dock(ui, app, &model_dock, &mut open_instance, &mut select_from_dock);
            }
            _ => {
                self.versions_dock(ui, app);
            }
        });
        self.dock_tab = dock_tab;

        // ---- diagram ----
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
            let style = tokens.diagram_style();
            let settings = app.diagram_settings();
            let model2 = model.clone();
            let out = viewport::widget::show(ui, &mut self.view, &model, &mut self.overlays, &style, &settings, &|_, _| {}, &mut |ui, target| {
                crate::app::diagram_context_menu(ui, &model2, target, &mut actions);
            });
            if let Some((kind, id)) = out.badge_clicked {
                match kind {
                    BadgeKind::Running | BadgeKind::Completed | BadgeKind::Timer => {
                        self.activity_filter = Some(id);
                        self.dock_tab = 1;
                        app.dock_visible = true;
                    }
                    BadgeKind::Incident => {
                        self.dock_tab = 2;
                        app.dock_visible = true;
                        if let Some(i) = model.element_idx(&id) {
                            self.view.select(Some(bpmn::Ref::Element(i)));
                        }
                    }
                }
            }
            app.last_diagram_viewport = out.viewport;
            if out.copied {
                app.toasts.push(crate::widgets::ToastKind::Info, "Element id copied");
            }
            if let Some(bpmn::Ref::Element(i)) = out.enter_pressed {
                let e = &model.elements[i];
                if e.kind == bpmn::ElementKind::CallActivity {
                    if let Some(k) = e.attr("calledElement") {
                        if !k.contains("${") {
                            app.navigate(Nav::DefinitionKey { key: k.to_string() }, NavMode::Here);
                        }
                    }
                } else {
                    self.dock_tab = 0;
                    app.dock_visible = true;
                }
            }
            if self.view.fit_tokens_marker {
                self.view.fit_tokens_marker = false;
                let ids: Vec<bpmn::Ref> = self.overlays.running.keys().chain(self.overlays.incidents.keys()).filter_map(|id| model.element_idx(id).map(bpmn::Ref::Element)).collect();
                self.view.fit_refs(&model, ids.into_iter());
            }
            if self.stats.is_loading() && self.stats.data.is_none() {
                let r = ui.max_rect();
                progress_line(ui, egui::Rect::from_min_size(r.min, egui::Vec2::new(r.width(), 2.0)), tokens.accent);
            }
        });

        if let Some(r) = select_from_dock {
            self.view.select(Some(r));
            self.view.fit_refs(&model, std::iter::once(r));
        }
        if let Some((id, mode)) = open_instance {
            app.navigate(Nav::Instance { id }, mode);
        }
        for a in actions {
            match a {
                InspectorAction::Select(r) => {
                    self.view.select(Some(r));
                    self.view.fit_refs(&model, std::iter::once(r));
                }
                InspectorAction::OpenDefinitionKey(k) => app.navigate(Nav::DefinitionKey { key: k }, NavMode::Here),
                InspectorAction::InstancesAt(a) => {
                    self.activity_filter = Some(a);
                    self.dock_tab = 1;
                    app.dock_visible = true;
                }
                InspectorAction::IncidentsAt(_) => {
                    self.dock_tab = 2;
                    app.dock_visible = true;
                }
                InspectorAction::ExecutedAt(a) => {
                    let key = self.def.data.as_ref().map(|d| d.key.clone()).unwrap_or_default();
                    app.navigate(Nav::Search { query: format!("def:{key} exec:{a} state:any") }, NavMode::NewTab);
                }
            }
        }
        app.status_hint = "Wheel zoom · drag pan · Tab next element · Enter inspect / drill into call activity · click a badge to filter instances · Ctrl+F find · ? help".into();
    }

    fn instances_dock(&mut self, ui: &mut egui::Ui, app: &mut App, model: &Arc<bpmn::Model>, open: &mut Option<(String, NavMode)>, select: &mut Option<bpmn::Ref>) {
        let tokens = app.tokens.clone();
        let w = ui.available_width();
        ui.add_sized([w, 24.0], |ui: &mut egui::Ui| filter_box(ui, egui::Id::new(("inst-filter", &self.def_id)), &mut self.instance_filter, "Business key or instance id", &mut self.filter_focus));
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.all_versions, "All versions");
            if let Some(a) = self.activity_filter.clone() {
                let name = model.element_by_id(&a).map(|e| e.display_name().to_string()).unwrap_or(a.clone());
                if pill(ui, &format!("at: {name}  ✕"), tokens.running).on_hover_text("Clear activity filter").clicked() {
                    self.activity_filter = None;
                }
            }
            if let Some(n) = self.instances_count.data {
                ui.label(RichText::new(format!("{n} running")).weak());
            }
            loader_status_inline(ui, &self.instances, &tokens);
        });
        let Some(rows) = self.instances.data.clone() else {
            if self.instances.error.is_none() {
                ui.spinner();
            }
            return;
        };
        if rows.is_empty() {
            crate::widgets::empty_state(ui, "No running instances match.");
            return;
        }
        self.row_cursor = self.row_cursor.min(rows.len() - 1);
        let text_focus = ui.ctx().memory(|m| m.focused()).map(|id| egui::text_edit::TextEditState::load(ui.ctx(), id).is_some()).unwrap_or(false);
        if !text_focus && !self.view.focused {
            ui.input_mut(|i| {
                if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    self.row_cursor = (self.row_cursor + 1).min(rows.len() - 1);
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    self.row_cursor = self.row_cursor.saturating_sub(1);
                }
                if i.consume_key(Modifiers::SHIFT, Key::Enter) {
                    *open = Some((rows[self.row_cursor].id.clone(), NavMode::NewTab));
                } else if i.consume_key(Modifiers::NONE, Key::Enter) {
                    *open = Some((rows[self.row_cursor].id.clone(), NavMode::Here));
                }
                if i.consume_key(Modifiers::NONE, Key::Slash) {
                    self.filter_focus = true;
                }
            });
        }
        let h = ui.available_height() - 30.0;
        let shift = ui.input(|i| i.modifiers.shift);
        TableBuilder::new(ui)
            .striped(true)
            .sense(egui::Sense::click())
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::initial(140.0).at_least(80.0))
            .column(Column::initial(90.0))
            .column(Column::remainder().at_least(80.0))
            .column(Column::initial(70.0))
            .max_scroll_height(h.max(100.0))
            .scroll_to_row(self.row_cursor, None)
            .header(22.0, |mut hr| {
                for t in ["Business key", "Started", "Id", "State"] {
                    hr.col(|ui| {
                        ui.label(RichText::new(t).strong().size(11.5));
                    });
                }
            })
            .body(|body| {
                body.rows(26.0, rows.len(), |mut row| {
                    let i = row.index();
                    let r = &rows[i];
                    row.set_selected(i == self.row_cursor);
                    row.col(|ui| {
                        ui.label(RichText::new(r.business_key.clone().unwrap_or_else(|| "—".into())).size(12.0));
                    });
                    row.col(|ui| {
                        ui.label(RichText::new(r.start_time.as_deref().map(crate::util::fmt_local_short).unwrap_or_default()).size(12.0)).on_hover_text(r.start_time.as_deref().map(crate::util::fmt_local).unwrap_or_default());
                    });
                    row.col(|ui| {
                        ui.label(RichText::new(crate::util::short_uuid(&r.id)).monospace().size(11.5)).on_hover_text(&r.id);
                    });
                    row.col(|ui| {
                        let st = r.state.as_deref().unwrap_or("ACTIVE");
                        ui.label(RichText::new(state_label(st)).color(state_color(st, &tokens)).size(11.5));
                    });
                    let resp = row.response();
                    if resp.clicked() {
                        self.row_cursor = i;
                        *open = Some((r.id.clone(), if shift { NavMode::NewTab } else { NavMode::Here }));
                    }
                    if resp.middle_clicked() {
                        *open = Some((r.id.clone(), NavMode::NewTab));
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Open").clicked() {
                            *open = Some((r.id.clone(), NavMode::Here));
                            ui.close();
                        }
                        if ui.button("Open in new tab").clicked() {
                            *open = Some((r.id.clone(), NavMode::NewTab));
                            ui.close();
                        }
                        if ui.button("Copy id").clicked() {
                            ui.ctx().copy_text(r.id.clone());
                            ui.close();
                        }
                        if let Some(bk) = &r.business_key {
                            if ui.button("Copy business key").clicked() {
                                ui.ctx().copy_text(bk.clone());
                                ui.close();
                            }
                        }
                    });
                });
            });
        let _ = select;
        // paging
        if let Some(n) = self.instances_count.data {
            let ps = app.settings.page_size as i64;
            if n > ps {
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.page > 0, egui::Button::new("‹").small()).clicked() {
                        self.page -= 1;
                    }
                    ui.label(RichText::new(format!("{}–{} of {n}", self.page as i64 * ps + 1, ((self.page as i64 + 1) * ps).min(n))).weak().small());
                    if ui.add_enabled(((self.page as i64 + 1) * ps) < n, egui::Button::new("›").small()).clicked() {
                        self.page += 1;
                    }
                });
            }
        }
    }

    fn incidents_dock(&mut self, ui: &mut egui::Ui, app: &mut App, model: &Arc<bpmn::Model>, open: &mut Option<(String, NavMode)>, select: &mut Option<bpmn::Ref>) {
        let tokens = app.tokens.clone();
        loader_status_inline(ui, &self.incidents, &tokens);
        let Some(list) = self.incidents.data.clone() else {
            if self.incidents.error.is_none() {
                ui.spinner();
            }
            return;
        };
        if list.is_empty() {
            crate::widgets::empty_state(ui, "No open incidents for this version.");
            return;
        }
        let sel_id = match self.view.selection {
            Some(bpmn::Ref::Element(i)) => Some(model.elements[i].id.clone()),
            _ => None,
        };
        for inc in &list {
            let act = inc.activity_id.clone().unwrap_or_default();
            let name = model.element_by_id(&act).map(|e| e.display_name().to_string()).unwrap_or(act.clone());
            let highlighted = sel_id.as_deref() == Some(act.as_str());
            let frame = egui::Frame::new().fill(if highlighted { tokens.accent.gamma_multiply(0.12) } else { tokens.bg2 }).stroke(egui::Stroke::new(1.0, tokens.bg3)).corner_radius(6.0).inner_margin(8.0);
            frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    pill(ui, &inc.incident_type, tokens.incident);
                    if ui.link(RichText::new(&name).strong()).on_hover_text("Select on diagram").clicked() {
                        if let Some(i) = model.element_idx(&act) {
                            *select = Some(bpmn::Ref::Element(i));
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(inc.time().map(crate::util::fmt_relative).unwrap_or_default()).weak().small());
                    });
                });
                ui.add(egui::Label::new(RichText::new(inc.incident_message.clone().unwrap_or_default()).size(12.0)).wrap());
                ui.horizontal(|ui| {
                    if let Some(pi) = &inc.process_instance_id {
                        if ui.link(RichText::new(format!("instance {}", crate::util::short_uuid(pi))).monospace().size(11.5)).clicked() {
                            *open = Some((pi.clone(), if ui.input(|i| i.modifiers.shift) { NavMode::NewTab } else { NavMode::Here }));
                        }
                    }
                    if inc.incident_type == "failedJob" {
                        if let Some(job) = inc.configuration.clone() {
                            if ui.small_button("Retry").on_hover_text("Set job retries to 1").clicked() {
                                app.action_set_job_retries(job, 1);
                            }
                        }
                    }
                    if inc.incident_type == "failedExternalTask" {
                        if let Some(et) = inc.configuration.clone() {
                            if ui.small_button("Retry").on_hover_text("Set external task retries to 1").clicked() {
                                app.action_set_external_task_retries(et, 1);
                            }
                        }
                    }
                });
            });
            ui.add_space(4.0);
        }
    }

    fn versions_dock(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let tokens = app.tokens.clone();
        let Some(list) = self.versions.data.clone() else {
            ui.spinner();
            return;
        };
        let stats = app.cache.def_stats.data.clone().unwrap_or_default();
        egui::Grid::new("versions-grid").num_columns(4).striped(true).spacing([12.0, 6.0]).show(ui, |ui| {
            for t in ["Version", "Running", "Incidents", "Deployed"] {
                ui.label(RichText::new(t).strong().size(11.5));
            }
            ui.end_row();
            for v in &list {
                let s = stats.iter().find(|s| s.id == v.id);
                let cur = v.id == self.def_id;
                let label = format!("v{}{}{}", v.version, v.version_tag.as_ref().map(|t| format!(" ({t})")).unwrap_or_default(), if cur { "  ●" } else { "" });
                if ui.link(RichText::new(label).strong()).clicked() && !cur {
                    app.navigate(Nav::Definition { id: v.id.clone() }, NavMode::Replace);
                }
                let running = s.map(|s| s.instances).unwrap_or(0);
                if running > 0 {
                    badge(ui, &running.to_string(), tokens.running);
                } else {
                    ui.label(RichText::new("0").weak());
                }
                let inc: i64 = s.map(|s| s.incidents.iter().map(|i| i.incident_count).sum()).unwrap_or(0);
                if inc > 0 {
                    badge(ui, &inc.to_string(), tokens.incident);
                } else {
                    ui.label(RichText::new("—").weak());
                }
                ui.label(RichText::new(v.deployment_id.as_deref().map(crate::util::short_uuid).unwrap_or_default()).monospace().size(11.0));
                ui.end_row();
            }
        });
        if let Some(d) = &self.def.data {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let label = if d.suspended { "Activate definition" } else { "Suspend definition" };
                if ui.button(label).clicked() {
                    app.action_suspend_definition(d.id.clone(), !d.suspended);
                }
            });
        }
    }
}
