//! Instance search: one query language, runtime or history backend, variable columns, bulk ops.

use crate::app::App;
use crate::net::Loader;
use crate::query::{self, Mode, Query};
use crate::state::{Nav, NavMode};
use crate::widgets::{cell, heading_row, id_cell, loader_status_inline, pill, progress_line, state_color, state_label, table_area, Confirm, ToastKind};
use camunda::{HistoricProcessInstance, VariableInstance};
use egui::{Key, Modifiers, RichText};
use egui_extras::{Column, TableBuilder};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: String,
    pub business_key: Option<String>,
    pub definition_id: String,
    pub definition_key: String,
    pub version: Option<i64>,
    pub state: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub duration_ms: Option<i64>,
}

impl From<HistoricProcessInstance> for Row {
    fn from(h: HistoricProcessInstance) -> Self {
        Row { id: h.id, business_key: h.business_key, definition_id: h.process_definition_id, definition_key: h.process_definition_key.unwrap_or_default(), version: h.process_definition_version, state: h.state.unwrap_or_else(|| "ACTIVE".into()), start: h.start_time, end: h.end_time, duration_ms: h.duration_in_millis }
    }
}

/// A bulk operation over selected instance ids, run after confirmation.
type BatchFn = Box<dyn FnOnce(&mut App, Vec<String>) + Send>;

/// variable name → (instance id → value), for the variable columns.
type VarColumns = HashMap<String, HashMap<String, VariableInstance>>;

pub struct SearchPlace {
    pub query: String,
    pub executed: String,
    pub force_mode: Option<Mode>,
    pub rows: Loader<Vec<Row>>,
    pub count: Loader<i64>,
    pub page: usize,
    pub cursor: usize,
    pub selected: HashSet<String>,
    pub var_columns: Vec<String>,
    pub var_values: HashMap<String, Loader<HashMap<String, VariableInstance>>>,
    pub new_var_col: String,
    pub preview: Loader<Vec<VariableInstance>>,
    pub preview_for: String,
    pub want_focus: bool,
    pub save_name: String,
    pub saving: bool,
    pub suggest_idx: usize,
    pub activities_cache: HashMap<String, Vec<(String, String)>>,
    pub known_var_names: Vec<String>,
    seen_epoch: u64,
    last_key: String,
    activities_for: Option<String>,
}

impl SearchPlace {
    pub fn new(query: String) -> Self {
        Self {
            executed: query.clone(),
            want_focus: query.is_empty(),
            query,
            force_mode: None,
            rows: Loader::default(),
            count: Loader::default(),
            page: 0,
            cursor: 0,
            selected: HashSet::new(),
            var_columns: vec![],
            var_values: HashMap::new(),
            new_var_col: String::new(),
            preview: Loader::default(),
            preview_for: String::new(),
            save_name: String::new(),
            saving: false,
            suggest_idx: 0,
            activities_cache: HashMap::new(),
            known_var_names: vec![],
            seen_epoch: 0,
            last_key: String::new(),
            activities_for: None,
        }
    }

    pub fn title(&self) -> String {
        if self.executed.trim().is_empty() {
            crate::t!("common-search")
        } else {
            format!("🔍 {}", crate::util::ellipsize(&self.executed, 22))
        }
    }

    fn mode(&self, q: &Query) -> Mode {
        self.force_mode.unwrap_or(if query::needs_history(q) { Mode::History } else { Mode::Runtime })
    }

    pub fn run_query(&mut self) {
        self.run();
    }

    fn run(&mut self) {
        self.executed = self.query.trim().to_string();
        self.page = 0;
        self.cursor = 0;
        self.selected.clear();
        self.rows.invalidate();
        self.count.invalidate();
        self.last_key.clear();
    }

