//! Home: process definitions with running/incident counts.

use crate::app::App;
use crate::state::{Nav, NavMode};
use crate::widgets::{badge, cell, filter_box, heading_row, progress_line, table_area};
use camunda::{ProcessDefinition, ProcessDefinitionStatistics};
use egui::{Key, Modifiers, RichText};
use egui_extras::{Column, TableBuilder};
use std::collections::HashMap;

pub struct DefinitionsPlace {
    pub filter: String,
    pub filter_focus: bool,
    pub cursor: usize,
    pub show_all_versions: bool,
    pub sort: (usize, bool),
}

impl Default for DefinitionsPlace {
    fn default() -> Self {
        Self { filter: String::new(), filter_focus: false, cursor: 0, show_all_versions: false, sort: (0, true) }
    }
}

#[derive(Clone)]
struct Row {
    def: ProcessDefinition,
    running: i64,
    incidents: i64,
    versions: usize,
}

impl DefinitionsPlace {
    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        app.ensure_definitions();
        egui::CentralPanel::default().frame(egui::Frame::new().fill(app.tokens.bg0).inner_margin(16.0)).show(ui, |ui| {
            let top = ui.max_rect();
            if app.cache.defs.is_loading() || app.cache.def_stats.is_loading() {
                progress_line(ui, egui::Rect::from_min_size(top.min - egui::Vec2::new(16.0, 16.0), egui::Vec2::new(top.width() + 32.0, 2.0)), app.tokens.accent);
            }
            ui.horizontal(|ui| {
                heading_row(ui, &crate::t!("definitions-title"), None);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.checkbox(&mut self.show_all_versions, crate::t!("common-all-versions"));
                    ui.add_space(8.0);
                    ui.add_sized([320.0, 24.0], |ui: &mut egui::Ui| filter_box(ui, egui::Id::unique("defs-filter"), &mut self.filter, &crate::t!("definitions-filter-hint"), &mut self.filter_focus));
                });
            });
            ui.add_space(8.0);
            if let Some(e) = &app.cache.defs.error {
                ui.label(RichText::new(crate::t!("definitions-load-failed", error = e.short())).color(app.tokens.incident));
            }
            let rows = self.rows(app);
            if rows.is_empty() && app.cache.defs.data.is_some() {
                crate::widgets::empty_state(ui, &if self.filter.is_empty() { crate::t!("definitions-empty") } else { crate::t!("common-nothing-matches-filter") });
                return;
            }
            self.cursor = self.cursor.min(rows.len().saturating_sub(1));
            // keyboard
            let text_focus = ctx.memory(|m| m.focused()).map(|id| egui::text_edit::TextEditState::load(ctx, id).is_some()).unwrap_or(false);
            let mut open: Option<(String, NavMode)> = None;
            ctx.input_mut(|i| {
                if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    self.cursor = (self.cursor + 1).min(rows.len().saturating_sub(1));
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    self.cursor = self.cursor.saturating_sub(1);
                }
                if !text_focus && i.consume_key(Modifiers::NONE, Key::Slash) {
                    self.filter_focus = true;
                }
                if let Some(r) = rows.get(self.cursor) {
                    if i.consume_key(Modifiers::SHIFT, Key::Enter) {
                        open = Some((r.def.id.clone(), NavMode::NewTab));
                    } else if i.consume_key(Modifiers::NONE, Key::Enter) {
                        open = Some((r.def.id.clone(), NavMode::Here));
                    }
                }
            });
            let tokens = app.tokens.clone();
            table_area(ui, "defs-table", 760.0, |ui| {
            let table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .sense(egui::Sense::click())
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(260.0).at_least(120.0))
                .column(Column::initial(220.0).at_least(100.0))
                .column(Column::initial(70.0))
                .column(Column::initial(90.0))
                .column(Column::initial(90.0))
                .column(Column::remainder())
                .scroll_to_row(self.cursor, None);
            table
                .header(26.0, |mut h| {
                    for (i, t) in ["Name", "Key", "Version", "Running", "Incidents", "State"].iter().enumerate() {
                        h.col(|ui| {
                            let arrow = if self.sort.0 == i { if self.sort.1 { " ▲" } else { " ▼" } } else { "" };
                            if ui.add(egui::Label::new(RichText::new(format!("{}{arrow}", crate::i18n::col(t))).strong().size(12.0)).sense(egui::Sense::click())).clicked() {
                                self.sort = if self.sort.0 == i { (i, !self.sort.1) } else { (i, true) };
                            }
                        });
                    }
                })
                .body(|body| {
                    body.rows(28.0, rows.len(), |mut row| {
                        let i = row.index();
                        let r = &rows[i];
                        row.set_selected(i == self.cursor);
                        row.col(|ui| {
                            cell(ui, r.def.display_name());
                        });
                        row.col(|ui| {
                            cell(ui, RichText::new(&r.def.key).monospace().size(12.0));
                        });
                        row.col(|ui| {
                            let v = if self.show_all_versions || r.versions <= 1 { format!("v{}", r.def.version) } else { format!("v{} (+{})", r.def.version, r.versions - 1) };
                            ui.label(v);
                        });
                        row.col(|ui| {
                            if r.running > 0 {
                                badge(ui, &r.running.to_string(), tokens.running);
                            } else {
                                ui.label(RichText::new("0").weak());
                            }
                        });
                        row.col(|ui| {
                            if r.incidents > 0 {
                                badge(ui, &r.incidents.to_string(), tokens.incident);
                            } else {
                                ui.label(RichText::new("—").weak());
                            }
                        });
                        row.col(|ui| {
                            if r.def.suspended {
                                ui.label(RichText::new(crate::t!("state-suspended")).color(tokens.suspended));
                            }
                        });
                        let resp = row.response();
                        if resp.clicked() {
                            self.cursor = i;
                            open = Some((r.def.id.clone(), if ui_shift(ctx) { NavMode::NewTab } else { NavMode::Here }));
                        }
                        if resp.middle_clicked() {
                            open = Some((r.def.id.clone(), NavMode::NewTab));
                        }
                        resp.context_menu(|ui| {
                            if ui.button(crate::t!("common-open")).clicked() {
                                open = Some((r.def.id.clone(), NavMode::Here));
                                ui.close();
                            }
                            if ui.button(crate::t!("common-open-new-tab")).clicked() {
                                open = Some((r.def.id.clone(), NavMode::NewTab));
                                ui.close();
                            }
                            if ui.button(crate::t!("common-search-instances")).clicked() {
                                app.navigate(Nav::Search { query: format!("def:{}", r.def.key) }, NavMode::Here);
                                ui.close();
                            }
                            if ui.button(crate::t!("definitions-copy-key")).clicked() {
                                ui.ctx().copy_text(r.def.key.clone());
                                ui.close();
                            }
                            if ui.button(crate::t!("definitions-copy-id")).clicked() {
                                ui.ctx().copy_text(r.def.id.clone());
                                ui.close();
                            }
                        });
                    });
                });
            });
            if let Some((id, mode)) = open {
                app.navigate(Nav::Definition { id }, mode);
            }
            app.status_hint = crate::t!("hint-list");
        });
    }

    fn rows(&self, app: &App) -> Vec<Row> {
        let Some(defs) = &app.cache.defs.data else { return vec![] };
        let stats: HashMap<&str, &ProcessDefinitionStatistics> = app.cache.def_stats.data.as_ref().map(|s| s.iter().map(|x| (x.id.as_str(), x)).collect()).unwrap_or_default();
        let all = app.cache.all_defs.data.as_deref().unwrap_or(&[]);
        let mut by_key: HashMap<&str, (i64, i64, usize)> = HashMap::new();
        for d in all {
            let s = stats.get(d.id.as_str());
            let e = by_key.entry(d.key.as_str()).or_default();
            e.0 += s.map(|s| s.instances).unwrap_or(0);
            e.1 += s.map(|s| s.incidents.iter().map(|i| i.incident_count).sum::<i64>()).unwrap_or(0);
            e.2 += 1;
        }
        let f = self.filter.to_lowercase();
        let source: Vec<&ProcessDefinition> = if self.show_all_versions { all.iter().collect() } else { defs.iter().collect() };
        let mut rows: Vec<Row> = source
            .into_iter()
            .filter(|d| f.is_empty() || d.key.to_lowercase().contains(&f) || d.name.as_deref().unwrap_or("").to_lowercase().contains(&f))
            .map(|d| {
                let s = stats.get(d.id.as_str());
                let (running, incidents, versions) = if self.show_all_versions {
                    (s.map(|s| s.instances).unwrap_or(0), s.map(|s| s.incidents.iter().map(|i| i.incident_count).sum()).unwrap_or(0), 1)
                } else {
                    by_key.get(d.key.as_str()).copied().unwrap_or((0, 0, 1))
                };
                Row { def: d.clone(), running, incidents, versions }
            })
            .collect();
        let (col, asc) = self.sort;
        rows.sort_by(|a, b| {
            let o = match col {
                0 => a.def.display_name().to_lowercase().cmp(&b.def.display_name().to_lowercase()),
                1 => a.def.key.cmp(&b.def.key),
                2 => a.def.version.cmp(&b.def.version),
                3 => a.running.cmp(&b.running),
                4 => a.incidents.cmp(&b.incidents),
                _ => a.def.suspended.cmp(&b.def.suspended),
            };
            if asc {
                o
            } else {
                o.reverse()
            }
        });
        rows
    }
}

pub fn ui_shift(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.modifiers.shift)
}
