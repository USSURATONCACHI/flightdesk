//! Process instance place: live tokens and history on the diagram, with variables, tasks,
//! incidents, jobs, external tasks, called instances, a timeline and token modification.

use crate::app::App;
use crate::inspector::{self, InspectorAction, RuntimeFacts};
use crate::net::Loader;
use crate::state::{Nav, NavMode};
use crate::viewport::{self, BadgeKind, DiagramView, Overlays, PickMode};
use crate::widgets::{badge, filter_box, kv, kv_link, kv_mono, loader_status_inline, pill, progress_line, section, state_color, state_label, AppAction, Confirm, ToastKind};
use camunda::{ActivityInstance, ExternalTask, HistoricActivityInstance, HistoricProcessInstance, Incident, Job, JobDefinition, ModificationInstruction, ModificationRequest, ProcessInstance, RestartRequest, Task, VariableInstance, VariableValue};
use egui::{Key, Modifiers, RichText};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

pub const DOCK_TABS: [&str; 10] = ["Overview", "Inspect", "Variables", "Tasks", "Incidents", "Jobs", "External", "Called", "Timeline", "Modify"];

pub struct VarEdit {
    pub name: String,
    pub type_: String,
    pub text: String,
    pub is_new: bool,
    pub error: Option<String>,
}

pub struct TraceView {
    pub title: String,
    pub text: String,
    pub filter: String,
}

#[derive(Default)]
pub struct ModState {
    pub instructions: Vec<ModificationInstruction>,
    pub skip_listeners: bool,
    pub skip_io: bool,
    pub annotation: String,
    /// Source activity of a pending "move token" pick.
    pub move_from: Option<String>,
}

pub struct InstancePlace {
    pub id: String,
    pub hist: Loader<HistoricProcessInstance>,
    pub runtime: Loader<ProcessInstance>,
    pub activity: Loader<ActivityInstance>,
    pub history_acts: Loader<Vec<HistoricActivityInstance>>,
    pub variables: Loader<Vec<VariableInstance>>,
    pub tasks: Loader<Vec<Task>>,
    pub incidents: Loader<Vec<Incident>>,
    pub jobs: Loader<Vec<Job>>,
    pub job_defs: Loader<Vec<JobDefinition>>,
    pub external_tasks: Loader<Vec<ExternalTask>>,
    pub called: Loader<Vec<HistoricProcessInstance>>,
    pub ancestors: Loader<Vec<HistoricProcessInstance>>,
    pub view: DiagramView,
    pub overlays: Overlays,
    pub dock_tab: usize,
    pub var_filter: String,
    pub var_focus: bool,
    pub var_expanded: HashSet<String>,
    pub var_edit: Option<VarEdit>,
    pub var_cursor: usize,
    pub trace: Option<TraceView>,
    pub trace_height: f32,
    pub modification: ModState,
    seen_epoch: u64,
    job_defs_for: String,
    assignee_edit: HashMap<String, String>,
}

impl InstancePlace {
    pub fn new(id: String) -> Self {
        Self {
            view: DiagramView::new(egui::Id::unique(("inst", &id))),
            id,
            hist: Loader::default(),
            runtime: Loader::default(),
            activity: Loader::default(),
            history_acts: Loader::default(),
            variables: Loader::default(),
            tasks: Loader::default(),
            incidents: Loader::default(),
            jobs: Loader::default(),
            job_defs: Loader::default(),
            external_tasks: Loader::default(),
            called: Loader::default(),
            ancestors: Loader::default(),
            overlays: Overlays::default(),
            dock_tab: 0,
            var_filter: String::new(),
            var_focus: false,
            var_expanded: HashSet::new(),
            var_edit: None,
            var_cursor: 0,
            trace: None,
            trace_height: 260.0,
            modification: ModState::default(),
            seen_epoch: 0,
            job_defs_for: String::new(),
            assignee_edit: HashMap::new(),
        }
    }

    pub fn title(&self) -> String {
        match &self.hist.data {
            Some(h) => {
                let def = h.process_definition_key.clone().unwrap_or_default();
                match &h.business_key {
                    Some(bk) => format!("{} · {}", crate::util::ellipsize(&def, 14), crate::util::ellipsize(bk, 16)),
                    None => format!("{} · {}", crate::util::ellipsize(&def, 14), crate::util::short_uuid(&self.id)),
                }
            }
            None => format!("inst {}", crate::util::short_uuid(&self.id)),
        }
    }

    fn finished(&self) -> bool {
        self.hist.data.as_ref().map(|h| h.is_finished()).unwrap_or(false)
    }

    fn definition_id(&self) -> Option<String> {
        self.hist.data.as_ref().map(|h| h.process_definition_id.clone())
    }

    fn poll(&mut self, app: &mut App) {
        if self.seen_epoch != app.refresh_epoch {
            self.seen_epoch = app.refresh_epoch;
            for inv in [&mut self.hist as &mut dyn Invalidate, &mut self.runtime, &mut self.activity, &mut self.history_acts, &mut self.variables, &mut self.tasks, &mut self.incidents, &mut self.jobs, &mut self.external_tasks, &mut self.called] {
                inv.invalidate_dyn();
            }
        }
        let Some(net) = app.net.as_ref() else { return };
        let id = self.id.clone();
        let refresh = Duration::from_secs(app.settings.refresh_instance_secs);
        let auto = app.auto_refresh_allowed();
        let due = |age: Option<Duration>| age.is_none() || auto;
        let finished = self.finished();
        let slow = if finished { Duration::from_secs(60) } else { refresh };
        let known = self.hist.data.is_some();

        if self.hist.stale(slow) && due(self.hist.age()) {
            let i = id.clone();
            net.fetch(&mut self.hist, move |c| async move { c.history_process_instance(&i).await });
        }
        if self.hist.poll() {
            if let Some(h) = &self.hist.data {
                if h.super_process_instance_id.is_some() && self.ancestors.is_empty() {
                    let mut cur = h.super_process_instance_id.clone();
                    net.fetch(&mut self.ancestors, move |c| async move {
                        let mut chain = vec![];
                        while let Some(sid) = cur.take() {
                            if chain.len() > 12 {
                                break;
                            }
                            let p = c.history_process_instance(&sid).await?;
                            cur = p.super_process_instance_id.clone();
                            chain.push(p);
                        }
                        Ok(chain)
                    });
                }
            }
        }
        self.ancestors.poll();
        if known && !finished {
            if self.runtime.stale(refresh) && due(self.runtime.age()) {
                let i = id.clone();
                net.fetch(&mut self.runtime, move |c| async move { c.process_instance(&i).await });
            }
            if self.activity.stale(refresh) && due(self.activity.age()) {
                let i = id.clone();
                net.fetch(&mut self.activity, move |c| async move { c.activity_instances(&i).await });
            }
        }
        self.runtime.poll();
        let mut changed = self.activity.poll();
        if known && self.history_acts.stale(slow) && due(self.history_acts.age()) {
            let i = id.clone();
            net.fetch(&mut self.history_acts, move |c| async move { c.history_activity_instances(&i).await });
        }
        changed |= self.history_acts.poll();
        if known && self.incidents.stale(slow) && due(self.incidents.age()) {
            let i = id.clone();
            if finished {
                net.fetch(&mut self.incidents, move |c| async move { c.history_incidents(&i).await });
            } else {
                net.fetch(&mut self.incidents, move |c| async move { c.incidents(&[("processInstanceId", i)]).await });
            }
        }
        changed |= self.incidents.poll();
        if known && !finished && self.jobs.stale(refresh) && due(self.jobs.age()) {
            let i = id.clone();
            net.fetch(&mut self.jobs, move |c| async move { c.jobs(&[("processInstanceId", i)]).await });
        }
        changed |= self.jobs.poll();
        if let Some(def_id) = self.definition_id() {
            if self.job_defs_for != def_id {
                self.job_defs_for = def_id.clone();
                self.job_defs.reset();
                net.fetch(&mut self.job_defs, move |c| async move { c.job_definitions(&def_id).await });
            }
        }
        changed |= self.job_defs.poll();
        if known && self.variables.stale(slow) && due(self.variables.age()) && (self.dock_tab == 2 || self.variables.data.is_none()) {
            let i = id.clone();
            if finished {
                net.fetch(&mut self.variables, move |c| async move { c.history_variables(&i).await });
            } else {
                net.fetch(&mut self.variables, move |c| async move { c.variables(&i).await });
            }
        }
        self.variables.poll();
        if known && !finished && self.tasks.stale(refresh) && due(self.tasks.age()) {
            let i = id.clone();
            net.fetch(&mut self.tasks, move |c| async move { c.tasks(&[("processInstanceId", i)]).await });
        }
        self.tasks.poll();
        if known && !finished && self.external_tasks.stale(refresh) && due(self.external_tasks.age()) {
            let i = id.clone();
            net.fetch(&mut self.external_tasks, move |c| async move { c.external_tasks(&[("processInstanceId", i)]).await });
        }
        self.external_tasks.poll();
        if known && self.called.stale(slow) && due(self.called.age()) && (self.dock_tab == 7 || self.called.data.is_none()) {
            let i = id.clone();
            net.fetch(&mut self.called, move |c| async move { c.called_process_instances(&i).await });
        }
        self.called.poll();
        if changed {
            self.rebuild_overlays(app);
        }
    }

    fn job_activity(&self, j: &Job) -> Option<String> {
        let defs = self.job_defs.data.as_ref()?;
        let d = defs.iter().find(|d| Some(&d.id) == j.job_definition_id.as_ref())?;
        d.activity_id.clone()
    }

