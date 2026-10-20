//! Engine-wide incidents.

use crate::app::App;
use crate::net::Loader;
use crate::state::{Nav, NavMode};
use crate::widgets::{cell, filter_box, heading_row, id_cell, loader_status_inline, pill, progress_line, table_area};
use camunda::Incident;
use egui::{Key, Modifiers, RichText};
use egui_extras::{Column, TableBuilder};
use std::time::Duration;

#[derive(Default)]
pub struct IncidentsPlace {
    pub list: Loader<Vec<Incident>>,
    pub filter: String,
    pub filter_focus: bool,
    pub cursor: usize,
    pub group: bool,
    seen_epoch: u64,
}

impl IncidentsPlace {
    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let ctx = ui.ctx().clone();
        if self.seen_epoch != app.refresh_epoch {
            self.seen_epoch = app.refresh_epoch;
            self.list.invalidate();
        }
        app.ensure_definitions();
        if let Some(net) = &app.net {
            if self.list.stale(Duration::from_secs(app.settings.refresh_list_secs)) && (self.list.age().is_none() || app.auto_refresh_allowed()) {
                net.fetch(&mut self.list, |c| async move { c.incidents(&[("maxResults", "1000".to_string())]).await });
            }
        }
        self.list.poll();
        let tokens = app.tokens.clone();
        egui::CentralPanel::default().frame(egui::Frame::new().fill(tokens.bg0).inner_margin(16.0)).show(ui, |ui| {
            let top = ui.max_rect();
            if self.list.is_loading() {
                progress_line(ui, egui::Rect::from_min_size(top.min - egui::Vec2::new(16.0, 16.0), egui::Vec2::new(top.width() + 32.0, 2.0)), tokens.accent);
            }
            ui.horizontal(|ui| {
                heading_row(ui, "Incidents", self.list.data.as_ref().map(|l| format!("{} open", l.len())).as_deref());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    app.refresh_controls(ui, self.list.age(), self.list.is_loading());
                    ui.checkbox(&mut self.group, "Group by message");
                    ui.add_sized([320.0, 24.0], |ui: &mut egui::Ui| filter_box(ui, egui::Id::unique("inc-filter"), &mut self.filter, "Filter message, type, definition, activity", &mut self.filter_focus));
                });
            });
            loader_status_inline(ui, &self.list, &tokens);
            ui.add_space(8.0);
            let Some(list) = self.list.data.clone() else { return };
            let f = self.filter.to_lowercase();
            let keys: std::collections::HashMap<String, String> = app.cache.known_defs.iter().map(|(id, d)| (id.clone(), d.key.clone())).collect();
            let def_key = move |id: &Option<String>| -> String { id.as_ref().and_then(|d| keys.get(d).cloned()).or_else(|| id.as_ref().map(|d| d.split(':').next().unwrap_or("").to_string())).unwrap_or_default() };
            let rows: Vec<&Incident> = list
                .iter()
                .filter(|i| {
                    f.is_empty() || i.incident_message.as_deref().unwrap_or("").to_lowercase().contains(&f) || i.incident_type.to_lowercase().contains(&f) || def_key(&i.process_definition_id).to_lowercase().contains(&f) || i.activity_id.as_deref().unwrap_or("").to_lowercase().contains(&f)
                })
                .collect();
            if rows.is_empty() {
                crate::widgets::empty_state(ui, if list.is_empty() { "No open incidents. 🎉" } else { "Nothing matches the filter." });
                return;
            }
            if self.group {
                let mut groups: Vec<(String, Vec<&Incident>)> = vec![];
                for i in &rows {
                    let key = format!("{} · {}", i.incident_type, crate::util::ellipsize(i.incident_message.as_deref().unwrap_or(""), 160));
                    match groups.iter_mut().find(|(k, _)| *k == key) {
                        Some((_, v)) => v.push(i),
                        None => groups.push((key, vec![i])),
                    }
                }
                groups.sort_by_key(|g| std::cmp::Reverse(g.1.len()));
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    for (k, v) in groups {
                        crate::widgets::section(ui, &k, &format!("{} × {}", v.len(), k), v.len() <= 3, |ui| {
                            for i in v {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(def_key(&i.process_definition_id)).monospace().size(11.5));
                                    ui.label(RichText::new(i.activity_id.clone().unwrap_or_default()).monospace().size(11.5));
                                    if let Some(pi) = &i.process_instance_id {
                                        if ui.link(RichText::new(crate::util::short_uuid(pi)).monospace().size(11.5)).clicked() {
                                            app.navigate(Nav::Instance { id: pi.clone() }, if ui.input(|x| x.modifiers.shift) { NavMode::NewTab } else { NavMode::Here });
                                        }
                                    }
                                    ui.label(RichText::new(i.time().map(crate::util::fmt_relative).unwrap_or_default()).weak().small());
                                    if let ("failedJob", Some(j)) = (i.incident_type.as_str(), i.configuration.clone()) {
                                        if ui.small_button("Retry").clicked() {
                                            app.action_set_job_retries(j, 1);
                                        }
                                    }
                                });
                            }
                        });
                    }
                });
                return;
            }
            self.cursor = self.cursor.min(rows.len() - 1);
            let text_focus = ctx.memory(|m| m.focused()).map(|id| egui::text_edit::TextEditState::load(&ctx, id).is_some()).unwrap_or(false);
            let mut open: Option<(String, NavMode)> = None;
            if !text_focus {
                ui.input_mut(|i| {
                    if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                        self.cursor = (self.cursor + 1).min(rows.len() - 1);
                    }
                    if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                        self.cursor = self.cursor.saturating_sub(1);
                    }
                    if let Some(pi) = rows[self.cursor].process_instance_id.clone() {
                        if i.consume_key(Modifiers::SHIFT, Key::Enter) {
                            open = Some((pi, NavMode::NewTab));
                        } else if i.consume_key(Modifiers::NONE, Key::Enter) {
                            open = Some((pi, NavMode::Here));
                        }
                    }
                    if i.consume_key(Modifiers::NONE, Key::Slash) {
                        self.filter_focus = true;
                    }
                });
            }
            let shift = ui.input(|i| i.modifiers.shift);
            table_area(ui, "inc-table", 900.0, |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .sense(egui::Sense::click())
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(110.0))
                .column(Column::initial(130.0))
                .column(Column::initial(150.0))
                .column(Column::initial(160.0))
                .column(Column::remainder().at_least(200.0))
                .column(Column::initial(100.0))
                .column(Column::initial(110.0))
                .scroll_to_row(self.cursor, None)
                .header(24.0, |mut h| {
                    for t in ["Time", "Type", "Definition", "Activity", "Message", "Instance", ""] {
                        h.col(|ui| {
                            ui.label(RichText::new(t).strong().size(12.0));
                        });
                    }
                })
                .body(|body| {
                    body.rows(26.0, rows.len(), |mut row| {
                        let i = row.index();
                        let inc = rows[i];
                        row.set_selected(i == self.cursor);
                        row.col(|ui| {
                            ui.label(RichText::new(inc.time().map(crate::util::fmt_relative).unwrap_or_default()).size(12.0)).on_hover_text(inc.time().map(crate::util::fmt_local).unwrap_or_default());
                        });
                        row.col(|ui| {
                            pill(ui, &inc.incident_type, tokens.incident);
                        });
                        row.col(|ui| {
                            cell(ui, RichText::new(def_key(&inc.process_definition_id)).monospace().size(11.5));
                        });
                        row.col(|ui| {
                            cell(ui, RichText::new(inc.activity_id.clone().unwrap_or_default()).monospace().size(11.5));
                        });
                        row.col(|ui| {
                            let m = inc.incident_message.clone().unwrap_or_default();
                            ui.add(egui::Label::new(RichText::new(&m).size(12.0)).truncate()).on_hover_text(&m);
                        });
                        row.col(|ui| {
                            if let Some(pi) = &inc.process_instance_id {
                                id_cell(ui, pi, &crate::util::short_uuid(pi));
                            }
                        });
                        row.col(|ui| {
                            let retry = |ui: &mut egui::Ui| ui.small_button("Retry").on_hover_text("Set retries to 1").clicked();
                            match (inc.incident_type.as_str(), inc.configuration.clone()) {
                                ("failedJob", Some(j)) if retry(ui) => app.action_set_job_retries(j, 1),
                                ("failedExternalTask", Some(e)) if retry(ui) => app.action_set_external_task_retries(e, 1),
                                _ => {}
                            }
                        });
                        let resp = row.response();
                        if resp.clicked() {
                            self.cursor = i;
                            if let Some(pi) = &inc.process_instance_id {
                                open = Some((pi.clone(), if shift { NavMode::NewTab } else { NavMode::Here }));
                            }
                        }
                        if resp.middle_clicked() {
                            if let Some(pi) = &inc.process_instance_id {
                                open = Some((pi.clone(), NavMode::NewTab));
                            }
                        }
                    });
                });
            });
            if let Some((id, mode)) = open {
                app.navigate(Nav::Instance { id }, mode);
            }
            app.status_hint = "↑↓ move · Enter open instance · Shift+Enter new tab · / filter".into();
        });
    }
}
