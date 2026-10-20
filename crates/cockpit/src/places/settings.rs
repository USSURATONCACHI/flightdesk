//! Settings place: connections, appearance, input, refresh.

use crate::app::App;
use crate::theme::ThemeMode;
use crate::widgets::{heading_row, section};
use egui::RichText;

#[derive(Default)]
pub struct SettingsPlace {
    pub edit_engine: Option<usize>,
}

impl SettingsPlace {
    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let tokens = app.tokens.clone();
        egui::CentralPanel::default().frame(egui::Frame::new().fill(tokens.bg0).inner_margin(16.0)).show(ui, |ui| {
            heading_row(ui, "Settings", Some("saved automatically"));
            ui.add_space(8.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.set_max_width(760.0);
                let mut dirty = false;
                section(ui, "s-engines", "Connections", true, |ui| {
                    let n = app.settings.engines.len();
                    let mut remove: Option<usize> = None;
                    for i in 0..n {
                        let is_current = app.net.is_some() && app.settings.engines[i].url == app.profile.url;
                        let mut connect_req: Option<crate::settings::EngineProfile> = None;
                        egui::Frame::new().fill(tokens.bg2).stroke(egui::Stroke::new(1.0, if is_current { tokens.accent } else { tokens.bg3 })).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            let hint = app.settings.engines[i].cockpit_base();
                            {
                                let e = &mut app.settings.engines[i];
                                egui::Grid::new(("engine", i)).num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                                    ui.label("Name");
                                    dirty |= ui.add(egui::TextEdit::singleline(&mut e.name).desired_width(300.0)).changed();
                                    ui.end_row();
                                    ui.label("REST URL");
                                    dirty |= ui.add(egui::TextEdit::singleline(&mut e.url).desired_width(f32::INFINITY)).changed();
                                    ui.end_row();
                                    ui.label("User");
                                    dirty |= ui.add(egui::TextEdit::singleline(&mut e.user).desired_width(300.0)).changed();
                                    ui.end_row();
                                    ui.label("Password");
                                    ui.horizontal(|ui| {
                                        dirty |= ui.add(egui::TextEdit::singleline(&mut e.password).password(true).desired_width(300.0)).changed();
                                        dirty |= ui.checkbox(&mut e.remember_password, "remember").changed();
                                    });
                                    ui.end_row();
                                    ui.label("Cockpit URL");
                                    dirty |= ui.add(egui::TextEdit::singleline(&mut e.cockpit_url).hint_text(hint).desired_width(f32::INFINITY)).on_hover_text("Used by “Copy Cockpit link”. Leave empty to derive it from the REST URL.").changed();
                                    ui.end_row();
                                    ui.label("");
                                    dirty |= ui.checkbox(&mut e.accept_invalid_certs, "Allow self-signed TLS").changed();
                                    ui.end_row();
                                });
                            }
                            ui.horizontal(|ui| {
                                if !is_current {
                                    if ui.button("Connect").clicked() {
                                        connect_req = Some(app.settings.engines[i].clone());
                                    }
                                    if n > 1 && ui.button("Remove").clicked() {
                                        remove = Some(i);
                                    }
                                } else {
                                    ui.label(RichText::new("connected").color(tokens.completed));
                                }
                            });
                        });
                        if let Some(p) = connect_req {
                            app.connect.profile = p.clone();
                            app.connect.connecting = true;
                            app.disconnect();
                            app.connect_to(p);
                        }
                        ui.add_space(4.0);
                    }
                    if let Some(i) = remove {
                        app.settings.engines.remove(i);
                        dirty = true;
                    }
                    if ui.button("+ Add connection").clicked() {
                        app.settings.engines.push(crate::settings::EngineProfile { name: format!("engine {}", n + 1), ..Default::default() });
                        dirty = true;
                    }
                    dirty |= ui.checkbox(&mut app.settings.auto_connect, "Connect to the last engine on start-up").changed();
                });
                section(ui, "s-look", "Appearance", true, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Theme");
                        for (m, l) in [(ThemeMode::System, "System"), (ThemeMode::Light, "Light"), (ThemeMode::Dark, "Dark")] {
                            if ui.selectable_label(app.settings.theme == m, l).clicked() {
                                app.settings.theme = m;
                                app.frame_theme_dirty = true;
                                dirty = true;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Density");
                        for (d, l) in [(0.9, "Compact"), (1.0, "Normal"), (1.15, "Comfortable")] {
                            if ui.selectable_label((app.settings.density - d).abs() < 0.01, l).clicked() {
                                app.settings.density = d;
                                app.frame_theme_dirty = true;
                                dirty = true;
                            }
                        }
                    });
                });
                section(ui, "s-input", "Diagram input", true, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Mouse wheel");
                        if ui.selectable_label(app.settings.wheel_zooms, "zooms (Ctrl+wheel pans)").clicked() {
                            app.settings.wheel_zooms = true;
                            dirty = true;
                        }
                        if ui.selectable_label(!app.settings.wheel_zooms, "pans (Ctrl+wheel zooms)").clicked() {
                            app.settings.wheel_zooms = false;
                            dirty = true;
                        }
                    });
                    dirty |= ui.checkbox(&mut app.settings.inertia, "Inertia after flicking the diagram").changed();
                    ui.horizontal(|ui| {
                        ui.label("Unlimited drag");
                        use crate::viewport::WrapMode;
                        let supported = crate::platform::warp_supported();
                        for (m, l, tip) in [(WrapMode::Auto, "On", "Teleport the pointer to the opposite edge when a drag reaches the view border"), (WrapMode::Off, "Off", "Drags stop at the edge")] {
                            let enabled = supported || m == WrapMode::Off;
                            let r = ui.add_enabled_ui(enabled, |ui| ui.selectable_label(app.settings.pointer_wrap == m, l)).inner.on_hover_text(tip);
                            if r.clicked() {
                                app.settings.pointer_wrap = m;
                                dirty = true;
                            }
                        }
                        ui.label(RichText::new(crate::platform::describe()).weak().small());
                    });
                });
                section(ui, "s-refresh", "Refresh", true, |ui| {
                    egui::Grid::new("refresh-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                        ui.label("Instance views (seconds)");
                        let mut v = app.settings.refresh_instance_secs as f64;
                        if ui.add(egui::DragValue::new(&mut v).range(1.0..=300.0)).changed() {
                            app.settings.refresh_instance_secs = v as u64;
                            dirty = true;
                        }
                        ui.end_row();
                        ui.label("Lists (seconds)");
                        let mut v = app.settings.refresh_list_secs as f64;
                        if ui.add(egui::DragValue::new(&mut v).range(2.0..=600.0)).changed() {
                            app.settings.refresh_list_secs = v as u64;
                            dirty = true;
                        }
                        ui.end_row();
                        ui.label("Page size");
                        let mut v = app.settings.page_size as f64;
                        if ui.add(egui::DragValue::new(&mut v).range(20.0..=500.0)).changed() {
                            app.settings.page_size = v as usize;
                            dirty = true;
                        }
                        ui.end_row();
                    });
                    dirty |= ui.checkbox(&mut app.settings.pause_when_unfocused, "Pause auto-refresh while the window is not focused").changed();
                    dirty |= ui.checkbox(&mut app.settings.confirm_destructive, "Confirm destructive actions").changed();
                });
                section(ui, "s-mcp", "MCP server (AI agents)", true, |ui| {
                    ui.label(RichText::new("Exposes Flightdeck to AI agents over the Model Context Protocol (Streamable HTTP on localhost). Tools cover the engine (definitions, instances, variables, incidents, jobs, tasks, modification, batches) and the UI itself (navigate, select, zoom, dock, find, screenshots, keys).").weak().small());
                    ui.horizontal(|ui| {
                        let running = app.mcp.is_some();
                        let mut on = running;
                        if ui.checkbox(&mut on, "Enabled").changed() {
                            if on {
                                app.start_mcp();
                            } else {
                                app.stop_mcp();
                            }
                            app.settings.mcp_enabled = app.mcp.is_some();
                            dirty = true;
                        }
                        ui.label("Port");
                        let mut port = app.settings.mcp_port as f64;
                        if ui.add_enabled(!running, egui::DragValue::new(&mut port).range(1024.0..=65535.0)).changed() {
                            app.settings.mcp_port = port as u16;
                            dirty = true;
                        }
                        ui.label("Bearer token (optional)");
                        if ui.add_enabled(!running, egui::TextEdit::singleline(&mut app.settings.mcp_token).desired_width(180.0).hint_text("none")).changed() {
                            dirty = true;
                        }
                    });
                    if let Some(s) = &app.mcp {
                        let url = s.url();
                        let st = s.stats.lock().unwrap().clone();
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("listening on {url}")).color(tokens.completed));
                            if ui.small_button("Copy URL").clicked() {
                                ui.ctx().copy_text(url.clone());
                            }
                            let cfg = format!("{{\"mcpServers\": {{\"flightdeck\": {{\"type\": \"http\", \"url\": \"{url}\"}}}}}}");
                            if ui.small_button("Copy Claude Code config").on_hover_text(&cfg).clicked() {
                                ui.ctx().copy_text(cfg);
                            }
                        });
                        ui.label(RichText::new(format!("{} requests · {} tool calls · {} errors{}", st.requests, st.tool_calls, st.errors, st.last_tool.map(|t| format!(" · last: {t}")).unwrap_or_default())).weak().small());
                        ui.label(RichText::new(format!("Claude Code:  claude mcp add --transport http flightdeck {url}      stdio clients:  cockpit --mcp-stdio {url}")).monospace().size(11.0));
                    } else if let Some(e) = &app.mcp_error {
                        ui.label(RichText::new(e).color(tokens.incident).small());
                    }
                });
                section(ui, "s-saved", "Saved searches", false, |ui| {
                    let mut remove: Option<usize> = None;
                    for (i, s) in app.settings.saved_searches.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&s.name).strong());
                            ui.label(RichText::new(&s.query).monospace().size(11.5));
                            if ui.small_button("✕").clicked() {
                                remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = remove {
                        app.settings.saved_searches.remove(i);
                        dirty = true;
                    }
                    if app.settings.saved_searches.is_empty() {
                        ui.label(RichText::new("Save a query from the Search place (☆).").weak());
                    }
                });
                section(ui, "s-about", "About", false, |ui| {
                    ui.label("Flightdeck — a fast, keyboard-first cockpit for Camunda 7.");
                    ui.label(RichText::new(format!("version {} · egui {}", env!("CARGO_PKG_VERSION"), "0.37")).weak().small());
                    if let Some(p) = directories::ProjectDirs::from("", "", "flightdeck") {
                        ui.label(RichText::new(format!("settings: {}", p.config_dir().join("settings.toml").display())).weak().small());
                    }
                });
                if dirty {
                    app.settings_dirty = true;
                }
            });
        });
    }
}