    fn rebuild_overlays(&mut self, app: &App) {
        let model = app.cache.diagrams.get(&self.definition_id().unwrap_or_default()).and_then(|l| l.data.clone());
        let timers: Vec<(String, Job)> = self.jobs.data.as_ref().map(|jobs| jobs.iter().filter_map(|j| self.job_activity(j).map(|a| (a, j.clone()))).collect()).unwrap_or_default();
        let o = &mut self.overlays;
        o.running.clear();
        o.active.clear();
        o.incidents.clear();
        o.completed.clear();
        o.canceled.clear();
        o.traversed_flows.clear();
        o.timers.clear();
        if let Some(ai) = &self.activity.data {
            for t in ai.tokens() {
                *o.running.entry(t.activity_id.clone()).or_default() += 1;
                o.active.insert(t.activity_id.clone());
            }
        }
        if let Some(inc) = &self.incidents.data {
            for i in inc {
                if i.end_time.is_some() {
                    continue;
                }
                if let Some(a) = i.activity_id.clone().or(i.failed_activity_id.clone()) {
                    *o.incidents.entry(a).or_default() += 1;
                }
            }
        }
        if let Some(h) = &self.history_acts.data {
            for a in h {
                if a.is_multi_instance_body() {
                    continue;
                }
                if a.end_time.is_some() {
                    *o.completed.entry(a.base_activity_id().to_string()).or_default() += 1;
                }
                if a.canceled {
                    o.canceled.insert(a.base_activity_id().to_string());
                }
            }
            if let Some(m) = &model {
                // Infer traversed sequence flows: for each activity instance B, the predecessor A whose
                // instance ended right before B started.
                let mut sorted: Vec<&HistoricActivityInstance> = h.iter().filter(|a| !a.is_multi_instance_body()).collect();
                sorted.sort_by(|a, b| a.start_time.cmp(&b.start_time));
                let ts = |s: &Option<String>| s.as_deref().and_then(crate::util::parse_time).map(|t| t.timestamp_millis());
                for b in &sorted {
                    let Some(bi) = m.element_idx(b.base_activity_id()) else { continue };
                    let Some(bs) = ts(&b.start_time) else { continue };
                    let mut best: Option<(i64, usize)> = None;
                    for (flow, ai_idx) in m.predecessors(bi) {
                        let aid = &m.elements[ai_idx].id;
                        for a in sorted.iter().filter(|a| a.base_activity_id() == aid) {
                            let Some(ae) = ts(&a.end_time) else { continue };
                            if ae <= bs + 1500 && best.map(|(t, _)| ae > t).unwrap_or(true) {
                                best = Some((ae, flow));
                            }
                        }
                    }
                    if let Some((_, f)) = best {
                        o.traversed_flows.insert(f);
                    }
                }
            }
        }
        for (act, j) in timers {
            if let Some(due) = &j.due_date {
                if let Some(t) = crate::util::parse_time(due) {
                    let overdue = t < chrono::Utc::now() && j.retries > 0;
                    let label = if j.retries == 0 { "failed".to_string() } else { crate::util::relative_from_now(t.with_timezone(&chrono::Utc)) };
                    o.timers.insert(act, (label, overdue || j.retries == 0));
                }
            }
        }
        self.overlays.suspended = self.runtime.data.as_ref().map(|r| r.suspended).unwrap_or(false);
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        self.poll(app);
        let tokens = app.tokens.clone();
        let def_id = self.definition_id();
        let model = def_id.as_ref().and_then(|d| app.diagram(d));
        if model.is_some() && self.overlays.running.is_empty() && self.overlays.completed.is_empty() && (self.activity.data.is_some() || self.history_acts.data.is_some()) {
            self.rebuild_overlays(app);
        }
        if let Some(m) = &model {
            if self.overlays.traversed_flows.is_empty() && self.history_acts.data.as_ref().map(|h| h.len() > 1).unwrap_or(false) {
                let _ = m;
                self.rebuild_overlays(app);
            }
        }
        let finished = self.finished();

        // ---- toolbar ----
        egui::Panel::top(("inst-toolbar", &self.id)).frame(egui::Frame::new().fill(tokens.bg1).inner_margin(egui::Margin::symmetric(12, 6))).show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.link(RichText::new("Definitions").weak()).clicked() {
                    app.navigate(Nav::Definitions, NavMode::Here);
                }
                ui.label(RichText::new("›").weak());
                if let Some(h) = self.hist.data.clone() {
                    let dname = app.cache.def_by_id(&h.process_definition_id).map(|d| d.display_name().to_string()).or(h.process_definition_name.clone()).or(h.process_definition_key.clone()).unwrap_or_default();
                    if ui.link(RichText::new(format!("{dname} v{}", h.process_definition_version.unwrap_or(0))).weak()).on_hover_text("Open definition").clicked() {
                        app.navigate(Nav::Definition { id: h.process_definition_id.clone() }, if ui.input(|i| i.modifiers.shift) { NavMode::NewTab } else { NavMode::Here });
                    }
                    // ancestors
                    if let Some(chain) = self.ancestors.data.clone() {
                        for a in chain.iter().rev() {
                            ui.label(RichText::new("›").weak());
                            let label = a.business_key.clone().unwrap_or_else(|| crate::util::short_uuid(&a.id));
                            if ui.link(RichText::new(format!("↑ {label}")).weak()).on_hover_text(format!("Super instance {}", a.id)).clicked() {
                                app.navigate(Nav::Instance { id: a.id.clone() }, NavMode::Here);
                            }
                        }
                    }
                    ui.label(RichText::new("›").weak());
                    let title = h.business_key.clone().unwrap_or_else(|| crate::util::short_uuid(&self.id));
                    let r = ui.add(egui::Label::new(RichText::new(&title).strong().size(15.0)).sense(egui::Sense::click())).on_hover_text("Click to copy business key");
                    if r.clicked() {
                        ctx.copy_text(title.clone());
                    }
                    let r = ui.add(egui::Label::new(RichText::new(crate::util::short_uuid(&self.id)).monospace().weak()).sense(egui::Sense::click())).on_hover_text(format!("{}\nClick to copy", self.id));
                    if r.clicked() {
                        ctx.copy_text(self.id.clone());
                        app.toasts.push(ToastKind::Info, "Instance id copied");
                    }
                    let st = h.state.clone().unwrap_or_else(|| "ACTIVE".into());
                    let st = if self.runtime.data.as_ref().map(|r| r.suspended).unwrap_or(false) { "SUSPENDED".to_string() } else { st };
                    pill(ui, state_label(&st), state_color(&st, &tokens));
                    let n_inc = self.overlays.incidents.values().sum::<u32>();
                    if n_inc > 0 && badge(ui, &format!("{n_inc} incidents"), tokens.incident).clicked() {
                        self.dock_tab = 4;
                        app.dock_visible = true;
                    }
                    let n_tok = self.overlays.running.values().sum::<u32>();
                    if !finished && n_tok > 0 {
                        badge(ui, &format!("{n_tok} token{}", if n_tok == 1 { "" } else { "s" }), tokens.running).on_hover_text("Shift+F fits the camera to the tokens");
                    }
                } else {
                    ui.label(RichText::new(crate::util::short_uuid(&self.id)).monospace());
                    if let Some(e) = &self.hist.error {
                        ui.label(RichText::new(if e.is_not_found() { "Instance not found (deleted, or history removed)".to_string() } else { e.short() }).color(tokens.incident));
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let age = self.activity.age().or(self.hist.age());
                    app.refresh_controls(ui, age, self.activity.is_loading() || self.hist.is_loading());
                    if ui.button("🔍 Find").on_hover_text("Find element (Ctrl+F)").clicked() {
                        self.view.open_find();
                    }
                    self.action_buttons(ui, app, finished);
                });
            });
        });

        let Some(model) = model else {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(tokens.bg0)).show(ui, |ui| {
                let r = ui.max_rect();
                progress_line(ui, egui::Rect::from_min_size(r.min, egui::Vec2::new(r.width(), 2.0)), tokens.accent);
                ui.centered_and_justified(|ui| {
                    if let Some(e) = &self.hist.error {
                        ui.label(RichText::new(e.short()).color(tokens.incident));
                    } else {
                        ui.label(RichText::new("Loading instance…").weak());
                    }
                });
            });
            return;
        };

        // ---- dock ----
        let mut actions: Vec<InspectorAction> = vec![];
        let mut dock_tab = self.dock_tab;
        let mut select_from_dock: Option<bpmn::Ref> = None;
        let mut open: Option<(Nav, NavMode)> = None;
        let model_dock = model.clone();
        let selection = self.view.selection.or(self.view.hover.filter(|_| ctx.input(|i| i.modifiers.alt)));
        let (mut main_rect, _) = super::dock(ui, app, "inst-dock", &DOCK_TABS, &mut dock_tab, |ui, tab, app| match tab {
            0 => self.overview_dock(ui, app, &model_dock, &mut open),
            1 => match selection {
                Some(r) => {
                    let facts = self.facts_for(&model_dock, r);
                    actions = inspector::show(ui, &model_dock, r, &facts, true);
                }
                None => crate::widgets::empty_state(ui, "Select an element to inspect it.\nTab cycles elements · hold Alt to inspect on hover."),
            },
            2 => self.variables_dock(ui, app, &model_dock),
            3 => self.tasks_dock(ui, app, &model_dock, &mut select_from_dock),
            4 => self.incidents_dock(ui, app, &model_dock, &mut select_from_dock, &mut open),
            5 => self.jobs_dock(ui, app, &model_dock, &mut select_from_dock),
            6 => self.external_dock(ui, app, &model_dock, &mut select_from_dock),
            7 => self.called_dock(ui, app, &model_dock, &mut open),
            8 => self.timeline_dock(ui, app, &model_dock, &mut select_from_dock),
            _ => self.modify_dock(ui, app, &model_dock, &mut select_from_dock),
        });
        self.dock_tab = dock_tab;

        // ---- trace pane (bottom split of the main area) ----
        let mut main_ui = crate::widgets::split::child_in(ui, main_rect);
        if self.trace.is_some() {
            let sp = crate::widgets::split::split(&mut main_ui, egui::Id::unique(("trace-split", &self.id)), crate::widgets::split::Side::Bottom, &mut self.trace_height, 120.0, 0.7);
            main_rect = sp.main;
            main_ui.painter().rect_filled(sp.pane, 0.0, tokens.bg1);
            let mut pane = crate::widgets::split::child_in(&mut main_ui, sp.pane);
            let mut close = false;
            egui::Frame::new().inner_margin(8.0).show(&mut pane, |ui| {
                let t = self.trace.as_mut().unwrap();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&t.title).strong());
                    ui.add(egui::TextEdit::singleline(&mut t.filter).hint_text("filter lines").desired_width(200.0));
                    if ui.small_button("Copy").clicked() {
                        ctx.copy_text(t.text.clone());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            close = true;
                        }
                    });
                });
                let w = ui.available_width();
                egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_min_width(w);
                    let f = t.filter.to_lowercase();
                    let text: String = if f.is_empty() { t.text.clone() } else { t.text.lines().filter(|l| l.to_lowercase().contains(&f)).collect::<Vec<_>>().join("\n") };
                    ui.add(egui::Label::new(RichText::new(text).monospace().size(11.5)).extend());
                });
            });
            if close || ctx.input(|i| i.key_pressed(Key::Escape)) && !self.view.focused {
                self.trace = None;
            }
        }
        let mut main_ui = crate::widgets::split::child_in(&mut main_ui, main_rect);

        // ---- diagram ----
        let tasks = self.tasks.data.clone().unwrap_or_default();
        let jobs = self.jobs.data.clone().unwrap_or_default();
        let job_acts: HashMap<String, String> = jobs.iter().filter_map(|j| self.job_activity(j).map(|a| (j.id.clone(), a))).collect();
        let ext = self.external_tasks.data.clone().unwrap_or_default();
        let extra = move |ui: &mut egui::Ui, act: &str| {
            for t in tasks.iter().filter(|t| t.task_definition_key.as_deref() == Some(act)) {
                ui.label(format!("task: {}{}", t.assignee.clone().map(|a| format!("assigned to {a}")).unwrap_or_else(|| "unassigned".into()), t.due.as_deref().map(|d| format!(" · due {}", crate::util::fmt_relative(d))).unwrap_or_default()));
            }
            for j in jobs.iter().filter(|j| job_acts.get(&j.id).map(|a| a == act).unwrap_or(false)) {
                ui.label(format!("job: due {} · retries {}", j.due_date.as_deref().map(crate::util::fmt_relative).unwrap_or_else(|| "now".into()), j.retries));
            }
            for e in ext.iter().filter(|e| e.activity_id.as_deref() == Some(act)) {
                ui.label(format!("external: topic {} · {}", e.topic_name.clone().unwrap_or_default(), e.worker_id.clone().map(|w| format!("locked by {w}")).unwrap_or_else(|| "unlocked".into())));
            }
        };
        let mut mod_actions: Vec<ModAction> = vec![];
        let overlays_running = self.overlays.running.clone();
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(&mut main_ui, |ui| {
            let style = tokens.diagram_style();
            let settings = app.diagram_settings();
            let model2 = model.clone();
            let out = viewport::Diagram { model: &model, style: &style, settings: &settings }.show(ui, &mut self.view, &mut self.overlays, &extra, &mut |ui, target| {
                crate::app::diagram_context_menu(ui, &model2, target, &mut actions);
                if let Some(bpmn::Ref::Element(i)) = target {
                    let e = &model2.elements[i];
                    if e.kind.is_flow_node() && !matches!(e.kind, bpmn::ElementKind::BoundaryEvent | bpmn::ElementKind::StartEvent) {
                        ui.separator();
                        ui.label(RichText::new("Modify").weak());
                        if ui.button("Start token before").clicked() {
                            mod_actions.push(ModAction::StartBefore(e.id.clone()));
                            ui.close();
                        }
                        if ui.button("Start token after").clicked() {
                            mod_actions.push(ModAction::StartAfter(e.id.clone()));
                            ui.close();
                        }
                        if overlays_running.contains_key(&e.id) {
                            if ui.button("Cancel tokens here").clicked() {
                                mod_actions.push(ModAction::Cancel(e.id.clone()));
                                ui.close();
                            }
                            if ui.button("Move token from here to…").clicked() {
                                mod_actions.push(ModAction::MoveFrom(e.id.clone()));
                                ui.close();
                            }
                        }
                    }
                }
            });
            if let Some((kind, id)) = out.badge_clicked {
                match kind {
                    BadgeKind::Incident => {
                        self.dock_tab = 4;
                        app.dock_visible = true;
                    }
                    BadgeKind::Timer => {
                        self.dock_tab = 5;
                        app.dock_visible = true;
                    }
                    BadgeKind::Running => {
                        self.dock_tab = if self.tasks.data.as_ref().map(|t| t.iter().any(|t| t.task_definition_key.as_deref() == Some(&id))).unwrap_or(false) { 3 } else { 1 };
                        app.dock_visible = true;
                    }
                    BadgeKind::Completed => {
                        self.dock_tab = 8;
                        app.dock_visible = true;
                    }
                }
                if let Some(i) = model.element_idx(&id) {
                    self.view.select(Some(bpmn::Ref::Element(i)));
                }
            }
            if let Some(i) = out.picked {
                let target = model.elements[i].id.clone();
                mod_actions.push(ModAction::Picked(target));
            }
            app.last_diagram_viewport = out.viewport;
            if out.copied {
                app.toasts.push(ToastKind::Info, "Element id copied");
            }
            if let Some(bpmn::Ref::Element(i)) = out.enter_pressed {
                let e = &model.elements[i];
                if e.kind == bpmn::ElementKind::CallActivity {
                    // drill into the running called instance at this activity, else the definition
                    let called = self.called.data.as_ref().and_then(|c| {
                        let acts = self.history_acts.data.as_ref()?;
                        let act = acts.iter().find(|a| a.base_activity_id() == e.id && a.end_time.is_none() && a.called_process_instance_id.is_some())?;
                        c.iter().find(|p| Some(&p.id) == act.called_process_instance_id.as_ref()).map(|p| p.id.clone())
                    });
                    if let Some(pid) = called {
                        app.navigate(Nav::Instance { id: pid }, NavMode::Here);
                    } else if let Some(k) = e.attr("calledElement") {
                        if !k.contains("${") {
                            app.navigate(Nav::DefinitionKey { key: k.to_string() }, NavMode::Here);
                        }
                    }
                } else {
                    self.dock_tab = 1;
                    app.dock_visible = true;
                }
            }
            if self.view.fit_tokens_marker {
                self.view.fit_tokens_marker = false;
                let ids: Vec<bpmn::Ref> = self.overlays.active.iter().chain(self.overlays.incidents.keys()).filter_map(|id| model.element_idx(id).map(bpmn::Ref::Element)).collect();
                self.view.fit_refs(&model, ids.into_iter());
            }
            if (self.activity.is_loading() && self.activity.data.is_none()) || (self.history_acts.is_loading() && self.history_acts.data.is_none()) {
                let r = ui.max_rect();
                progress_line(ui, egui::Rect::from_min_size(r.min, egui::Vec2::new(r.width(), 2.0)), tokens.accent);
            }
        });

        for a in mod_actions {
            self.apply_mod_action(a, app, &model);
        }
        if let Some(r) = select_from_dock {
            self.view.select(Some(r));
            self.view.fit_refs(&model, std::iter::once(r));
        }
        if let Some((nav, mode)) = open {
            app.navigate(nav, mode);
        }
        for a in actions {
            match a {
                InspectorAction::Select(r) => {
                    if matches!(r, bpmn::Ref::Element(usize::MAX)) {
                        self.view.fit_all();
                    } else {
                        self.view.select(Some(r));
                        self.view.fit_refs(&model, std::iter::once(r));
                    }
                }
                InspectorAction::OpenDefinitionKey(k) => app.navigate(Nav::DefinitionKey { key: k }, NavMode::Here),
                InspectorAction::InstancesAt(a) => {
                    if let Some(d) = &def_id {
                        let key = app.cache.def_by_id(d).map(|d| d.key.clone()).unwrap_or_default();
                        app.navigate(Nav::Search { query: format!("def:{key} act:{a}") }, NavMode::NewTab);
                    }
                }
                InspectorAction::IncidentsAt(_) => {
                    self.dock_tab = 4;
                    app.dock_visible = true;
                }
                InspectorAction::ExecutedAt(a) => {
                    if let Some(d) = &def_id {
                        let key = app.cache.def_by_id(d).map(|d| d.key.clone()).unwrap_or_default();
                        app.navigate(Nav::Search { query: format!("def:{key} exec:{a} state:any") }, NavMode::NewTab);
                    }
                }
            }
        }
        app.status_hint = if self.view.pick.is_some() {
            "Pick a target element on the diagram · Esc cancels".into()
        } else {
            "Shift+F fit tokens · Enter inspect / drill into called instance · right-click: modify tokens · Ctrl+1…9 dock tabs · ? help".into()
        };
    }

    fn facts_for(&self, model: &bpmn::Model, r: bpmn::Ref) -> RuntimeFacts {
        let mut f = RuntimeFacts::default();
        let bpmn::Ref::Element(i) = r else { return f };
        let id = &model.elements[i].id;
        f.running = Some(self.overlays.running.get(id).copied().unwrap_or(0));
        f.incidents = Some(self.overlays.incidents.get(id).copied().unwrap_or(0));
        f.executed = self.overlays.completed.get(id).copied();
        if let Some(ai) = &self.activity.data {
            for t in ai.tokens().iter().filter(|t| &t.activity_id == id) {
                f.rows.push((if t.is_transition { "async continuation".into() } else { "token".into() }, camunda::short_id(&t.id)));
                if !t.path.is_empty() {
                    f.rows.push(("scope".into(), t.path.join(" › ")));
                }
            }
        }
        if let Some(tasks) = &self.tasks.data {
            for t in tasks.iter().filter(|t| t.task_definition_key.as_deref() == Some(id)) {
                f.rows.push(("assignee".into(), t.assignee.clone().unwrap_or_else(|| "—".into())));
                if let Some(d) = &t.due {
                    f.rows.push(("due".into(), format!("{} ({})", crate::util::fmt_local(d), crate::util::fmt_relative(d))));
                }
                f.rows.push(("task id".into(), t.id.clone()));
            }
        }
        if let Some(jobs) = &self.jobs.data {
            for j in jobs.iter().filter(|j| self.job_activity(j).as_deref() == Some(id)) {
                f.rows.push(("job due".into(), j.due_date.as_deref().map(|d| format!("{} ({})", crate::util::fmt_local(d), crate::util::fmt_relative(d))).unwrap_or_else(|| "asap".into())));
                f.rows.push(("job retries".into(), j.retries.to_string()));
                if let Some(e) = &j.exception_message {
                    f.rows.push(("job error".into(), crate::util::ellipsize(e, 200)));
                }
            }
        }
        if let Some(ext) = &self.external_tasks.data {
            for e in ext.iter().filter(|e| e.activity_id.as_deref() == Some(id)) {
                f.rows.push(("topic".into(), e.topic_name.clone().unwrap_or_default()));
                f.rows.push(("worker".into(), e.worker_id.clone().unwrap_or_else(|| "— (unlocked)".into())));
                if let Some(r) = e.retries {
                    f.rows.push(("retries".into(), r.to_string()));
                }
                if let Some(m) = &e.error_message {
                    f.rows.push(("error".into(), crate::util::ellipsize(m, 200)));
                }
            }
        }
        if let Some(inc) = &self.incidents.data {
            for i in inc.iter().filter(|x| x.activity_id.as_deref() == Some(id) && x.end_time.is_none()) {
                f.rows.push((format!("incident ({})", i.incident_type), crate::util::ellipsize(i.incident_message.as_deref().unwrap_or(""), 200)));
            }
        }
        if let Some(h) = &self.history_acts.data {
            let mut visits: Vec<&HistoricActivityInstance> = h.iter().filter(|a| a.base_activity_id() == id && !a.is_multi_instance_body()).collect();
            visits.sort_by(|a, b| b.start_time.cmp(&a.start_time));
            for v in visits.iter().take(3) {
                let dur = v.duration_in_millis.map(crate::util::human_millis).unwrap_or_else(|| "running".into());
                f.rows.push(("visited".into(), format!("{} · {}{}", v.start_time.as_deref().map(crate::util::fmt_local_short).unwrap_or_default(), dur, if v.canceled { " · cancelled" } else { "" })));
            }
        }
        f
    }

    fn action_buttons(&mut self, ui: &mut egui::Ui, app: &mut App, finished: bool) {
        let id = self.id.clone();
        if finished {
            if ui.button("Restart…").on_hover_text("Start a new instance from the history of this one (Modify tab)").clicked() {
                self.dock_tab = 9;
                app.dock_visible = true;
            }
            return;
        }
        let suspended = self.runtime.data.as_ref().map(|r| r.suspended).unwrap_or(false);
        let menu = ui.button("Actions ▾");
        egui::Popup::menu(&menu).show(|ui| {
            if ui.button(if suspended { "Activate instance" } else { "Suspend instance" }).clicked() {
                let i = id.clone();
                app.run_action(move |c| async move { c.suspend_instance(&i, !suspended).await }, if suspended { "Instance activated" } else { "Instance suspended" }, "Could not change suspension");
            }
            let failed: Vec<String> = self.jobs.data.as_ref().map(|j| j.iter().filter(|j| j.retries == 0).map(|j| j.id.clone()).collect()).unwrap_or_default();
            if ui.add_enabled(!failed.is_empty(), egui::Button::new(format!("Retry {} failed job{}", failed.len(), if failed.len() == 1 { "" } else { "s" }))).clicked() {
                for j in failed {
                    app.action_set_job_retries(j, 1);
                }
            }
            let failed_ext: Vec<String> = self.external_tasks.data.as_ref().map(|e| e.iter().filter(|e| e.retries == Some(0)).map(|e| e.id.clone()).collect()).unwrap_or_default();
            if !failed_ext.is_empty() && ui.button(format!("Retry {} failed external task(s)", failed_ext.len())).clicked() {
                for e in failed_ext {
                    app.action_set_external_task_retries(e, 1);
                }
            }
            if ui.button("Modify tokens…").clicked() {
                self.dock_tab = 9;
                app.dock_visible = true;
            }
            if ui.button("Copy Cockpit link").clicked() {
                app.copy_cockpit_link();
            }
            ui.separator();
            if ui.button(RichText::new("Delete instance…").color(app.tokens.incident)).clicked() {
                let bk = self.hist.data.as_ref().and_then(|h| h.business_key.clone()).unwrap_or_default();
                let i = id.clone();
                app.confirm = Some(Confirm::new("Delete process instance", format!("Delete instance {} {}?\nAll its tokens, tasks and jobs are removed. This cannot be undone.", crate::util::short_uuid(&i), if bk.is_empty() { String::new() } else { format!("(business key {bk})") }), "Delete", move |app: &mut App| {
                    let i2 = i.clone();
                    app.run_action(move |c| async move { c.delete_instance(&i2, Some("deleted via Flightdeck"), false, false, false).await }, "Instance deleted", "Could not delete instance");
                }));
            }
        });
    }

    fn overview_dock(&mut self, ui: &mut egui::Ui, app: &mut App, model: &Arc<bpmn::Model>, open: &mut Option<(Nav, NavMode)>) {
        let tokens = app.tokens.clone();
        loader_status_inline(ui, &self.hist, &tokens);
        let Some(h) = self.hist.data.clone() else { return };
        section(ui, "ov-id", "Instance", true, |ui| {
            kv_mono(ui, "id", &h.id);
            if let Some(bk) = &h.business_key {
                kv(ui, "business key", bk);
            }
            let dname = app.cache.def_by_id(&h.process_definition_id).map(|d| d.display_name().to_string()).or(h.process_definition_name.clone()).unwrap_or_default();
            if kv_link(ui, "definition", &format!("{dname} v{}", h.process_definition_version.unwrap_or(0))) {
                *open = Some((Nav::Definition { id: h.process_definition_id.clone() }, NavMode::Here));
            }
            kv_mono(ui, "definition id", &h.process_definition_id);
            let st = if self.runtime.data.as_ref().map(|r| r.suspended).unwrap_or(false) { "SUSPENDED".to_string() } else { h.state.clone().unwrap_or_else(|| "ACTIVE".into()) };
            kv(ui, "state", state_label(&st));
            if let Some(s) = &h.start_time {
                kv(ui, "started", &format!("{}  ({})", crate::util::fmt_local(s), crate::util::fmt_relative(s)));
            }
            if let Some(e) = &h.end_time {
                kv(ui, "ended", &format!("{}  ({})", crate::util::fmt_local(e), crate::util::fmt_relative(e)));
            }
            if let Some(d) = h.duration_in_millis {
                kv(ui, "duration", &crate::util::human_millis(d));
            } else if let Some(s) = h.start_time.as_deref().and_then(crate::util::parse_time) {
                kv(ui, "running for", &crate::util::human_secs(chrono::Utc::now().signed_duration_since(s).num_seconds()));
            }
            if let Some(u) = &h.start_user_id {
                kv(ui, "started by", u);
            }
            if let Some(a) = &h.start_activity_id {
                kv_mono(ui, "start activity", a);
            }
            if let Some(r) = &h.delete_reason {
                kv(ui, "delete reason", r);
            }
            if let Some(s) = &h.super_process_instance_id {
                if kv_link(ui, "super instance", &crate::util::short_uuid(s)) {
                    *open = Some((Nav::Instance { id: s.clone() }, NavMode::Here));
                }
            }
            if let Some(r) = &h.root_process_instance_id {
                if r != &h.id && kv_link(ui, "root instance", &crate::util::short_uuid(r)) {
                    *open = Some((Nav::Instance { id: r.clone() }, NavMode::Here));
                }
            }
            if let Some(t) = &h.tenant_id {
                kv(ui, "tenant", t);
            }
            if let Some(rt) = &h.removal_time {
                kv(ui, "history removal", &crate::util::fmt_local(rt));
            }
        });
        section(ui, "ov-now", "Now", true, |ui| {
            if let Some(ai) = &self.activity.data {
                let toks = ai.tokens();
                if toks.is_empty() {
                    ui.label(RichText::new("No active tokens").weak());
                }
                for t in toks {
                    let name = model.element_by_id(&t.activity_id).map(|e| e.display_name().to_string()).unwrap_or(t.activity_id.clone());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(if t.is_transition { "⏵" } else { "●" }).color(tokens.running));
                        if ui.link(&name).clicked() {
                            if let Some(i) = model.element_idx(&t.activity_id) {
                                self.view.select(Some(bpmn::Ref::Element(i)));
                                self.view.fit_element(model, i);
                            }
                        }
                        ui.label(RichText::new(t.activity_type.clone().unwrap_or_default()).weak().small());
                        if !t.path.is_empty() {
                            ui.label(RichText::new(format!("in {}", t.path.join(" › "))).weak().small());
                        }
                    });
                }
            } else if self.finished() {
                ui.label(RichText::new("Finished — no tokens").weak());
            }
            ui.horizontal_wrapped(|ui| {
                let cnt = |n: Option<usize>| n.map(|n| n.to_string()).unwrap_or_else(|| "…".into());
                let n = |l: &Loader<Vec<Task>>| l.data.as_ref().map(|v| v.len());
                if ui.add(egui::Button::new(format!("{} user tasks", cnt(n(&self.tasks)))).small()).clicked() {
                    self.dock_tab = 3;
                }
                let inc = self.incidents.data.as_ref().map(|v| v.iter().filter(|i| i.end_time.is_none()).count()).unwrap_or(0);
                if ui.add(egui::Button::new(RichText::new(format!("{inc} incidents")).color(if inc > 0 { tokens.incident } else { tokens.fg1 })).small()).clicked() {
                    self.dock_tab = 4;
                }
                if ui.add(egui::Button::new(format!("{} jobs", cnt(self.jobs.data.as_ref().map(|v| v.len())))).small()).clicked() {
                    self.dock_tab = 5;
                }
                if ui.add(egui::Button::new(format!("{} external", cnt(self.external_tasks.data.as_ref().map(|v| v.len())))).small()).clicked() {
                    self.dock_tab = 6;
                }
                if ui.add(egui::Button::new(format!("{} variables", cnt(self.variables.data.as_ref().map(|v| v.len())))).small()).clicked() {
                    self.dock_tab = 2;
                }
            });
        });
    }

    // ---------- variables ----------

    fn scope_name(&self, model: &bpmn::Model, v: &VariableInstance) -> String {
        if v.execution_id.as_deref() == Some(self.id.as_str()) || v.execution_id.is_none() {
            return "process".into();
        }
        if let Some(ai) = &self.activity.data {
            for a in ai.all() {
                if a.execution_ids.iter().any(|e| Some(e) == v.execution_id.as_ref()) {
                    if a.activity_id == a.process_definition_id || a.parent_activity_instance_id.is_none() {
                        return "process".into();
                    }
                    return model.element_by_id(&a.activity_id).map(|e| e.display_name().to_string()).unwrap_or(a.activity_id.clone());
                }
            }
        }
        if let Some(a) = &v.activity_instance_id {
            let act = a.split(':').next().unwrap_or("");
            if let Some(e) = model.element_by_id(act) {
                return e.display_name().to_string();
            }
        }
        if let Some(t) = &v.task_id {
            return format!("task {}", crate::util::short_uuid(t));
        }
        format!("execution {}", v.execution_id.as_deref().map(crate::util::short_uuid).unwrap_or_default())
    }

    fn variables_dock(&mut self, ui: &mut egui::Ui, app: &mut App, model: &Arc<bpmn::Model>) {
        let tokens = app.tokens.clone();
        let finished = self.finished();
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 170.0).max(120.0);
            ui.add_sized([w, 24.0], |ui: &mut egui::Ui| filter_box(ui, egui::Id::unique(("var-filter", &self.id)), &mut self.var_filter, "Filter name or value", &mut self.var_focus));
            if !finished && ui.button("+ Add").clicked() {
                self.var_edit = Some(VarEdit { name: String::new(), type_: "String".into(), text: String::new(), is_new: true, error: None });
            }
            if ui.button("Copy JSON").on_hover_text("Copy all variables as a JSON object").clicked() {
                if let Some(vars) = &self.variables.data {
                    let mut m = serde_json::Map::new();
                    for v in vars {
                        m.insert(v.name.clone(), v.value.clone());
                    }
                    ui.ctx().copy_text(serde_json::to_string_pretty(&serde_json::Value::Object(m)).unwrap_or_default());
                    app.toasts.push(ToastKind::Info, "Variables copied as JSON");
                }
            }
            loader_status_inline(ui, &self.variables, &tokens);
        });
        if let Some(e) = &self.variables.error {
            ui.label(RichText::new(e.short()).color(tokens.incident));
        }
        // editor
        if let Some(ed) = &mut self.var_edit {
            let mut save = false;
            let mut cancel = false;
            egui::Frame::new().fill(tokens.bg2).stroke(egui::Stroke::new(1.0, tokens.accent)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    if ed.is_new {
                        let r = ui.add(egui::TextEdit::singleline(&mut ed.name).hint_text("name").desired_width(160.0));
                        if ed.name.is_empty() {
                            r.request_focus();
                        }
                    } else {
                        ui.label(RichText::new(&ed.name).strong());
                    }
                    egui::ComboBox::from_id_salt("var-type").selected_text(ed.type_.clone()).width(100.0).show_ui(ui, |ui| {
                        for t in ["String", "Integer", "Long", "Double", "Boolean", "Date", "Json", "Xml", "Object", "Null"] {
                            ui.selectable_value(&mut ed.type_, t.to_string(), t);
                        }
                    });
                });
                let rows = if matches!(ed.type_.as_str(), "Json" | "Xml" | "Object") || ed.text.len() > 60 { 8 } else { 1 };
                let r = ui.add(egui::TextEdit::multiline(&mut ed.text).code_editor().desired_rows(rows).desired_width(f32::INFINITY));
                if !ed.is_new && ed.error.is_none() && ui.input(|i| i.time) < 1e12 && !r.has_focus() && ed.text.is_empty() {
                    // nothing
                }
                if let Some(e) = &ed.error {
                    ui.label(RichText::new(e).color(tokens.incident).small());
                }
                ui.horizontal(|ui| {
                    if ui.button("Save  (Ctrl+Enter)").clicked() || (r.has_focus() && ui.input(|i| i.modifiers.command && i.key_pressed(Key::Enter))) {
                        save = true;
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                        cancel = true;
                    }
                    if ed.type_ == "Date" {
                        ui.label(RichText::new("format: 2026-10-20T10:00:00.000+0300").weak().small());
                    }
                });
            });
            if cancel {
                self.var_edit = None;
            } else if save {
                match build_value(&ed.type_, &ed.text) {
                    Ok(val) => {
                        let name = ed.name.trim().to_string();
                        if name.is_empty() {
                            ed.error = Some("Name is required".into());
                        } else {
                            let pid = self.id.clone();
                            let old = self.variables.data.as_ref().and_then(|v| v.iter().find(|v| v.name == name)).map(|v| v.as_value());
                            let n2 = name.clone();
                            let v2 = val.clone();
                            let Some(net) = &app.net else { return };
                            net.run(
                                move |c| async move { c.put_process_variable(&pid, &n2, &v2).await },
                                move |app, r| match r {
                                    Ok(()) => {
                                        let undo: Option<AppAction> = old.map(|o| {
                                            let pid2 = app_pid(app);
                                            let n3 = name.clone();
                                            Box::new(move |app: &mut App| {
                                                let n4 = n3.clone();
                                                app.run_action(move |c| async move { c.put_process_variable(&pid2, &n4, &o).await }, format!("Variable {n3} restored"), "Undo failed");
                                            }) as AppAction
                                        });
                                        app.toasts.push_full(ToastKind::Success, format!("Variable {name} saved"), None, undo);
                                        app.bump_refresh();
                                    }
                                    Err(e) => {
                                        app.toasts.error(format!("Could not save variable {name}"), Some(e.to_string()));
                                    }
                                },
                            );
                            self.var_edit = None;
                        }
                    }
                    Err(e) => ed.error = Some(e),
                }
            }
        }
        let Some(vars) = self.variables.data.clone() else {
            if self.variables.error.is_none() {
                ui.spinner();
            }
            return;
        };
        let f = self.var_filter.to_lowercase();
        let rows: Vec<&VariableInstance> = vars.iter().filter(|v| f.is_empty() || v.name.to_lowercase().contains(&f) || v.value.to_string().to_lowercase().contains(&f) || v.type_.as_deref().unwrap_or("").to_lowercase().contains(&f)).collect();
        if rows.is_empty() {
            crate::widgets::empty_state(ui, if vars.is_empty() { "No variables." } else { "No variable matches the filter." });
            return;
        }
        ui.label(RichText::new(format!("{} of {}", rows.len(), vars.len())).weak().small());
        self.var_cursor = self.var_cursor.min(rows.len() - 1);
        let text_focus = ui.ctx().memory(|m| m.focused()).map(|id| egui::text_edit::TextEditState::load(ui.ctx(), id).is_some()).unwrap_or(false);
        if !text_focus && !self.view.focused && self.var_edit.is_none() {
            ui.input_mut(|i| {
                if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    self.var_cursor = (self.var_cursor + 1).min(rows.len() - 1);
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    self.var_cursor = self.var_cursor.saturating_sub(1);
                }
                if i.consume_key(Modifiers::NONE, Key::Enter) || i.consume_key(Modifiers::NONE, Key::Space) {
                    let n = rows[self.var_cursor].name.clone();
                    if !self.var_expanded.remove(&n) {
                        self.var_expanded.insert(n);
                    }
                }
            });
        }
        let mut start_edit: Option<VarEdit> = None;
        let mut delete: Option<(String, String)> = None;
        for (i, v) in rows.iter().enumerate() {
            let expanded = self.var_expanded.contains(&v.name);
            let selected = i == self.var_cursor;
            let frame = egui::Frame::new().fill(if selected { tokens.accent.gamma_multiply(0.08) } else if i % 2 == 1 { tokens.bg2.gamma_multiply(0.5) } else { egui::Color32::TRANSPARENT }).inner_margin(egui::Margin::symmetric(6, 3));
            frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let arrow = ui.add(egui::Label::new(RichText::new(if expanded { "▾" } else { "▸" }).weak()).sense(egui::Sense::click()));
                    let name = ui.add(egui::Label::new(RichText::new(&v.name).strong().size(12.5)).sense(egui::Sense::click())).on_hover_text("Click to expand · double-click to copy name");
                    if arrow.clicked() || name.clicked() {
                        self.var_cursor = i;
                        if !self.var_expanded.remove(&v.name) {
                            self.var_expanded.insert(v.name.clone());
                        }
                    }
                    if name.double_clicked() {
                        ui.ctx().copy_text(v.name.clone());
                    }
                    ui.label(RichText::new(v.type_.clone().unwrap_or_default()).weak().small());
                    let scope = self.scope_name(model, v);
                    if scope != "process" {
                        pill(ui, &scope, tokens.fg1);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !finished {
                            if ui.add(egui::Button::new("✎").small().frame(false)).on_hover_text("Edit").clicked() {
                                start_edit = Some(VarEdit { name: v.name.clone(), type_: v.type_.clone().unwrap_or_else(|| "String".into()), text: edit_text(v), is_new: false, error: None });
                            }
                            if ui.add(egui::Button::new("🗑").small().frame(false)).on_hover_text("Delete").clicked() {
                                delete = Some((v.name.clone(), v.execution_id.clone().unwrap_or_default()));
                            }
                        }
                        if ui.add(egui::Button::new("⎘").small().frame(false)).on_hover_text("Copy value").clicked() {
                            ui.ctx().copy_text(edit_text(v));
                        }
                    });
                });
                if !expanded {
                    let preview = crate::util::value_preview(&v.value, v.type_.as_deref());
                    let r = ui.add(egui::Label::new(RichText::new(highlight_plain(&preview)).monospace().size(11.5).color(tokens.fg1)).truncate().sense(egui::Sense::click()));
                    if r.clicked() {
                        self.var_cursor = i;
                        self.var_expanded.insert(v.name.clone());
                    }
                } else {
                    value_viewer(ui, v, &f, &tokens);
                    if let Some(e) = &v.error_message {
                        ui.label(RichText::new(format!("deserialisation error: {e}")).color(tokens.incident).small());
                    }
                    let info = &v.value_info;
                    if info.is_object() && !info.as_object().unwrap().is_empty() {
                        ui.label(RichText::new(format!("valueInfo: {info}")).weak().small());
                    }
                }
            });
        }
        if let Some(e) = start_edit {
            self.var_edit = Some(e);
        }
        if let Some((name, _exec)) = delete {
            let pid = self.id.clone();
            let old = vars.iter().find(|v| v.name == name).map(|v| v.as_value());
            app.confirm = Some(Confirm::new("Delete variable", format!("Delete variable “{name}” from this instance?"), "Delete", move |app: &mut App| {
                let Some(net) = &app.net else { return };
                let p2 = pid.clone();
                let n2 = name.clone();
                net.run(
                    move |c| async move { c.delete_process_variable(&p2, &n2).await },
                    move |app, r| match r {
                        Ok(()) => {
                            let undo: Option<AppAction> = old.map(|o| {
                                let n3 = name.clone();
                                let p3 = pid.clone();
                                Box::new(move |app: &mut App| {
                                    let n4 = n3.clone();
                                    app.run_action(move |c| async move { c.put_process_variable(&p3, &n4, &o).await }, format!("Variable {n3} restored"), "Undo failed");
                                }) as AppAction
                            });
                            app.toasts.push_full(ToastKind::Success, format!("Variable {name} deleted"), None, undo);
                            app.bump_refresh();
                        }
                        Err(e) => {
                            app.toasts.error(format!("Could not delete {name}"), Some(e.to_string()));
                        }
                    },
                );
            }));
        }
    }

    // ---------- tasks ----------

    fn tasks_dock(&mut self, ui: &mut egui::Ui, app: &mut App, model: &Arc<bpmn::Model>, select: &mut Option<bpmn::Ref>) {
        let tokens = app.tokens.clone();
        loader_status_inline(ui, &self.tasks, &tokens);
        let Some(tasks) = self.tasks.data.clone() else {
            if self.finished() {
                crate::widgets::empty_state(ui, "Finished instance — no open tasks.");
            } else {
                ui.spinner();
            }
            return;
        };
        if tasks.is_empty() {
            crate::widgets::empty_state(ui, "No open user tasks.");
            return;
        }
        let me = app.profile.user.clone();
        for t in &tasks {
            let act = t.task_definition_key.clone().unwrap_or_default();
            let name = t.name.clone().or_else(|| model.element_by_id(&act).map(|e| e.display_name().to_string())).unwrap_or(act.clone());
            egui::Frame::new().fill(tokens.bg2).stroke(egui::Stroke::new(1.0, tokens.bg3)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    if ui.link(RichText::new(&name).strong()).on_hover_text("Select on diagram").clicked() {
                        if let Some(i) = model.element_idx(&act) {
                            *select = Some(bpmn::Ref::Element(i));
                        }
                    }
                    if t.suspended {
                        pill(ui, "suspended", tokens.suspended);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("created {}", t.created.as_deref().map(crate::util::fmt_relative).unwrap_or_default())).weak().small());
                    });
                });
                kv_mono(ui, "task id", &t.id);
                if let Some(d) = &t.due {
                    let overdue = crate::util::parse_time(d).map(|x| x < chrono::Utc::now()).unwrap_or(false);
                    ui.horizontal(|ui| {
                        ui.add_sized([120.0, 18.0], egui::Label::new(RichText::new("due").weak().size(12.0)).halign(egui::Align::Max));
                        ui.label(RichText::new(format!("{} ({})", crate::util::fmt_local(d), crate::util::fmt_relative(d))).color(if overdue { tokens.incident } else { tokens.fg0 }));
                    });
                }
                if let Some(f) = &t.follow_up {
                    kv(ui, "follow-up", &crate::util::fmt_local(f));
                }
                kv(ui, "priority", &t.priority.to_string());
                if let Some(d) = &t.description {
                    kv(ui, "description", d);
                }
                // assignee editor
                ui.horizontal(|ui| {
                    ui.add_sized([120.0, 18.0], egui::Label::new(RichText::new("assignee").weak().size(12.0)).halign(egui::Align::Max));
                    let buf = self.assignee_edit.entry(t.id.clone()).or_insert_with(|| t.assignee.clone().unwrap_or_default());
                    let r = ui.add(egui::TextEdit::singleline(buf).hint_text("unassigned").desired_width(140.0));
                    let changed = buf.trim() != t.assignee.as_deref().unwrap_or("");
                    let tid = t.id.clone();
                    if changed && (ui.small_button("Set").clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))) {
                        let user = buf.trim().to_string();
                        let u = if user.is_empty() { None } else { Some(user.clone()) };
                        if let Some(u) = &u {
                            if !app.settings.recent_users.contains(u) {
                                app.settings.recent_users.insert(0, u.clone());
                                app.settings.recent_users.truncate(10);
                                app.settings_dirty = true;
                            }
                        }
                        app.run_action(move |c| async move { c.set_assignee(&tid, u.as_deref()).await }, if user.is_empty() { "Task unassigned".to_string() } else { format!("Assigned to {user}") }, "Could not set assignee");
                    }
                    if !me.is_empty() && t.assignee.as_deref() != Some(me.as_str()) {
                        let tid = t.id.clone();
                        let me2 = me.clone();
                        if ui.small_button("Claim").on_hover_text(format!("Claim as {me}")).clicked() {
                            app.run_action(move |c| async move { c.claim_task(&tid, &me2).await }, "Task claimed", "Could not claim");
                        }
                    }
                    if t.assignee.is_some() {
                        let tid = t.id.clone();
                        if ui.small_button("Unclaim").clicked() {
                            app.run_action(move |c| async move { c.unclaim_task(&tid).await }, "Task unclaimed", "Could not unclaim");
                        }
                    }
                    if !app.settings.recent_users.is_empty() {
                        let m = ui.small_button("▾");
                        egui::Popup::menu(&m).show(|ui| {
                            for u in app.settings.recent_users.clone() {
                                if ui.button(&u).clicked() {
                                    let tid = t.id.clone();
                                    let u2 = u.clone();
                                    app.run_action(move |c| async move { c.set_assignee(&tid, Some(&u2)).await }, format!("Assigned to {u}"), "Could not set assignee");
                                }
                            }
                        });
                    }
                });
                ui.horizontal(|ui| {
                    let tid = t.id.clone();
                    if ui.small_button("Due +1 day").clicked() {
                        let mut t2 = t.clone();
                        t2.due = Some(crate::util::format_camunda(chrono::Local::now() + chrono::Duration::days(1)));
                        app.run_action(move |c| async move { c.update_task(&t2).await }, "Due date set to tomorrow", "Could not set due date");
                    }
                    if t.due.is_some() && ui.small_button("Clear due").clicked() {
                        let mut t2 = t.clone();
                        t2.due = None;
                        app.run_action(move |c| async move { c.update_task(&t2).await }, "Due date cleared", "Could not clear due date");
                    }
                    let _ = tid;
                    let tid = t.id.clone();
                    if ui.small_button("Complete…").on_hover_text("Complete the task without variables").clicked() {
                        let name2 = name.clone();
                        app.confirm = Some(Confirm::new("Complete task", format!("Complete task “{name2}” with no variables? The process continues."), "Complete", move |app: &mut App| {
                            let tid2 = tid.clone();
                            app.run_action(move |c| async move { c.complete_task(&tid2, &serde_json::json!({})).await }, "Task completed", "Could not complete task");
                        }));
                    }
                });
            });
            ui.add_space(4.0);
        }
    }

    // ---------- incidents ----------

    fn incidents_dock(&mut self, ui: &mut egui::Ui, app: &mut App, model: &Arc<bpmn::Model>, select: &mut Option<bpmn::Ref>, open: &mut Option<(Nav, NavMode)>) {
        let tokens = app.tokens.clone();
        loader_status_inline(ui, &self.incidents, &tokens);
        let Some(list) = self.incidents.data.clone() else {
            ui.spinner();
            return;
        };
        if list.is_empty() {
            crate::widgets::empty_state(ui, "No incidents.");
            return;
        }
        for inc in &list {
            let act = inc.activity_id.clone().or(inc.failed_activity_id.clone()).unwrap_or_default();
            let name = model.element_by_id(&act).map(|e| e.display_name().to_string()).unwrap_or(act.clone());
            let resolved = inc.end_time.is_some();
            egui::Frame::new().fill(tokens.bg2).stroke(egui::Stroke::new(1.0, if resolved { tokens.bg3 } else { tokens.incident.gamma_multiply(0.5) })).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    pill(ui, &inc.incident_type, if resolved { tokens.fg1 } else { tokens.incident });
                    if ui.link(RichText::new(&name).strong()).clicked() {
                        if let Some(i) = model.element_idx(&act) {
                            *select = Some(bpmn::Ref::Element(i));
                        }
                    }
                    if resolved {
                        ui.label(RichText::new(if inc.resolved == Some(true) { "resolved" } else if inc.deleted == Some(true) { "deleted" } else { "closed" }).weak().small());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(inc.time().map(crate::util::fmt_relative).unwrap_or_default()).weak().small()).on_hover_text(inc.time().map(crate::util::fmt_local).unwrap_or_default());
                    });
                });
                let msg = inc.incident_message.clone().unwrap_or_default();
                let r = ui.add(egui::Label::new(RichText::new(&msg).size(12.0)).wrap().sense(egui::Sense::click())).on_hover_text("Click to copy");
                if r.clicked() {
                    ui.ctx().copy_text(msg.clone());
                }
                if let Some(a) = &inc.annotation {
                    kv(ui, "annotation", a);
                }
                if let (Some(root), Some(me)) = (&inc.root_cause_incident_id, Some(&inc.id)) {
                    if root != me {
                        if let Some(rp) = &inc.process_instance_id {
                            let _ = rp;
                        }
                        kv_mono(ui, "root cause incident", root);
                    }
                }
                if inc.process_instance_id.as_deref() != Some(self.id.as_str()) {
                    if let Some(pi) = &inc.process_instance_id {
                        if kv_link(ui, "in instance", &crate::util::short_uuid(pi)) {
                            *open = Some((Nav::Instance { id: pi.clone() }, NavMode::Here));
                        }
                    }
                }
                if !resolved {
                    ui.horizontal(|ui| {
                        match (inc.incident_type.as_str(), inc.configuration.clone()) {
                            ("failedJob", Some(job)) => {
                                if ui.small_button("Retry (retries=1)").clicked() {
                                    app.action_set_job_retries(job.clone(), 1);
                                }
                                if ui.small_button("Retries=3").clicked() {
                                    app.action_set_job_retries(job.clone(), 3);
                                }
                                if ui.small_button("Stack trace").clicked() {
                                    self.load_trace(app, format!("Job {} stack trace", crate::util::short_uuid(&job)), TraceSource::Job(job.clone()));
                                }
                            }
                            ("failedExternalTask", Some(et)) => {
                                if ui.small_button("Retry (retries=1)").clicked() {
                                    app.action_set_external_task_retries(et.clone(), 1);
                                }
                                if ui.small_button("Error details").clicked() {
                                    self.load_trace(app, format!("External task {} error details", crate::util::short_uuid(&et)), TraceSource::ExternalTask(et.clone()));
                                }
                            }
                            _ => {}
                        }
                        let iid = inc.id.clone();
                        if ui.small_button("Annotate…").clicked() {
                            self.trace = Some(TraceView { title: format!("annotate:{iid}"), text: inc.annotation.clone().unwrap_or_default(), filter: String::new() });
                        }
                        if ui.small_button("Resolve").on_hover_text("Delete the incident (does not fix the cause)").clicked() {
                            let iid2 = iid.clone();
                            app.confirm = Some(Confirm::new("Resolve incident", "Mark this incident as resolved? The underlying job or task is left untouched.", "Resolve", move |app: &mut App| {
                                let i3 = iid2.clone();
                                app.run_action(move |c| async move { c.resolve_incident(&i3).await }, "Incident resolved", "Could not resolve incident");
                            }));
                        }
                    });
                }
            });
            ui.add_space(4.0);
        }
        // annotation editor rides in the trace pane
        if let Some(t) = &mut self.trace {
            if let Some(iid) = t.title.strip_prefix("annotate:").map(String::from) {
                let mut done = false;
                egui::Frame::new().fill(tokens.bg2).stroke(egui::Stroke::new(1.0, tokens.accent)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                    ui.label(RichText::new("Incident annotation").strong());
                    ui.add(egui::TextEdit::multiline(&mut t.text).desired_rows(3).desired_width(f32::INFINITY));
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            let a = t.text.clone();
                            app.run_action(move |c| async move { c.set_incident_annotation(&iid, &a).await }, "Annotation saved", "Could not save annotation");
                            done = true;
                        }
                        if ui.button("Cancel").clicked() {
                            done = true;
                        }
                    });
                });
                if done {
                    self.trace = None;
                }
            }
        }
    }

    fn load_trace(&mut self, app: &mut App, title: String, src: TraceSource) {
        let Some(net) = &app.net else { return };
        self.trace = Some(TraceView { title: title.clone(), text: "loading…".into(), filter: String::new() });
        let id = self.id.clone();
        net.run(
            move |c| async move {
                match src {
                    TraceSource::Job(j) => c.job_stacktrace(&j).await,
                    TraceSource::ExternalTask(e) => c.external_task_error_details(&e).await,
                }
            },
            move |app, r| {
                let text = match r {
                    Ok(t) => t,
                    Err(e) => format!("Could not load: {e}"),
                };
                for tab in &mut app.tabs {
                    for p in &mut tab.history {
                        if let crate::state::Place::Instance(ip) = p {
                            if ip.id == id {
                                if let Some(t) = &mut ip.trace {
                                    if t.title == title {
                                        t.text = text.clone();
                                    }
                                }
                            }
                        }
                    }
                }
            },
        );
    }

    // ---------- jobs ----------

    fn jobs_dock(&mut self, ui: &mut egui::Ui, app: &mut App, model: &Arc<bpmn::Model>, select: &mut Option<bpmn::Ref>) {
        let tokens = app.tokens.clone();
        loader_status_inline(ui, &self.jobs, &tokens);
        let Some(jobs) = self.jobs.data.clone() else {
            if self.finished() {
                crate::widgets::empty_state(ui, "Finished instance — no jobs.");
            } else {
                ui.spinner();
            }
            return;
        };
        if jobs.is_empty() {
            crate::widgets::empty_state(ui, "No jobs (timers or async continuations).");
            return;
        }
        let defs = self.job_defs.data.clone().unwrap_or_default();
        for j in &jobs {
            let def = defs.iter().find(|d| Some(&d.id) == j.job_definition_id.as_ref());
            let act = def.and_then(|d| d.activity_id.clone()).or(j.failed_activity_id.clone()).unwrap_or_default();
            let name = model.element_by_id(&act).map(|e| e.display_name().to_string()).unwrap_or(act.clone());
            let kind = def.and_then(|d| d.job_type.clone()).unwrap_or_else(|| "job".into());
            let failed = j.retries == 0;
            egui::Frame::new().fill(tokens.bg2).stroke(egui::Stroke::new(1.0, if failed { tokens.incident.gamma_multiply(0.5) } else { tokens.bg3 })).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    pill(ui, &kind, if failed { tokens.incident } else { tokens.suspended });
                    if ui.link(RichText::new(&name).strong()).clicked() {
                        if let Some(i) = model.element_idx(&act) {
                            *select = Some(bpmn::Ref::Element(i));
                        }
                    }
                    if j.suspended {
                        pill(ui, "suspended", tokens.suspended);
                    }
                });
                kv_mono(ui, "job id", &j.id);
                if let Some(d) = &j.due_date {
                    kv(ui, "due", &format!("{}  ({})", crate::util::fmt_local(d), crate::util::fmt_relative(d)));
                } else {
                    kv(ui, "due", "as soon as possible");
                }
                kv(ui, "retries", &j.retries.to_string());
                kv(ui, "priority", &j.priority.to_string());
                if let Some(c) = def.and_then(|d| d.job_configuration.clone()) {
                    kv(ui, "configuration", &c);
                }
                if let Some(e) = &j.exception_message {
                    kv(ui, "exception", &crate::util::ellipsize(e, 300));
                }
                ui.horizontal_wrapped(|ui| {
                    let id = j.id.clone();
                    if ui.small_button("Execute now").on_hover_text("Run the job synchronously").clicked() {
                        let i2 = id.clone();
                        app.run_action(move |c| async move { c.execute_job(&i2).await }, "Job executed", "Job execution failed");
                    }
                    if ui.small_button("Retries → 1").clicked() {
                        app.action_set_job_retries(id.clone(), 1);
                    }
                    if ui.small_button("Retries → 3").clicked() {
                        app.action_set_job_retries(id.clone(), 3);
                    }
                    if ui.small_button("Due now").clicked() {
                        let i2 = id.clone();
                        let now = crate::util::format_camunda(chrono::Local::now());
                        app.run_action(move |c| async move { c.set_job_duedate(&i2, Some(&now)).await }, "Due date set to now", "Could not set due date");
                    }
                    if ui.small_button("Due +1 h").clicked() {
                        let i2 = id.clone();
                        let t = crate::util::format_camunda(chrono::Local::now() + chrono::Duration::hours(1));
                        app.run_action(move |c| async move { c.set_job_duedate(&i2, Some(&t)).await }, "Due date moved by one hour", "Could not set due date");
                    }
                    if ui.small_button(if j.suspended { "Activate" } else { "Suspend" }).clicked() {
                        let i2 = id.clone();
                        let s = !j.suspended;
                        app.run_action(move |c| async move { c.suspend_job(&i2, s).await }, "Job suspension changed", "Could not change job");
                    }
                    if j.exception_message.is_some() && ui.small_button("Stack trace").clicked() {
                        self.load_trace(app, format!("Job {} stack trace", crate::util::short_uuid(&id)), TraceSource::Job(id.clone()));
                    }
                });
            });
            ui.add_space(4.0);
        }
    }

    // ---------- external tasks ----------

    fn external_dock(&mut self, ui: &mut egui::Ui, app: &mut App, model: &Arc<bpmn::Model>, select: &mut Option<bpmn::Ref>) {
        let tokens = app.tokens.clone();
        loader_status_inline(ui, &self.external_tasks, &tokens);
        let Some(list) = self.external_tasks.data.clone() else {
            if self.finished() {
                crate::widgets::empty_state(ui, "Finished instance — no external tasks.");
            } else {
                ui.spinner();
            }
            return;
        };
        if list.is_empty() {
            crate::widgets::empty_state(ui, "No external tasks.");
            return;
        }
        for e in &list {
            let act = e.activity_id.clone().unwrap_or_default();
            let name = model.element_by_id(&act).map(|x| x.display_name().to_string()).unwrap_or(act.clone());
            let failed = e.retries == Some(0);
            egui::Frame::new().fill(tokens.bg2).stroke(egui::Stroke::new(1.0, if failed { tokens.incident.gamma_multiply(0.5) } else { tokens.bg3 })).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    pill(ui, e.topic_name.as_deref().unwrap_or("topic"), tokens.external);
                    if ui.link(RichText::new(&name).strong()).clicked() {
                        if let Some(i) = model.element_idx(&act) {
                            *select = Some(bpmn::Ref::Element(i));
                        }
                    }
                });
                kv_mono(ui, "task id", &e.id);
                kv(ui, "worker", e.worker_id.as_deref().unwrap_or("— (not locked)"));
                if let Some(l) = &e.lock_expiration_time {
                    kv(ui, "lock expires", &format!("{} ({})", crate::util::fmt_local(l), crate::util::fmt_relative(l)));
                }
                kv(ui, "retries", &e.retries.map(|r| r.to_string()).unwrap_or_else(|| "— (never failed)".into()));
                kv(ui, "priority", &e.priority.to_string());
                if let Some(m) = &e.error_message {
                    kv(ui, "error", m);
                }
                ui.horizontal_wrapped(|ui| {
                    let id = e.id.clone();
                    if ui.small_button("Retries → 1").clicked() {
                        app.action_set_external_task_retries(id.clone(), 1);
                    }
                    if e.worker_id.is_some() && ui.small_button("Unlock").clicked() {
                        let i2 = id.clone();
                        app.run_action(move |c| async move { c.unlock_external_task(&i2).await }, "External task unlocked", "Could not unlock");
                    }
                    if ui.small_button("Priority +10").clicked() {
                        let i2 = id.clone();
                        let p = e.priority + 10;
                        app.run_action(move |c| async move { c.set_external_task_priority(&i2, p).await }, "Priority raised", "Could not set priority");
                    }
                    if e.error_message.is_some() && ui.small_button("Error details").clicked() {
                        self.load_trace(app, format!("External task {} error details", crate::util::short_uuid(&id)), TraceSource::ExternalTask(id.clone()));
                    }
                });
            });
            ui.add_space(4.0);
        }
    }

    // ---------- called instances ----------

    fn called_dock(&mut self, ui: &mut egui::Ui, app: &mut App, model: &Arc<bpmn::Model>, open: &mut Option<(Nav, NavMode)>) {
        let tokens = app.tokens.clone();
        if let Some(chain) = self.ancestors.data.clone() {
            if !chain.is_empty() {
                section(ui, "called-by", "Called by", true, |ui| {
                    for a in chain.iter() {
                        let label = format!("{} · {}", a.process_definition_key.clone().unwrap_or_default(), a.business_key.clone().unwrap_or_else(|| crate::util::short_uuid(&a.id)));
                        if ui.link(label).clicked() {
                            *open = Some((Nav::Instance { id: a.id.clone() }, NavMode::Here));
                        }
                    }
                });
            }
        }
        loader_status_inline(ui, &self.called, &tokens);
        let Some(list) = self.called.data.clone() else {
            ui.spinner();
            return;
        };
        if list.is_empty() {
            crate::widgets::empty_state(ui, "No called process instances.");
            return;
        }
        let acts = self.history_acts.data.clone().unwrap_or_default();
        egui::Grid::new("called-grid").num_columns(4).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
            for t in ["Definition", "Business key / id", "Via", "State"] {
                ui.label(RichText::new(t).strong().size(11.5));
            }
            ui.end_row();
            for c in &list {
                let via = acts.iter().find(|a| a.called_process_instance_id.as_deref() == Some(c.id.as_str())).map(|a| model.element_by_id(a.base_activity_id()).map(|e| e.display_name().to_string()).unwrap_or(a.activity_id.clone())).unwrap_or_default();
                ui.label(c.process_definition_key.clone().unwrap_or_default());
                if ui.link(RichText::new(c.business_key.clone().unwrap_or_else(|| crate::util::short_uuid(&c.id))).monospace().size(12.0)).clicked() {
                    *open = Some((Nav::Instance { id: c.id.clone() }, if ui.input(|i| i.modifiers.shift) { NavMode::NewTab } else { NavMode::Here }));
                }
                ui.label(via);
                let st = c.state.clone().unwrap_or_default();
                ui.label(RichText::new(state_label(&st)).color(state_color(&st, &tokens)));
                ui.end_row();
            }
        });
    }

    // ---------- timeline ----------

    fn timeline_dock(&mut self, ui: &mut egui::Ui, app: &mut App, model: &Arc<bpmn::Model>, select: &mut Option<bpmn::Ref>) {
        let tokens = app.tokens.clone();
        loader_status_inline(ui, &self.history_acts, &tokens);
        let Some(acts) = self.history_acts.data.clone() else {
            ui.spinner();
            return;
        };
        let mut rows: Vec<&HistoricActivityInstance> = acts.iter().filter(|a| !a.is_multi_instance_body()).collect();
        rows.sort_by(|a, b| a.start_time.cmp(&b.start_time));
        if rows.is_empty() {
            crate::widgets::empty_state(ui, "No activity history yet.");
            return;
        }
        let ts = |s: &Option<String>| s.as_deref().and_then(crate::util::parse_time).map(|t| t.timestamp_millis());
        let t0 = rows.iter().filter_map(|a| ts(&a.start_time)).min().unwrap_or(0);
        let now = chrono::Utc::now().timestamp_millis();
        let t1 = rows.iter().filter_map(|a| ts(&a.end_time)).max().unwrap_or(now).max(if self.finished() { 0 } else { now });
        let span = (t1 - t0).max(1000) as f32;
        ui.label(RichText::new(format!("{} activity instances · span {}", rows.len(), crate::util::human_millis(t1 - t0))).weak().small());
        let label_w = 170.0;
        let row_h = 20.0;
        let total_h = rows.len() as f32 * row_h + 24.0;
        let (rect, resp) = ui.allocate_exact_size(egui::Vec2::new(ui.available_width(), total_h), egui::Sense::click());
        let p = ui.painter_at(rect);
        let bar_x0 = rect.min.x + label_w;
        let bar_w = (rect.width() - label_w - 8.0).max(50.0);
        // axis ticks
        for i in 0..=4 {
            let x = bar_x0 + bar_w * i as f32 / 4.0;
            p.line_segment([egui::Pos2::new(x, rect.min.y), egui::Pos2::new(x, rect.max.y)], egui::Stroke::new(1.0, tokens.bg3));
            let t = t0 + (span * i as f32 / 4.0) as i64;
            let fmt = if span < 10_000.0 { "%H:%M:%S%.3f" } else { "%H:%M:%S" };
            let label = chrono::DateTime::from_timestamp_millis(t).map(|d| d.with_timezone(&chrono::Local).format(fmt).to_string()).unwrap_or_default();
            p.text(egui::Pos2::new(x + 2.0, rect.min.y), egui::Align2::LEFT_TOP, label, egui::FontId::proportional(10.0), tokens.fg2);
        }
        let hover = resp.hover_pos();
        let mut hovered_row: Option<usize> = None;
        for (i, a) in rows.iter().enumerate() {
            let y = rect.min.y + 18.0 + i as f32 * row_h;
            let name = a.activity_name.clone().filter(|n| !n.is_empty()).or_else(|| model.element_by_id(a.base_activity_id()).map(|e| e.display_name().to_string())).unwrap_or(a.activity_id.clone());
            let rrow = egui::Rect::from_min_size(egui::Pos2::new(rect.min.x, y), egui::Vec2::new(rect.width(), row_h));
            let is_hover = hover.map(|h| rrow.contains(h)).unwrap_or(false);
            if is_hover {
                hovered_row = Some(i);
                p.rect_filled(rrow, 2.0, tokens.accent.gamma_multiply(0.1));
            }
            let is_sel = match self.view.selection {
                Some(bpmn::Ref::Element(ei)) => model.elements[ei].id == a.base_activity_id(),
                _ => false,
            };
            if is_sel {
                p.rect_filled(rrow, 2.0, tokens.accent.gamma_multiply(0.18));
            }
            p.text(egui::Pos2::new(rect.min.x + 4.0, y + row_h / 2.0), egui::Align2::LEFT_CENTER, crate::util::ellipsize(&name, 26), egui::FontId::proportional(11.5), tokens.fg0);
            let s = ts(&a.start_time).unwrap_or(t0);
            let e = ts(&a.end_time).unwrap_or(now);
            let x0 = bar_x0 + bar_w * ((s - t0) as f32 / span);
            let x1 = bar_x0 + bar_w * ((e - t0) as f32 / span);
            let color = if a.canceled {
                tokens.incident
            } else if a.end_time.is_none() {
                tokens.running
            } else if a.activity_type.as_deref().map(|t| t.contains("Gateway") || t.contains("Event")).unwrap_or(false) {
                tokens.fg2
            } else {
                tokens.completed
            };
            let br = egui::Rect::from_min_max(egui::Pos2::new(x0, y + 5.0), egui::Pos2::new(x1.max(x0 + 3.0), y + row_h - 5.0));
            p.rect_filled(br, 3.0, color);
            if a.end_time.is_none() {
                p.rect_stroke(br, 3.0, egui::Stroke::new(1.0, tokens.fg0.gamma_multiply(0.4)), egui::StrokeKind::Outside);
            }
            if let Some(as_) = &a.assignee {
                p.text(egui::Pos2::new(br.max.x + 4.0, y + row_h / 2.0), egui::Align2::LEFT_CENTER, as_, egui::FontId::proportional(10.0), tokens.fg1);
            }
        }
        if !self.finished() {
            let x = bar_x0 + bar_w * ((now - t0) as f32 / span);
            p.line_segment([egui::Pos2::new(x, rect.min.y + 14.0), egui::Pos2::new(x, rect.max.y)], egui::Stroke::new(1.0, tokens.running));
        }
        if let Some(i) = hovered_row {
            let a = rows[i];
            crate::widgets::tooltip_at_pointer(ui, egui::Id::unique(("tl-tip", &a.id)), |ui| {
                ui.label(RichText::new(a.activity_name.clone().unwrap_or(a.activity_id.clone())).strong());
                ui.label(format!("{}  ·  {}", a.activity_type.clone().unwrap_or_default(), a.base_activity_id()));
                ui.label(format!("start {}", a.start_time.as_deref().map(crate::util::fmt_local).unwrap_or_default()));
                if let Some(e) = &a.end_time {
                    ui.label(format!("end   {}  ({})", crate::util::fmt_local(e), a.duration_in_millis.map(crate::util::human_millis).unwrap_or_default()));
                } else {
                    ui.label("still active");
                }
                if a.canceled {
                    ui.label(RichText::new("cancelled").color(tokens.incident));
                }
                if let Some(u) = &a.assignee {
                    ui.label(format!("assignee {u}"));
                }
                if let Some(c) = &a.called_process_instance_id {
                    ui.label(format!("called instance {}", crate::util::short_uuid(c)));
                }
            });
            if resp.clicked() {
                if let Some(ei) = model.element_idx(a.base_activity_id()) {
                    *select = Some(bpmn::Ref::Element(ei));
                }
            }
        }
    }

    // ---------- modification ----------

    fn apply_mod_action(&mut self, a: ModAction, app: &mut App, model: &Arc<bpmn::Model>) {
        let name = |id: &str| model.element_by_id(id).map(|e| e.display_name().to_string()).unwrap_or(id.to_string());
        match a {
            ModAction::StartBefore(id) => {
                self.modification.instructions.push(ModificationInstruction::start_before(&id));
                app.toasts.push(ToastKind::Info, format!("Added: start before {}", name(&id)));
            }
            ModAction::StartAfter(id) => {
                self.modification.instructions.push(ModificationInstruction::start_after(&id));
                app.toasts.push(ToastKind::Info, format!("Added: start after {}", name(&id)));
            }
            ModAction::Cancel(id) => {
                self.modification.instructions.push(ModificationInstruction::cancel_activity(&id));
                app.toasts.push(ToastKind::Info, format!("Added: cancel tokens at {}", name(&id)));
            }
            ModAction::MoveFrom(id) => {
                self.modification.move_from = Some(id.clone());
                self.view.pick = Some(PickMode::Target { label: format!("Move token from {}", name(&id)) });
            }
            ModAction::Picked(target) => {
                self.view.pick = None;
                if let Some(from) = self.modification.move_from.take() {
                    self.modification.instructions.push(ModificationInstruction::cancel_activity(&from));
                    self.modification.instructions.push(ModificationInstruction::start_before(&target));
                    app.toasts.push(ToastKind::Info, format!("Added: move token {} → {}", name(&from), name(&target)));
                } else {
                    self.modification.instructions.push(ModificationInstruction::start_before(&target));
                }
            }
        }
        self.dock_tab = 9;
        app.dock_visible = true;
    }

    fn modify_dock(&mut self, ui: &mut egui::Ui, app: &mut App, model: &Arc<bpmn::Model>, select: &mut Option<bpmn::Ref>) {
        let tokens = app.tokens.clone();
        let finished = self.finished();
        if finished {
            ui.label(RichText::new("Restart this finished instance").strong());
            ui.label(RichText::new("Pick where the new instance should start: right-click an element on the diagram → “Start token before/after”. Variables from the end of this instance are copied when “initial variables” is off.").weak().small());
        } else {
            ui.label(RichText::new("Token modification").strong());
            ui.label(RichText::new("Right-click an element on the diagram: start a token before/after it, cancel tokens there, or move a token. Nothing is sent until you apply.").weak().small());
        }
        ui.add_space(6.0);
        if self.modification.instructions.is_empty() {
            crate::widgets::empty_state(ui, "No instructions yet.");
        }
        let mut remove: Option<usize> = None;
        let mut swap: Option<(usize, usize)> = None;
        let n = self.modification.instructions.len();
        for (i, ins) in self.modification.instructions.iter().enumerate() {
            let act = ins.activity_id.clone().unwrap_or_default();
            let name = model.element_by_id(&act).map(|e| e.display_name().to_string()).unwrap_or(act.clone());
            egui::Frame::new().fill(tokens.bg2).stroke(egui::Stroke::new(1.0, tokens.bg3)).corner_radius(6.0).inner_margin(6.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}.", i + 1)).weak());
                    let color = if ins.type_ == "cancel" { tokens.incident } else { tokens.completed };
                    pill(ui, if ins.type_ == "cancel" { "cancel" } else if ins.type_ == "startAfterActivity" { "start after" } else { "start before" }, color);
                    if ui.link(RichText::new(&name).strong()).clicked() {
                        if let Some(ei) = model.element_idx(&act) {
                            *select = Some(bpmn::Ref::Element(ei));
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            remove = Some(i);
                        }
                        if i + 1 < n && ui.small_button("▼").clicked() {
                            swap = Some((i, i + 1));
                        }
                        if i > 0 && ui.small_button("▲").clicked() {
                            swap = Some((i, i - 1));
                        }
                    });
                });
            });
        }
        if let Some(i) = remove {
            self.modification.instructions.remove(i);
        }
        if let Some((a, b)) = swap {
            self.modification.instructions.swap(a, b);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.modification.skip_listeners, "Skip custom listeners");
            ui.checkbox(&mut self.modification.skip_io, "Skip I/O mappings");
        });
        if !finished {
            ui.add(egui::TextEdit::singleline(&mut self.modification.annotation).hint_text("annotation (optional, stored in the operation log)").desired_width(f32::INFINITY));
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let ok = !self.modification.instructions.is_empty();
            let label = if finished { "Restart instance…" } else { "Apply modification…" };
            if ui.add_enabled(ok, egui::Button::new(RichText::new(label).color(egui::Color32::WHITE)).fill(tokens.accent)).clicked() {
                let instructions = self.modification.instructions.clone();
                let desc: Vec<String> = instructions.iter().map(|i| format!("• {}", i.describe())).collect();
                let id = self.id.clone();
                let skip_l = self.modification.skip_listeners;
                let skip_io = self.modification.skip_io;
                let ann = self.modification.annotation.clone();
                let def_id = self.definition_id().unwrap_or_default();
                let body = if finished {
                    format!("Start a new instance of this process with:\n{}\n\nVariables are copied from the finished instance.", desc.join("\n"))
                } else {
                    format!("Apply to instance {}:\n{}", crate::util::short_uuid(&id), desc.join("\n"))
                };
                app.confirm = Some(Confirm::new(if finished { "Restart instance" } else { "Modify tokens" }, body, if finished { "Restart" } else { "Apply" }, move |app: &mut App| {
                    if finished {
                        let req = RestartRequest { process_instance_ids: vec![id.clone()], instructions: instructions.clone(), initial_variables: Some(false), skip_custom_listeners: Some(skip_l), skip_io_mappings: Some(skip_io), without_business_key: Some(false) };
                        let d = def_id.clone();
                        app.run_action(move |c| async move { c.restart_instances(&d, &req, false).await }, "Instance restarted (see the definition's instance list)", "Restart failed");
                    } else {
                        let req = ModificationRequest { instructions: instructions.clone(), skip_custom_listeners: Some(skip_l), skip_io_mappings: Some(skip_io), annotation: if ann.trim().is_empty() { None } else { Some(ann.clone()) } };
                        let i2 = id.clone();
                        app.run_action(move |c| async move { c.modify_instance(&i2, &req, false).await }, "Modification applied", "Modification failed");
                    }
                }));
                self.modification.instructions.clear();
            }
            if ui.button("Clear").clicked() {
                self.modification.instructions.clear();
                self.modification.move_from = None;
                self.view.pick = None;
            }
        });
    }
}

