//! Settings place: connections, appearance, input, refresh.

use crate::app::App;
use crate::t;
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
            heading_row(ui, &t!("settings-title"), Some(&t!("settings-saved-automatically")));
            ui.add_space(8.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.set_max_width(760.0);
                let mut dirty = false;
                section(ui, "s-engines", &t!("settings-connections"), true, |ui| {
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
                                    ui.label(t!("connect-name"));
                                    dirty |= ui.add(egui::TextEdit::singleline(&mut e.name).desired_width(300.0)).changed();
                                    ui.end_row();
                                    ui.label(t!("connect-rest-url"));
                                    dirty |= ui.add(egui::TextEdit::singleline(&mut e.url).desired_width(f32::INFINITY)).changed();
                                    ui.end_row();
                                    ui.label(t!("connect-user"));
                                    dirty |= ui.add(egui::TextEdit::singleline(&mut e.user).desired_width(300.0)).changed();
                                    ui.end_row();
                                    ui.label(t!("connect-password"));
                                    ui.horizontal(|ui| {
                                        dirty |= ui.add(egui::TextEdit::singleline(&mut e.password).password(true).desired_width(300.0)).changed();
                                        dirty |= ui.checkbox(&mut e.remember_password, t!("connect-remember")).changed();
                                    });
                                    ui.end_row();
                                    ui.label(t!("connect-cockpit-url"));
                                    dirty |= ui.add(egui::TextEdit::singleline(&mut e.cockpit_url).hint_text(hint).desired_width(f32::INFINITY)).on_hover_text(t!("connect-cockpit-url-hint")).changed();
                                    ui.end_row();
                                    ui.label("");
                                    dirty |= ui.checkbox(&mut e.accept_invalid_certs, t!("connect-allow-self-signed")).changed();
                                    ui.end_row();
                                });
                            }
                            ui.horizontal(|ui| {
                                if !is_current {
                                    if ui.button(t!("connect-connect")).clicked() {
                                        connect_req = Some(app.settings.engines[i].clone());
                                    }
                                    if n > 1 && ui.button(t!("common-remove")).clicked() {
                                        remove = Some(i);
                                    }
                                } else {
                                    ui.label(RichText::new(t!("connect-connected")).color(tokens.completed));
                                }
                            });
                        });
                        if let Some(p) = connect_req {
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
                    if ui.button(t!("connect-add")).clicked() {
                        app.settings.engines.push(crate::settings::EngineProfile { name: t!("connect-default-name", n = n + 1), ..Default::default() });
                        dirty = true;
                    }
                    dirty |= ui.checkbox(&mut app.settings.auto_connect, t!("settings-auto-connect")).changed();
                });
                section(ui, "s-look", &t!("settings-appearance"), true, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(t!("settings-theme"));
                        for (m, l) in [(ThemeMode::System, t!("theme-system")), (ThemeMode::Light, t!("theme-light")), (ThemeMode::Dark, t!("theme-dark"))] {
                            if ui.selectable_label(app.settings.theme == m, l).clicked() {
                                app.settings.theme = m;
                                app.frame_theme_dirty = true;
                                dirty = true;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(t!("settings-language"));
                        let system_name = {
                            let sys = crate::i18n::system_languages();
                            crate::i18n::choose(crate::i18n::SYSTEM, &sys, &app.language_packs).map(|p| p.name.clone()).unwrap_or_else(|| "—".into())
                        };
                        let mut pick: Option<String> = None;
                        let is_system = app.settings.language == crate::i18n::SYSTEM;
                        if ui.selectable_label(is_system, format!("{} ({system_name})", t!("settings-language-system"))).clicked() && !is_system {
                            pick = Some(crate::i18n::SYSTEM.to_string());
                        }
                        for pack in &app.language_packs {
                            let on = app.settings.language.eq_ignore_ascii_case(&pack.tag);
                            if ui.selectable_label(on, &pack.name).on_hover_text(format!("{}\n{}", pack.tag, pack.path.display())).clicked() && !on {
                                pick = Some(pack.tag.clone());
                            }
                        }
                        if app.language_packs.is_empty() {
                            ui.label(RichText::new(t!("settings-language-none")).weak().small());
                        }
                        if let Some(p) = pick {
                            app.settings.language = p;
                            app.apply_language();
                            dirty = true;
                        }
                    });
                    if !app.language_report.warnings.is_empty() {
                        ui.label(RichText::new(t!("settings-language-warnings", n = app.language_report.warnings.len())).weak().small()).on_hover_text(app.language_report.warnings.join("\n"));
                    }
                    ui.horizontal(|ui| {
                        ui.label(t!("settings-density"));
                        for (d, l) in [(0.9, t!("density-compact")), (1.0, t!("density-normal")), (1.15, t!("density-comfortable"))] {
                            if ui.selectable_label((app.settings.density - d).abs() < 0.01, l).clicked() {
                                app.settings.density = d;
                                app.frame_theme_dirty = true;
                                dirty = true;
                            }
                        }
                    });
                });
                section(ui, "s-input", &t!("settings-diagram-input"), true, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(t!("settings-mouse-wheel"));
                        if ui.selectable_label(app.settings.wheel_zooms, t!("settings-wheel-zooms")).clicked() {
                            app.settings.wheel_zooms = true;
                            dirty = true;
                        }
                        if ui.selectable_label(!app.settings.wheel_zooms, t!("settings-wheel-pans")).clicked() {
                            app.settings.wheel_zooms = false;
                            dirty = true;
                        }
                    });
                    dirty |= ui.checkbox(&mut app.settings.inertia, t!("settings-inertia")).changed();
                    ui.add_space(4.0);
                    ui.label(t!("settings-zoom-speed"));
                    let (lo, hi) = crate::settings::ZOOM_SPEED_RANGE;
                    let zoom_row = |ui: &mut egui::Ui, label: String, v: &mut f32, tip: String, default: f32| -> bool {
                        let mut changed = false;
                        ui.horizontal(|ui| {
                            ui.add_sized([110.0, 20.0], egui::Label::new(label)).on_hover_text(&tip);
                            // Wheel: logarithmic so halving and doubling take the same distance.
                            changed |= ui.add(egui::Slider::new(v, lo..=hi).logarithmic(true).show_value(false)).on_hover_text(&tip).changed();
                            changed |= ui.add(egui::DragValue::new(v).range(lo..=hi).speed(0.02).fixed_decimals(2).suffix("×")).changed();
                            if (*v - default).abs() > 1e-6 && ui.small_button(t!("common-reset")).on_hover_text(t!("settings-reset-to", value = crate::viewport::fmt_multiplier(default))).clicked() {
                                *v = default;
                                changed = true;
                            }
                        });
                        changed
                    };
                    let d = crate::settings::Settings::default();
                    dirty |= zoom_row(ui, t!("settings-zoom-global"), &mut app.settings.zoom_speed, t!("settings-zoom-global-tip"), d.zoom_speed);
                    dirty |= zoom_row(ui, t!("settings-zoom-mouse"), &mut app.settings.zoom_speed_mouse, t!("settings-zoom-mouse-tip"), d.zoom_speed_mouse);
                    dirty |= zoom_row(ui, t!("settings-zoom-keyboard"), &mut app.settings.zoom_speed_keyboard, t!("settings-zoom-keyboard-tip"), d.zoom_speed_keyboard);
                    dirty |= zoom_row(ui, t!("settings-zoom-alt"), &mut app.settings.alt_speed, t!("settings-zoom-alt-tip"), d.alt_speed);
                    ui.label(RichText::new(t!("settings-zoom-effective", mouse = crate::viewport::fmt_multiplier(app.settings.zoom_mouse()), keyboard = crate::viewport::fmt_multiplier(app.settings.zoom_keyboard()), alt = crate::viewport::fmt_multiplier(app.settings.alt_boost()))).weak().small());
                    ui.horizontal(|ui| {
                        ui.label(t!("settings-unlimited-drag"));
                        use crate::viewport::WrapMode;
                        let supported = crate::platform::warp_supported();
                        for (m, l, tip) in [(WrapMode::Auto, t!("common-on"), t!("settings-unlimited-drag-on-tip")), (WrapMode::Off, t!("common-off"), t!("settings-unlimited-drag-off-tip"))] {
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
                section(ui, "s-find", &t!("settings-find"), true, |ui| {
                    dirty |= ui.checkbox(&mut app.settings.find_instances, t!("settings-find-instances")).changed();
                    ui.add_enabled_ui(app.settings.find_instances, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            dirty |= ui.checkbox(&mut app.settings.find_business_keys, t!("settings-find-business-keys")).changed();
                            dirty |= ui.checkbox(&mut app.settings.find_ids, t!("settings-find-ids")).changed();
                        });
                    });
                });
                section(ui, "s-xml", &t!("settings-xml-editor"), true, |ui| {
                    dirty |= ui.add(egui::TextEdit::singleline(&mut app.settings.xml_editor).hint_text("code -g {file}:{line}:{col}").desired_width(f32::INFINITY).font(egui::TextStyle::Monospace)).changed();
                    ui.label(RichText::new(t!("settings-xml-editor-hint")).weak().small());
                });
                section(ui, "s-refresh", &t!("settings-refresh"), true, |ui| {
                    egui::Grid::new("refresh-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                        ui.label(t!("settings-refresh-instances"));
                        let mut v = app.settings.refresh_instance_secs as f64;
                        if ui.add(egui::DragValue::new(&mut v).range(1.0..=300.0)).changed() {
                            app.settings.refresh_instance_secs = v as u64;
                            dirty = true;
                        }
                        ui.end_row();
                        ui.label(t!("settings-refresh-lists"));
                        let mut v = app.settings.refresh_list_secs as f64;
                        if ui.add(egui::DragValue::new(&mut v).range(2.0..=600.0)).changed() {
                            app.settings.refresh_list_secs = v as u64;
                            dirty = true;
                        }
                        ui.end_row();
                        ui.label(t!("settings-page-size"));
                        let mut v = app.settings.page_size as f64;
                        if ui.add(egui::DragValue::new(&mut v).range(20.0..=500.0)).changed() {
                            app.settings.page_size = v as usize;
                            dirty = true;
                        }
                        ui.end_row();
                    });
                    dirty |= ui.checkbox(&mut app.settings.pause_refresh_unfocused, t!("settings-pause-unfocused")).changed();
                    dirty |= ui.checkbox(&mut app.settings.confirm_destructive, t!("settings-confirm-destructive")).changed();
                });
                section(ui, "s-mcp", &t!("settings-mcp"), true, |ui| {
                    ui.label(RichText::new(t!("settings-mcp-description")).weak().small());
                    ui.horizontal(|ui| {
                        let running = app.mcp.is_some();
                        let mut on = running;
                        if ui.checkbox(&mut on, t!("common-enabled")).changed() {
                            if on {
                                app.start_mcp();
                            } else {
                                app.stop_mcp();
                            }
                            app.settings.mcp_enabled = app.mcp.is_some();
                            dirty = true;
                        }
                        ui.label(t!("settings-mcp-port"));
                        let mut port = app.settings.mcp_port as f64;
                        if ui.add_enabled(!running, egui::DragValue::new(&mut port).range(1024.0..=65535.0)).changed() {
                            app.settings.mcp_port = port as u16;
                            dirty = true;
                        }
                        ui.label(t!("settings-mcp-token"));
                        if ui.add_enabled(!running, egui::TextEdit::singleline(&mut app.settings.mcp_token).desired_width(180.0).hint_text(t!("common-none"))).changed() {
                            dirty = true;
                        }
                    });
                    if let Some(s) = &app.mcp {
                        let url = s.url();
                        let st = s.stats.lock().unwrap().clone();
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(t!("settings-mcp-listening", url = url.clone())).color(tokens.completed));
                            if ui.small_button(t!("settings-mcp-copy-url")).clicked() {
                                ui.ctx().copy_text(url.clone());
                            }
                            let cfg = format!("{{\"mcpServers\": {{\"flightdeck\": {{\"type\": \"http\", \"url\": \"{url}\"}}}}}}");
                            if ui.small_button(t!("settings-mcp-copy-config")).on_hover_text(&cfg).clicked() {
                                ui.ctx().copy_text(cfg);
                            }
                        });
                        let last = st.last_tool.map(|tool| format!(" · {}", t!("settings-mcp-last", tool = tool))).unwrap_or_default();
                        ui.label(RichText::new(format!("{}{last}", t!("settings-mcp-stats", requests = st.requests, calls = st.tool_calls, errors = st.errors))).weak().small());
                        ui.label(RichText::new(format!("Claude Code:  claude mcp add --transport http flightdeck {url}      stdio:  flightdeck --mcp-stdio {url}")).monospace().size(11.0));
                    } else if let Some(e) = &app.mcp_error {
                        ui.label(RichText::new(e).color(tokens.incident).small());
                    }
                });
                section(ui, "s-saved", &t!("settings-saved-searches"), false, |ui| {
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
                        ui.label(RichText::new(t!("settings-saved-searches-empty")).weak());
                    }
                });
                section(ui, "s-about", &t!("settings-about"), false, |ui| {
                    ui.label(t!("settings-about-text"));
                    ui.label(RichText::new(format!("{} {} · egui {}", t!("common-version"), env!("CARGO_PKG_VERSION"), "0.37")).weak().small());
                    match &app.settings_path {
                        Some(p) => ui.label(RichText::new(format!("{}: {}", t!("settings-file"), p.display())).weak().small()),
                        None => ui.label(RichText::new(t!("settings-not-persisted")).weak().small()),
                    };
                    if let Some(tag) = crate::i18n::active_tag() {
                        ui.label(RichText::new(format!("{}: {tag}", t!("settings-language"))).weak().small());
                    }
                });
                if dirty {
                    app.settings_dirty = true;
                }
            });
        });
    }
}