    fn poll(&mut self, app: &mut App) {
        if self.seen_epoch != app.refresh_epoch {
            self.seen_epoch = app.refresh_epoch;
            self.rows.invalidate();
            self.count.invalidate();
            for l in self.var_values.values_mut() {
                l.invalidate();
            }
        }
        app.ensure_definitions();
        let Some(net) = app.net.as_ref() else { return };
        if self.executed.trim().is_empty() {
            return;
        }
        let q = query::parse(&self.executed);
        let mode = self.mode(&q);
        let key = format!("{}|{:?}|{}", self.executed, mode, self.page);
        if key != self.last_key {
            self.last_key = key;
            self.rows.invalidate();
            self.count.invalidate();
        }
        let defs = app.cache.all_defs.data.clone().unwrap_or_default();
        let refresh = Duration::from_secs(app.settings.refresh_list_secs);
        let auto = app.auto_refresh_allowed();
        let due = |age: Option<Duration>| age.is_none() || auto;
        if self.rows.stale(refresh) && due(self.rows.age()) {
            let first = self.page * app.settings.page_size;
            let max = app.settings.page_size;
            match mode {
                Mode::Runtime => {
                    let rq = query::build_runtime(&q, &defs);
                    let rq2 = rq.clone();
                    let defs2 = defs.clone();
                    net.fetch(&mut self.rows, move |c| async move {
                        let pis = c.process_instances(&rq2, first, max).await?;
                        // enrich with history rows (start time, key) in one query by ids
                        let ids: Vec<String> = pis.iter().map(|p| p.id.clone()).collect();
                        let mut rows: Vec<Row> = vec![];
                        if !ids.is_empty() {
                            let hq = camunda::HistoricProcessInstanceQuery { process_instance_ids: Some(ids.clone()), ..Default::default() };
                            let hist = c.history_process_instances(&hq, 0, ids.len()).await.unwrap_or_default();
                            let by_id: HashMap<String, HistoricProcessInstance> = hist.into_iter().map(|h| (h.id.clone(), h)).collect();
                            for p in pis {
                                let mut r: Row = match by_id.get(&p.id) {
                                    Some(h) => h.clone().into(),
                                    None => Row { id: p.id.clone(), business_key: p.business_key.clone(), definition_id: p.definition_id.clone(), definition_key: defs2.iter().find(|d| d.id == p.definition_id).map(|d| d.key.clone()).unwrap_or_default(), version: None, state: "ACTIVE".into(), start: None, end: None, duration_ms: None },
                                };
                                if p.suspended {
                                    r.state = "SUSPENDED".into();
                                }
                                rows.push(r);
                            }
                        }
                        Ok(rows)
                    });
                    net.fetch(&mut self.count, move |c| async move { c.process_instances_count(&rq).await });
                }
                Mode::History => {
                    let hq = query::build_history(&q, &defs);
                    let hq2 = hq.clone();
                    net.fetch(&mut self.rows, move |c| async move { Ok(c.history_process_instances(&hq2, first, max).await?.into_iter().map(Row::from).collect()) });
                    net.fetch(&mut self.count, move |c| async move { c.history_process_instances_count(&hq).await });
                }
            }
        }
        if self.rows.poll() {
            for l in self.var_values.values_mut() {
                l.invalidate();
            }
        }
        self.count.poll();
        // variable columns
        if let Some(rows) = &self.rows.data {
            let ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
            for name in self.var_columns.clone() {
                let l = self.var_values.entry(name.clone()).or_default();
                if l.stale(refresh) && !ids.is_empty() {
                    let ids2 = ids.clone();
                    let n2 = name.clone();
                    net.fetch(l, move |c| async move {
                        let v = c.history_variable_for_instances(&n2, &ids2).await?;
                        Ok(v.into_iter().filter_map(|x| x.process_instance_id.clone().map(|p| (p, x))).collect())
                    });
                }
                l.poll();
            }
            // preview of the focused row
            if let Some(r) = rows.get(self.cursor) {
                if self.preview_for != r.id {
                    self.preview_for = r.id.clone();
                    self.preview.reset();
                    let id = r.id.clone();
                    net.fetch(&mut self.preview, move |c| async move { c.history_variables(&id).await });
                }
            }
        }
        if self.preview.poll() {
            if let Some(v) = &self.preview.data {
                for x in v {
                    if !self.known_var_names.contains(&x.name) {
                        self.known_var_names.push(x.name.clone());
                    }
                }
                self.known_var_names.sort();
            }
        }
        // activity suggestions for the def in the query
        let def_key = query::parse(&self.query).terms.iter().find(|t| t.field == "def").map(|t| t.value.clone());
        if let Some(k) = def_key {
            if self.activities_for.as_deref() != Some(k.as_str()) {
                if let Some(d) = defs.iter().filter(|d| d.key == k).max_by_key(|d| d.version) {
                    if let Some(m) = app.diagram(&d.id) {
                        let acts: Vec<(String, String)> = m.flow_nodes().map(|(_, e)| (e.id.clone(), e.name.clone().unwrap_or_default())).collect();
                        self.activities_cache.insert(k.clone(), acts);
                        self.activities_for = Some(k);
                    }
                }
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let ctx = ui.ctx().clone();
        self.poll(app);
        let tokens = app.tokens.clone();
        let parsed = query::parse(&self.query);
        let mode = self.mode(&parsed);

        // ---- dock: preview ----
        let mut open: Option<(String, NavMode)> = None;
        let mut dock_tab = 0usize;
        let cursor_row = self.rows.data.as_ref().and_then(|r| r.get(self.cursor).cloned());
        let (main_rect, _) = super::dock(ui, app, "search-dock", &["Preview", "Syntax"], &mut dock_tab, |ui, tab, app| {
            if tab == 1 {
                ui.label(RichText::new(crate::t!("search-syntax-title")).strong());
                for (f, d) in query::FIELDS {
                    crate::widgets::kv_mono(ui, f, d);
                }
                ui.add_space(6.0);
                ui.label(RichText::new(crate::t!("search-syntax-operators")).weak().small());
                ui.label(RichText::new(crate::t!("search-examples")).strong());
                for ex in ["def:order_process var.amount>1000", "bk:ORD-2* inc:yes", "exec:Task_Validate state:finished started>-7d", "var.customer=\"Big Corp 1\" state:any"] {
                    if ui.link(RichText::new(ex).monospace().size(12.0)).clicked() {
                        self.query = ex.to_string();
                        self.run();
                    }
                }
                return;
            }
            match &cursor_row {
                None => crate::widgets::empty_state(ui, &crate::t!("search-preview-empty")),
                Some(r) => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(r.business_key.clone().unwrap_or_else(|| crate::util::short_uuid(&r.id))).strong());
                        pill(ui, state_label(&r.state), state_color(&r.state, &tokens));
                        if ui.small_button(crate::t!("common-open")).clicked() {
                            app.navigate(Nav::Instance { id: r.id.clone() }, NavMode::Here);
                        }
                    });
                    crate::widgets::kv_mono(ui, "id", &r.id);
                    crate::widgets::kv(ui, "definition", &format!("{} v{}", r.definition_key, r.version.unwrap_or(0)));
                    if let Some(s) = &r.start {
                        crate::widgets::kv(ui, "started", &format!("{} ({})", crate::util::fmt_local(s), crate::util::fmt_relative(s)));
                    }
                    if let Some(e) = &r.end {
                        crate::widgets::kv(ui, "ended", &crate::util::fmt_local(e));
                    }
                    ui.add_space(6.0);
                    ui.label(RichText::new(crate::t!("search-variables")).strong());
                    loader_status_inline(ui, &self.preview, &tokens);
                    if let Some(v) = &self.preview.data {
                        for x in v {
                            let preview = crate::util::value_preview(&x.value, x.type_.as_deref());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&x.name).strong().size(12.0));
                                ui.label(RichText::new(x.type_.clone().unwrap_or_default()).weak().small());
                                if ui.add(egui::Button::new(crate::t!("search-add-column")).small()).on_hover_text(crate::t!("search-add-column-tip")).clicked() && !self.var_columns.contains(&x.name) {
                                    self.var_columns.push(x.name.clone());
                                }
                            });
                            let rr = ui.add(egui::Label::new(RichText::new(&preview).monospace().size(11.5).color(tokens.fg1)).truncate().sense(egui::Sense::click()));
                            if rr.clicked() {
                                ctx.copy_text(crate::places::instance::edit_text(x));
                            }
                        }
                    }
                }
            }
        });

        let mut main_ui = crate::widgets::split::child_in(ui, main_rect);
        egui::CentralPanel::default().frame(egui::Frame::new().fill(tokens.bg0).inner_margin(16.0)).show(&mut main_ui, |ui| {
            let top = ui.max_rect();
            if self.rows.is_loading() {
                progress_line(ui, egui::Rect::from_min_size(top.min - egui::Vec2::new(16.0, 16.0), egui::Vec2::new(top.width() + 32.0, 2.0)), tokens.accent);
            }
            heading_row(ui, &crate::t!("cmd-search-instances"), None);
            ui.add_space(6.0);
            // ---- query bar ----
            let mut run_now = false;
            let edit_id = egui::Id::unique("search-query");
            ui.horizontal(|ui| {
                let edit = egui::TextEdit::singleline(&mut self.query).id(edit_id).hint_text(crate::t!("search-query-hint")).desired_width(ui.available_width() - 330.0).font(egui::TextStyle::Monospace);
                let out = edit.show(ui);
                let r = &out.response;
                if self.want_focus {
                    r.request_focus();
                    self.want_focus = false;
                }
                // autocomplete
                let caret: usize = out.state.cursor.char_range().map(|c| c.primary.index.0).unwrap_or(self.query.chars().count());
                let byte_caret = self.query.char_indices().nth(caret).map(|(b, _)| b).unwrap_or(self.query.len());
                let def_keys: Vec<(String, String)> = app.cache.defs.data.as_ref().map(|d| d.iter().map(|d| (d.key.clone(), d.name.clone().unwrap_or_default())).collect()).unwrap_or_default();
                let def_in_query = parsed.terms.iter().find(|t| t.field == "def").map(|t| t.value.clone()).unwrap_or_default();
                let acts = self.activities_cache.get(&def_in_query).cloned().unwrap_or_default();
                let sctx = query::SuggestCtx { def_keys: &def_keys, activities: &acts, variable_names: &self.known_var_names, users: &app.settings.recent_users };
                let suggestions = if r.has_focus() { query::suggest(&self.query, byte_caret, &sctx) } else { vec![] };
                let mut accept: Option<usize> = None;
                if r.has_focus() {
                    ui.input_mut(|i| {
                        if !suggestions.is_empty() {
                            if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                                self.suggest_idx = (self.suggest_idx + 1) % suggestions.len();
                            }
                            if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                                self.suggest_idx = (self.suggest_idx + suggestions.len() - 1) % suggestions.len();
                            }
                            if i.consume_key(Modifiers::NONE, Key::Tab) {
                                accept = Some(self.suggest_idx.min(suggestions.len() - 1));
                            }
                        }
                        if i.consume_key(Modifiers::NONE, Key::Enter) {
                            run_now = true;
                        }
                    });
                }
                if !suggestions.is_empty() {
                    self.suggest_idx = self.suggest_idx.min(suggestions.len() - 1);
                    let pos = egui::Pos2::new(r.rect.min.x, r.rect.max.y + 2.0);
                    egui::Area::new(edit_id.with("sugg")).order(egui::Order::Foreground).fixed_pos(pos).show(&ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.set_min_width(360.0);
                            for (i, s) in suggestions.iter().enumerate() {
                                let sel = i == self.suggest_idx;
                                let resp = ui.horizontal(|ui| {
                                    let fill = if sel { tokens.accent.gamma_multiply(0.2) } else { egui::Color32::TRANSPARENT };
                                    egui::Frame::new().fill(fill).corner_radius(3.0).inner_margin(egui::Margin::symmetric(6, 2)).show(ui, |ui| {
                                        ui.set_width(348.0);
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(&s.label).monospace().size(12.0));
                                            ui.label(RichText::new(&s.hint).weak().small());
                                        });
                                    })
                                });
                                let rr = ui.interact(resp.response.rect, edit_id.with(("s", i)), egui::Sense::click());
                                if rr.clicked() {
                                    accept = Some(i);
                                }
                            }
                            ui.label(RichText::new(crate::t!("search-suggest-hint")).weak().small());
                        });
                    });
                }
                if let Some(i) = accept {
                    let s = &suggestions[i];
                    let mut q = self.query.clone();
                    q.replace_range(s.range.0..s.range.1, &s.insert);
                    let new_caret = s.range.0 + s.insert.len();
                    self.query = q;
                    let mut st = out.state.clone();
                    let ci = self.query[..new_caret.min(self.query.len())].chars().count();
                    st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(ci))));
                    st.store(&ctx, edit_id);
                    r.request_focus();
                }
                if ui.add(egui::Button::new(RichText::new(format!("{} ↵", crate::t!("common-search"))).color(egui::Color32::WHITE)).fill(tokens.accent)).clicked() {
                    run_now = true;
                }
                // mode pill
                let auto = self.force_mode.is_none();
                let label = match mode {
                    Mode::Runtime => crate::t!("search-mode-runtime"),
                    Mode::History => crate::t!("search-mode-history"),
                };
                let p = pill(ui, &format!("{label}{}", if auto { format!(" ({})", crate::t!("search-mode-auto")) } else { String::new() }), if mode == Mode::History { tokens.completed } else { tokens.running }).on_hover_text(crate::t!("search-mode-tip"));
                if p.clicked() {
                    self.force_mode = match self.force_mode {
                        None => Some(if mode == Mode::Runtime { Mode::History } else { Mode::Runtime }),
                        Some(_) => None,
                    };
                    self.run();
                }
                // saved searches
                let sv = ui.button("☆").on_hover_text(crate::t!("settings-saved-searches"));
                egui::Popup::menu(&sv).show(|ui| {
                    if ui.button(crate::t!("search-save-query")).clicked() {
                        self.saving = true;
                        self.save_name = query::describe(&parsed);
                    }
                    ui.separator();
                    let saved = app.settings.saved_searches.clone();
                    if saved.is_empty() {
                        ui.label(RichText::new(crate::t!("search-no-saved")).weak());
                    }
                    for (i, s) in saved.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.button(&s.name).on_hover_text(&s.query).clicked() {
                                self.query = s.query.clone();
                                self.run();
                            }
                            if ui.small_button("✕").clicked() {
                                app.settings.saved_searches.remove(i);
                                app.settings_dirty = true;
                            }
                        });
                    }
                });
            });
            if self.saving {
                ui.horizontal(|ui| {
                    ui.label(format!("{}:", crate::t!("connect-name")));
                    let r = ui.add(egui::TextEdit::singleline(&mut self.save_name).desired_width(260.0));
                    if ui.button(crate::t!("common-save")).clicked() || (r.has_focus() && ui.input(|i| i.key_pressed(Key::Enter))) {
                        app.settings.saved_searches.push(crate::settings::SavedSearch { name: self.save_name.clone(), query: self.query.clone() });
                        app.settings_dirty = true;
                        self.saving = false;
                    }
                    if ui.button(crate::t!("common-cancel")).clicked() {
                        self.saving = false;
                    }
                });
            }
            if !parsed.warnings.is_empty() {
                ui.label(RichText::new(parsed.warnings.join(" · ")).color(tokens.suspended).small());
            }
            if run_now {
                self.run();
            }
            ui.add_space(6.0);
            // ---- result header ----
            ui.horizontal(|ui| {
                if let Some(n) = self.count.data {
                    ui.label(RichText::new(crate::t!("search-results", n = n)).strong());
                }
                if !self.executed.is_empty() {
                    ui.label(RichText::new(query::describe(&query::parse(&self.executed))).weak().small());
                }
                loader_status_inline(ui, &self.rows, &tokens);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    app.refresh_controls(ui, self.rows.age(), self.rows.is_loading());
                    // variable column adder
                    let r = ui.add(egui::TextEdit::singleline(&mut self.new_var_col).hint_text(crate::t!("search-var-column-hint")).desired_width(140.0));
                    if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) && !self.new_var_col.trim().is_empty() {
                        let n = self.new_var_col.trim().to_string();
                        if !self.var_columns.contains(&n) {
                            self.var_columns.push(n);
                        }
                        self.new_var_col.clear();
                    }
                    for name in self.var_columns.clone() {
                        if pill(ui, &format!("{name} ✕"), tokens.external).on_hover_text(crate::t!("search-remove-column")).clicked() {
                            self.var_columns.retain(|c| c != &name);
                        }
                    }
                });
            });
            // bulk toolbar
            if !self.selected.is_empty() {
                self.bulk_toolbar(ui, app);
            }
            let Some(rows) = self.rows.data.clone() else {
                if self.executed.is_empty() {
                    crate::widgets::empty_state(ui, &crate::t!("search-empty-hint"));
                }
                if let Some(e) = &self.rows.error {
                    ui.label(RichText::new(e.to_string()).color(tokens.incident));
                }
                return;
            };
            if rows.is_empty() {
                crate::widgets::empty_state(ui, &crate::t!("search-no-match"));
                return;
            }
            self.cursor = self.cursor.min(rows.len() - 1);
            let text_focus = ctx.memory(|m| m.focused()).map(|id| egui::text_edit::TextEditState::load(&ctx, id).is_some()).unwrap_or(false);
            if !text_focus {
                ui.input_mut(|i| {
                    if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                        self.cursor = (self.cursor + 1).min(rows.len() - 1);
                    }
                    if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                        self.cursor = self.cursor.saturating_sub(1);
                    }
                    if i.consume_key(Modifiers::SHIFT, Key::Enter) {
                        open = Some((rows[self.cursor].id.clone(), NavMode::NewTab));
                    } else if i.consume_key(Modifiers::NONE, Key::Enter) {
                        open = Some((rows[self.cursor].id.clone(), NavMode::Here));
                    }
                    if i.consume_key(Modifiers::NONE, Key::Space) {
                        let id = rows[self.cursor].id.clone();
                        if !self.selected.remove(&id) {
                            self.selected.insert(id);
                        }
                    }
                    if i.consume_key(Modifiers::COMMAND, Key::A) {
                        if self.selected.len() == rows.len() {
                            self.selected.clear();
                        } else {
                            self.selected = rows.iter().map(|r| r.id.clone()).collect();
                        }
                    }
                    if i.consume_key(Modifiers::NONE, Key::Slash) {
                        self.want_focus = true;
                    }
                });
            }
            let var_cols = self.var_columns.clone();
            let var_values: VarColumns = var_cols.iter().filter_map(|n| self.var_values.get(n).and_then(|l| l.data.clone()).map(|d| (n.clone(), d))).collect();
            let shift = ui.input(|i| i.modifiers.shift);
            let h = ui.available_height() - 28.0;
            let min_w = 760.0 + 140.0 * var_cols.len() as f32;
            let mut toggle: Option<String> = None;
            table_area(ui, "search-table", min_w, |ui| {
            let mut table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .sense(egui::Sense::click())
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(22.0))
                .column(Column::initial(150.0).at_least(80.0))
                .column(Column::initial(90.0))
                .column(Column::initial(150.0).at_least(80.0))
                .column(Column::initial(90.0))
                .column(Column::initial(120.0))
                .column(Column::initial(100.0));
            for _ in &var_cols {
                table = table.column(Column::initial(140.0).at_least(60.0));
            }
            table = table.column(Column::remainder()).max_scroll_height(h.max(120.0)).scroll_to_row(self.cursor, None);
            table
                .header(24.0, |mut hr| {
                    hr.col(|ui| {
                        let all = self.selected.len() == rows.len();
                        let mut b = all;
                        if ui.checkbox(&mut b, "").clicked() {
                            if all {
                                self.selected.clear();
                            } else {
                                self.selected = rows.iter().map(|r| r.id.clone()).collect();
                            }
                        }
                    });
                    for t in ["Business key", "Id", "Definition", "State", "Started", "Ended / duration"] {
                        hr.col(|ui| {
                            ui.label(RichText::new(crate::i18n::col(t)).strong().size(12.0));
                        });
                    }
                    for v in &var_cols {
                        hr.col(|ui| {
                            ui.label(RichText::new(crate::t!("search-var-col", name = v.clone())).strong().size(12.0).color(tokens.external));
                        });
                    }
                    hr.col(|_| {});
                })
                .body(|body| {
                    body.rows(26.0, rows.len(), |mut row| {
                        let i = row.index();
                        let r = &rows[i];
                        row.set_selected(i == self.cursor);
                        row.col(|ui| {
                            let mut c = self.selected.contains(&r.id);
                            if ui.checkbox(&mut c, "").clicked() {
                                toggle = Some(r.id.clone());
                            }
                        });
                        row.col(|ui| match &r.business_key {
                            Some(bk) => {
                                id_cell(ui, bk, bk);
                            }
                            None => {
                                cell(ui, RichText::new("—").weak().size(12.0));
                            }
                        });
                        row.col(|ui| {
                            id_cell(ui, &r.id, &crate::util::short_uuid(&r.id));
                        });
                        row.col(|ui| {
                            cell(ui, RichText::new(format!("{} v{}", r.definition_key, r.version.unwrap_or(0))).size(12.0));
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(state_label(&r.state)).color(state_color(&r.state, &tokens)).size(12.0));
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(r.start.as_deref().map(crate::util::fmt_local_short).unwrap_or_default()).size(12.0)).on_hover_text(r.start.as_deref().map(crate::util::fmt_local).unwrap_or_default());
                        });
                        row.col(|ui| {
                            let t = match (&r.end, r.duration_ms) {
                                (Some(e), Some(d)) => format!("{} · {}", crate::util::fmt_local_short(e), crate::util::human_millis(d)),
                                (Some(e), None) => crate::util::fmt_local_short(e),
                                (None, _) => r.start.as_deref().and_then(crate::util::parse_time).map(|s| crate::t!("running-for", duration = crate::util::human_secs(chrono::Utc::now().signed_duration_since(s).num_seconds()))).unwrap_or_default(),
                            };
                            ui.label(RichText::new(t).size(12.0));
                        });
                        for v in &var_cols {
                            row.col(|ui| {
                                let val = var_values.get(v).and_then(|m| m.get(&r.id));
                                match val {
                                    Some(x) => {
                                        let p = crate::util::value_preview(&x.value, x.type_.as_deref());
                                        let rr = ui.add(egui::Label::new(RichText::new(&p).monospace().size(11.5)).truncate().sense(egui::Sense::click())).on_hover_text(&p);
                                        if rr.clicked() {
                                            ui.ctx().copy_text(p.clone());
                                        }
                                    }
                                    None => {
                                        ui.label(RichText::new("—").weak());
                                    }
                                }
                            });
                        }
                        row.col(|_| {});
                        let resp = row.response();
                        if resp.clicked() {
                            self.cursor = i;
                            if shift {
                                open = Some((r.id.clone(), NavMode::NewTab));
                            }
                        }
                        if resp.double_clicked() {
                            open = Some((r.id.clone(), NavMode::Here));
                        }
                        if resp.middle_clicked() {
                            open = Some((r.id.clone(), NavMode::NewTab));
                        }
                        resp.context_menu(|ui| {
                            if ui.button(crate::t!("common-open")).clicked() {
                                open = Some((r.id.clone(), NavMode::Here));
                                ui.close();
                            }
                            if ui.button(crate::t!("common-open-new-tab")).clicked() {
                                open = Some((r.id.clone(), NavMode::NewTab));
                                ui.close();
                            }
                            if ui.button(crate::t!("ctx-copy-id")).clicked() {
                                ui.ctx().copy_text(r.id.clone());
                                ui.close();
                            }
                            if let Some(bk) = &r.business_key {
                                if ui.button(crate::t!("common-copy-business-key")).clicked() {
                                    ui.ctx().copy_text(bk.clone());
                                    ui.close();
                                }
                            }
                            if ui.button(crate::t!("common-open-definition")).clicked() {
                                app.navigate(Nav::Definition { id: r.definition_id.clone() }, NavMode::Here);
                                ui.close();
                            }
                        });
                    });
                });
            });
            if let Some(id) = toggle {
                if !self.selected.remove(&id) {
                    self.selected.insert(id);
                }
            }
            // paging
            if let Some(n) = self.count.data {
                let ps = app.settings.page_size as i64;
                if n > ps {
                    ui.horizontal(|ui| {
                        if ui.add_enabled(self.page > 0, egui::Button::new(format!("‹ {}", crate::t!("common-prev")))).clicked() {
                            self.page -= 1;
                        }
                        ui.label(RichText::new(format!("{}–{} of {n}", self.page as i64 * ps + 1, ((self.page as i64 + 1) * ps).min(n))).weak());
                        if ui.add_enabled(((self.page as i64 + 1) * ps) < n, egui::Button::new(format!("{} ›", crate::t!("common-next")))).clicked() {
                            self.page += 1;
                        }
                    });
                }
            }
            app.status_hint = crate::t!("hint-search");
        });
        if let Some((id, mode)) = open {
            app.navigate(Nav::Instance { id }, mode);
        }
    }

    fn bulk_toolbar(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let tokens = app.tokens.clone();
        let ids: Vec<String> = self.selected.iter().cloned().collect();
        let n = ids.len();
        egui::Frame::new().fill(tokens.accent.gamma_multiply(0.12)).corner_radius(6.0).inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(crate::t!("search-selected", n = n)).strong());
                let batch = |app: &mut App, title: String, body: String, label: String, f: BatchFn| {
                    let ids2 = ids.clone();
                    app.confirm = Some(Confirm::new(title, body, label, move |app: &mut App| f(app, ids2)));
                };
                if ui.button(crate::t!("common-suspend")).clicked() {
                    batch(app, crate::t!("batch-suspend-title"), crate::t!("batch-suspend-body", n = n), crate::t!("common-suspend"), Box::new(|app, ids| run_batch(app, move |c| async move { c.suspend_async(&ids, true).await })));
                }
                if ui.button(crate::t!("common-activate")).clicked() {
                    batch(app, crate::t!("batch-activate-title"), crate::t!("batch-activate-body", n = n), crate::t!("common-activate"), Box::new(|app, ids| run_batch(app, move |c| async move { c.suspend_async(&ids, false).await })));
                }
                if ui.button(crate::t!("batch-retry-jobs")).clicked() {
                    batch(app, crate::t!("batch-retry-title"), crate::t!("batch-retry-body", n = n), crate::t!("common-retry"), Box::new(|app, ids| run_batch(app, move |c| async move { c.set_retries_async(&ids, 1).await })));
                }
                if ui.button(crate::t!("batch-set-variable")).clicked() {
                    self.new_var_col = "name=value".into();
                    app.toasts.push(ToastKind::Info, crate::t!("batch-set-variable-hint"));
                }
                if ui.button(RichText::new(crate::t!("batch-delete")).color(tokens.incident)).clicked() {
                    batch(app, crate::t!("batch-delete-instances-title"), crate::t!("batch-delete-instances-body", n = n), crate::t!("common-delete"), Box::new(|app, ids| run_batch(app, move |c| async move { c.delete_async(&ids, "deleted via Flightdeck", false, false).await })));
                }
                if ui.button(crate::t!("search-clear-selection")).clicked() {
                    self.selected.clear();
                }
            });
        });
        // set variable via the "name=value" field
        if let Some((k, v)) = self.new_var_col.split_once('=') {
            if ui.input(|i| i.key_pressed(Key::Enter)) && !k.trim().is_empty() && k.trim() != "name" {
                let name = k.trim().to_string();
                let val = v.trim().to_string();
                let ids2 = ids.clone();
                let vars = serde_json::json!({ name.clone(): {"value": val, "type": "String"} });
                run_batch(app, move |c| async move { c.set_variables_async(&ids2, &vars).await });
                self.new_var_col.clear();
            }
        }
    }
}

fn run_batch<F, Fut>(app: &mut App, f: F)
where
    F: FnOnce(std::sync::Arc<camunda::Client>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<camunda::Batch, camunda::ApiError>> + Send + 'static,
{
    let Some(net) = &app.net else { return };
    net.run(f, |app, r| match r {
        Ok(b) => {
            app.toasts.push(ToastKind::Success, crate::t!("toast-batch-started", id = crate::util::short_uuid(&b.id), jobs = b.total_jobs));
            app.bump_refresh();
        }
        Err(e) => {
            app.toasts.error(crate::t!("toast-batch-start-failed"), Some(e.to_string()));
        }
    });
}
