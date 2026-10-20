//! Batches: progress of asynchronous bulk operations.

use crate::app::App;
use crate::net::Loader;
use crate::widgets::{heading_row, loader_status_inline, pill, progress_line, Confirm};
use camunda::Batch;
use egui::RichText;
use std::time::Duration;

#[derive(Default)]
pub struct BatchesPlace {
    pub running: Loader<Vec<Batch>>,
    pub history: Loader<Vec<Batch>>,
    seen_epoch: u64,
}

impl BatchesPlace {
    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        if self.seen_epoch != app.refresh_epoch {
            self.seen_epoch = app.refresh_epoch;
            self.running.invalidate();
            self.history.invalidate();
        }
        if let Some(net) = &app.net {
            let r = Duration::from_secs(3);
            if self.running.stale(r) && (self.running.age().is_none() || app.auto_refresh_allowed()) {
                net.fetch(&mut self.running, |c| async move { c.batches().await });
            }
            if self.history.stale(Duration::from_secs(15)) && (self.history.age().is_none() || app.auto_refresh_allowed()) {
                net.fetch(&mut self.history, |c| async move { c.history_batches().await });
            }
        }
        self.running.poll();
        self.history.poll();
        let tokens = app.tokens.clone();
        egui::CentralPanel::default().frame(egui::Frame::new().fill(tokens.bg0).inner_margin(16.0)).show(ui, |ui| {
            let top = ui.max_rect();
            if self.running.is_loading() {
                progress_line(ui, egui::Rect::from_min_size(top.min - egui::Vec2::new(16.0, 16.0), egui::Vec2::new(top.width() + 32.0, 2.0)), tokens.accent);
            }
            ui.horizontal(|ui| {
                heading_row(ui, "Batches", None);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    app.refresh_controls(ui, self.running.age(), self.running.is_loading());
                });
            });
            loader_status_inline(ui, &self.running, &tokens);
            ui.add_space(8.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.label(RichText::new("Running").strong());
                match &self.running.data {
                    Some(list) if list.is_empty() => crate::widgets::empty_state(ui, "No running batches."),
                    Some(list) => {
                        for b in list.clone() {
                            egui::Frame::new().fill(tokens.bg2).stroke(egui::Stroke::new(1.0, tokens.bg3)).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    pill(ui, b.type_.as_deref().unwrap_or("batch"), tokens.accent);
                                    ui.label(RichText::new(crate::util::short_uuid(&b.id)).monospace()).on_hover_text(&b.id);
                                    if b.suspended {
                                        pill(ui, "suspended", tokens.suspended);
                                    }
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let id = b.id.clone();
                                        if ui.button(RichText::new("Delete").color(tokens.incident)).clicked() {
                                            app.confirm = Some(Confirm::new("Delete batch", "Delete this batch and its remaining jobs? Already executed jobs are not undone.", "Delete", move |app: &mut App| {
                                                let i2 = id.clone();
                                                app.run_action(move |c| async move { c.delete_batch(&i2, true).await }, "Batch deleted", "Could not delete batch");
                                            }));
                                        }
                                        let id = b.id.clone();
                                        let s = !b.suspended;
                                        if ui.button(if b.suspended { "Resume" } else { "Suspend" }).clicked() {
                                            app.run_action(move |c| async move { c.suspend_batch(&id, s).await }, "Batch updated", "Could not update batch");
                                        }
                                        ui.label(RichText::new(format!("started {}", b.start_time.as_deref().map(crate::util::fmt_relative).unwrap_or_default())).weak().small());
                                    });
                                });
                                let total = b.total_jobs.max(1) as f32;
                                let done = b.completed_jobs.unwrap_or(0) as f32;
                                let failed = b.failed_jobs.unwrap_or(0);
                                let frac = (done / total).clamp(0.0, 1.0);
                                let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(ui.available_width(), 10.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 5.0, tokens.bg3);
                                ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, egui::Vec2::new(rect.width() * frac, 10.0)), 5.0, if failed > 0 { tokens.suspended } else { tokens.completed });
                                ui.label(RichText::new(format!("{} of {} jobs done · {} remaining · {} failed · {} invocations per job", done as i64, b.total_jobs, b.remaining_jobs.unwrap_or(0), failed, b.invocations_per_batch_job)).small());
                                if let Some(u) = &b.create_user_id {
                                    ui.label(RichText::new(format!("created by {u}")).weak().small());
                                }
                            });
                            ui.add_space(4.0);
                        }
                    }
                    None => {
                        ui.spinner();
                    }
                }
                ui.add_space(12.0);
                ui.label(RichText::new("Finished (history)").strong());
                if let Some(list) = &self.history.data {
                    let finished: Vec<&Batch> = list.iter().filter(|b| b.end_time.is_some()).collect();
                    if finished.is_empty() {
                        crate::widgets::empty_state(ui, "No finished batches.");
                    }
                    egui::Grid::new("hist-batches").num_columns(5).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
                        for t in ["Type", "Id", "Jobs", "Started", "Ended"] {
                            ui.label(RichText::new(t).strong().size(11.5));
                        }
                        ui.end_row();
                        for b in finished {
                            ui.label(b.type_.clone().unwrap_or_default());
                            ui.label(RichText::new(crate::util::short_uuid(&b.id)).monospace().size(11.5)).on_hover_text(&b.id);
                            ui.label(b.total_jobs.to_string());
                            ui.label(b.start_time.as_deref().map(crate::util::fmt_local).unwrap_or_default());
                            ui.label(b.end_time.as_deref().map(crate::util::fmt_local).unwrap_or_default());
                            ui.end_row();
                        }
                    });
                }
            });
        });
    }
}