enum ModAction {
    StartBefore(String),
    StartAfter(String),
    Cancel(String),
    MoveFrom(String),
    Picked(String),
}

enum TraceSource {
    Job(String),
    ExternalTask(String),
}

trait Invalidate {
    fn invalidate_dyn(&mut self);
}
impl<T> Invalidate for Loader<T> {
    fn invalidate_dyn(&mut self) {
        self.invalidate();
    }
}

fn app_pid(app: &App) -> String {
    match app.tabs.get(app.active).map(|t| t.place()) {
        Some(crate::state::Place::Instance(p)) => p.id.clone(),
        _ => String::new(),
    }
}

/// Text shown in the editor / copied for a variable.
pub fn edit_text(v: &VariableInstance) -> String {
    match &v.value {
        serde_json::Value::String(s) => {
            if matches!(v.type_.as_deref(), Some("Json") | Some("Object")) {
                serde_json::from_str::<serde_json::Value>(s).map(|j| serde_json::to_string_pretty(&j).unwrap_or(s.clone())).unwrap_or(s.clone())
            } else {
                s.clone()
            }
        }
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn build_value(type_: &str, text: &str) -> Result<VariableValue, String> {
    let t = text.trim();
    Ok(match type_ {
        "String" => VariableValue::new("String", serde_json::Value::String(text.to_string())),
        "Integer" | "Long" | "Short" => VariableValue::new(type_, serde_json::Value::Number(t.parse::<i64>().map_err(|e| format!("not an integer: {e}"))?.into())),
        "Double" => VariableValue::new("Double", serde_json::json!(t.parse::<f64>().map_err(|e| format!("not a number: {e}"))?)),
        "Boolean" => VariableValue::new("Boolean", serde_json::Value::Bool(match t {
            "true" | "1" | "yes" => true,
            "false" | "0" | "no" => false,
            _ => return Err("use true or false".into()),
        })),
        "Date" => {
            let d = crate::query::date_value(t).ok_or("unrecognised date")?;
            VariableValue::new("Date", serde_json::Value::String(d))
        }
        "Json" => {
            let j: serde_json::Value = serde_json::from_str(t).map_err(|e| format!("invalid JSON: {e}"))?;
            VariableValue::new("Json", serde_json::Value::String(serde_json::to_string(&j).unwrap()))
        }
        "Xml" => VariableValue::new("Xml", serde_json::Value::String(text.to_string())),
        "Object" => {
            let j: serde_json::Value = serde_json::from_str(t).map_err(|e| format!("invalid JSON: {e}"))?;
            let mut v = VariableValue::new("Object", serde_json::Value::String(serde_json::to_string(&j).unwrap()));
            v.value_info = serde_json::json!({"serializationDataFormat": "application/json", "objectTypeName": "java.util.LinkedHashMap"});
            v
        }
        "Null" => VariableValue { type_: Some("Null".into()), value: serde_json::Value::Null, value_info: serde_json::json!({}) },
        other => VariableValue::new(other, serde_json::Value::String(text.to_string())),
    })
}

fn highlight_plain(s: &str) -> String {
    s.to_string()
}

/// Expanded value view: JSON tree with folding, or wrapped text.
fn value_viewer(ui: &mut egui::Ui, v: &VariableInstance, filter: &str, tokens: &crate::theme::Tokens) {
    let text = edit_text(v);
    let parsed: Option<serde_json::Value> = match &v.value {
        serde_json::Value::String(s) if matches!(v.type_.as_deref(), Some("Json") | Some("Object")) => serde_json::from_str(s).ok(),
        serde_json::Value::String(_) => None,
        other => Some(other.clone()),
    };
    egui::Frame::new().fill(ui.visuals().extreme_bg_color).corner_radius(4.0).inner_margin(6.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        match parsed {
            Some(j) if j.is_object() || j.is_array() => {
                json_tree(ui, &j, "", 0, filter, tokens, &v.name);
            }
            _ => {
                let r = ui.add(egui::Label::new(RichText::new(&text).monospace().size(11.5)).wrap().sense(egui::Sense::click()));
                if r.clicked() {
                    ui.ctx().copy_text(text.clone());
                }
            }
        }
    });
}

fn json_tree(ui: &mut egui::Ui, v: &serde_json::Value, key: &str, depth: usize, filter: &str, tokens: &crate::theme::Tokens, salt: &str) {
    let f = filter.to_lowercase();
    let matches = |s: &str| !f.is_empty() && s.to_lowercase().contains(&f);
    match v {
        serde_json::Value::Object(m) => {
            let header = if key.is_empty() { format!("{{…}} {} keys", m.len()) } else { format!("{key}: {{…}} {} keys", m.len()) };
            let id = ui.make_persistent_id((salt, key, depth, "obj"));
            let default_open = depth < 2 || (!f.is_empty() && v.to_string().to_lowercase().contains(&f));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, default_open)
                .show_header(ui, |ui| {
                    ui.label(RichText::new(header).monospace().size(11.5).color(tokens.fg1));
                })
                .body(|ui| {
                    for (k, val) in m {
                        json_tree(ui, val, k, depth + 1, filter, tokens, salt);
                    }
                });
        }
        serde_json::Value::Array(a) => {
            let header = if key.is_empty() { format!("[…] {} items", a.len()) } else { format!("{key}: […] {} items", a.len()) };
            let id = ui.make_persistent_id((salt, key, depth, "arr"));
            let default_open = depth < 2 || (!f.is_empty() && v.to_string().to_lowercase().contains(&f));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, default_open)
                .show_header(ui, |ui| {
                    ui.label(RichText::new(header).monospace().size(11.5).color(tokens.fg1));
                })
                .body(|ui| {
                    for (i, val) in a.iter().enumerate() {
                        json_tree(ui, val, &format!("[{i}]"), depth + 1, filter, tokens, salt);
                    }
                });
        }
        scalar => {
            let s = match scalar {
                serde_json::Value::String(s) => format!("\"{s}\""),
                other => other.to_string(),
            };
            let hit = matches(key) || matches(&s);
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{key}:")).monospace().size(11.5).color(if hit { tokens.matched } else { tokens.fg1 }));
                let color = match scalar {
                    serde_json::Value::String(_) => tokens.completed,
                    serde_json::Value::Number(_) => tokens.running,
                    serde_json::Value::Bool(_) => tokens.suspended,
                    _ => tokens.fg2,
                };
                let r = ui.add(egui::Label::new(RichText::new(&s).monospace().size(11.5).color(if hit { tokens.matched } else { color })).wrap().sense(egui::Sense::click()));
                if r.clicked() {
                    ui.ctx().copy_text(match scalar {
                        serde_json::Value::String(x) => x.clone(),
                        o => o.to_string(),
                    });
                }
            });
        }
    }
}
