//! The application shell: tabs, top bar, status line, overlays and the navigation model.

use std::path::PathBuf;
use crate::commands::{self, Cmd, Palette, PlaceRequest};
use crate::t;
use crate::net::{Loader, Net};
use crate::places::*;
use crate::places::connect::ConnectScreen;
use crate::settings::{EngineProfile, Settings};
use crate::state::{Nav, NavMode, Place, Tab};
use crate::theme::{ThemeMode, Tokens};
use crate::viewport::DiagramSettings;
use crate::widgets::{Confirm, ToastKind, Toasts};
use bpmn::Model;
use camunda::{Client, Config, ProcessDefinition, ProcessDefinitionStatistics};
use egui::RichText;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub enum ConnStatus {
    Disconnected,
    Connecting,
    Connected { version: String },
    Error(String),
}

/// Engine-wide cached data shared by all places.
#[derive(Default)]
pub struct Cache {
    pub defs: Loader<Vec<ProcessDefinition>>,
    pub all_defs: Loader<Vec<ProcessDefinition>>,
    pub def_stats: Loader<Vec<ProcessDefinitionStatistics>>,
    pub diagrams: HashMap<String, Loader<Arc<Model>>>,
    pub known_defs: HashMap<String, ProcessDefinition>,
}

impl Cache {
    pub fn def_by_id(&self, id: &str) -> Option<&ProcessDefinition> {
        self.known_defs.get(id)
    }
    pub fn remember_def(&mut self, d: ProcessDefinition) {
        self.known_defs.insert(d.id.clone(), d);
    }
    pub fn diagram_error(&self, id: &str) -> Option<String> {
        self.diagrams.get(id).and_then(|l| l.error.as_ref().map(|e| e.short()))
    }
}

pub struct App {
    pub settings: Settings,
    pub settings_dirty: bool,
    /// Where settings are persisted. `None` (the default, used by tests) never writes anything.
    pub settings_path: Option<std::path::PathBuf>,
    pub tokens: Tokens,
    pub net: Option<Net<App>>,
    pub conn: ConnStatus,
    pub profile: EngineProfile,
    pub latency_ms: Option<u128>,
    pub connect: ConnectScreen,
    pub offline: bool,
    pub tabs: Vec<Tab>,
    pub active: usize,
    next_tab_id: u64,
    pub cache: Cache,
    pub toasts: Toasts,
    pub palette: Palette,
    pub help_open: bool,
    pub confirm: Option<Confirm>,
    pub paused: bool,
    pub dock_visible: bool,
    pub refresh_epoch: u64,
    pub status_hint: String,
    pub place_request: Option<PlaceRequest>,
    pending_nav: Vec<(Nav, NavMode)>,
    pub screenshot: Option<ScreenshotPlan>,
    frame: u64,
    last_health: Option<Instant>,
    health_in_flight: bool,
    scale: f32,
    ui_scale_applied: f32,
    pub ctx: Option<egui::Context>,
    pub window_focused: bool,
    pub frame_theme_dirty: bool,
    pub startup_nav: Vec<Nav>,
    pub startup_dock: Option<usize>,
    /// `--theme` from the command line: applies to this run only and is never saved.
    pub theme_override: Option<ThemeMode>,
    /// Where language packs are searched (handed in by the binary; empty in tests).
    pub locale_dirs: Vec<PathBuf>,
    /// Packs found in `locale_dirs`, for the settings picker.
    pub language_packs: Vec<crate::i18n::Pack>,
    pub language_report: crate::i18n::Report,
    pub stats: bool,
    frame_times: Vec<f32>,
    pub mcp: Option<crate::mcp::McpServer>,
    /// Bind address for the MCP server (`--mcp-host`; default loopback).
    pub mcp_host: String,
    pub mcp_queue: crate::mcp::Queue,
    pub mcp_error: Option<String>,
    pub synthetic_events: Vec<egui::Event>,
    pub last_diagram_viewport: egui::Rect,
}

pub struct ScreenshotPlan {
    pub path: std::path::PathBuf,
    pub after_frames: u64,
    pub exit: bool,
    pub requested: bool,
}

impl App {
    pub fn new(egui_ctx: &egui::Context, settings: Settings, scale: f32) -> Self {
        crate::theme::install_fonts(egui_ctx);
        let tokens = Self::tokens_for(settings.theme, egui_ctx);
        tokens.apply(egui_ctx, settings.density);
        egui_ctx.set_zoom_factor(scale);
        let profile = settings.engine();
        Self {
            settings,
            settings_dirty: false,
            settings_path: None,
            tokens,
            net: None,
            conn: ConnStatus::Disconnected,
            profile,
            latency_ms: None,
            connect: ConnectScreen::default(),
            offline: false,
            tabs: vec![],
            active: 0,
            next_tab_id: 1,
            cache: Cache::default(),
            toasts: Toasts::default(),
            palette: Palette::default(),
            help_open: false,
            confirm: None,
            paused: false,
            dock_visible: true,
            refresh_epoch: 1,
            status_hint: String::new(),
            place_request: None,
            pending_nav: vec![],
            screenshot: None,
            frame: 0,
            last_health: None,
            health_in_flight: false,
            scale,
            ui_scale_applied: scale,
            ctx: Some(egui_ctx.clone()),
            window_focused: true,
            frame_theme_dirty: false,
            startup_nav: vec![],
            startup_dock: None,
            theme_override: None,
            locale_dirs: vec![],
            language_packs: vec![],
            language_report: Default::default(),
            stats: false,
            frame_times: Vec::new(),
            mcp: None,
            mcp_host: "127.0.0.1".into(),
            mcp_queue: Default::default(),
            mcp_error: None,
            synthetic_events: Vec::new(),
            last_diagram_viewport: egui::Rect::NOTHING,
        }
    }

    /// Whether the given mode currently means dark, consulting the OS for `System`.
    pub fn wants_dark(mode: ThemeMode, ctx: &egui::Context) -> bool {
        match mode {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System => ctx.system_theme().map(|t| t == egui::Theme::Dark).or_else(crate::theme::system::prefers_dark).unwrap_or(false),
        }
    }

    fn tokens_for(mode: ThemeMode, ctx: &egui::Context) -> Tokens {
        let dark = Self::wants_dark(mode, ctx);
        if dark {
            Tokens::dark()
        } else {
            Tokens::light()
        }
    }

    /// The theme mode in effect: a command-line override, else the saved setting.
    /// Frames rendered so far (diagnostics for agents and tests).
    pub fn frame_number(&self) -> u64 {
        self.frame
    }

    pub fn theme_mode(&self) -> ThemeMode {
        self.theme_override.unwrap_or(self.settings.theme)
    }

    pub fn apply_theme(&mut self, ctx: &egui::Context) {
        self.tokens = Self::tokens_for(self.theme_mode(), ctx);
        self.tokens.apply(ctx, self.settings.density);
    }

    pub fn set_theme_mode(&mut self, mode: ThemeMode) {
        self.theme_override = None;
        self.settings.theme = mode;
        self.settings_dirty = true;
        self.frame_theme_dirty = true;
    }

    /// Ctrl+Shift+T cycles system → light → dark.
    pub fn toggle_theme(&mut self) {
        self.set_theme_mode(match self.theme_mode() {
            ThemeMode::System => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::System,
        });
    }

    pub fn diagram_settings(&self) -> DiagramSettings {
        DiagramSettings { wheel_zooms: self.settings.wheel_zooms, inertia: self.settings.inertia, wrap: self.settings.pointer_wrap, zoom_mouse: self.settings.zoom_mouse(), zoom_keyboard: self.settings.zoom_keyboard(), alt_boost: self.settings.alt_boost(), ..Default::default() }
    }

    pub fn auto_refresh_allowed(&self) -> bool {
        !self.paused && (self.window_focused || !self.settings.pause_when_unfocused)
    }

    // ---------- connection ----------

    pub fn connect_to(&mut self, profile: EngineProfile) {
        self.connect_with(profile, None);
    }

    /// Connect using `profile`; on success it is stored in `slot` of the saved connections when
    /// given (the entry being edited), otherwise merged by URL and user.
    pub fn connect_with(&mut self, profile: EngineProfile, slot: Option<usize>) {
        let cfg = Config {
            base_url: profile.url.clone(),
            auth: if profile.user.is_empty() { camunda::Auth::None } else { camunda::Auth::Basic { user: profile.user.clone(), password: profile.password.clone() } },
            timeout: Duration::from_secs(30),
            accept_invalid_certs: profile.accept_invalid_certs,
        };
        let client = match Client::new(cfg) {
            Ok(c) => c,
            Err(e) => {
                self.connect.error = Some(e.to_string());
                self.connect.connecting = false;
                return;
            }
        };
        self.profile = profile.clone();
        self.conn = ConnStatus::Connecting;
        let ctx = self.ctx.clone().expect("ctx");
        let net = Net::new(client, ctx);
        let started = Instant::now();
        net.run(
            |c| async move { c.version().await },
            move |app: &mut App, r| {
                app.connect.connecting = false;
                match r {
                    Ok(v) => {
                        app.latency_ms = Some(started.elapsed().as_millis());
                        app.conn = ConnStatus::Connected { version: v.version.clone() };
                        app.connect.error = None;
                        // persist profile: the edited slot wins, otherwise merge by URL + user
                        let p = app.profile.clone();
                        let existing = slot.filter(|&i| i < app.settings.engines.len()).or_else(|| app.settings.engines.iter().position(|e| e.url == p.url && e.user == p.user));
                        match existing {
                            Some(i) => {
                                app.settings.engines[i] = p;
                                app.settings.last_engine = i;
                            }
                            None => {
                                app.settings.engines.push(p);
                                app.settings.last_engine = app.settings.engines.len() - 1;
                            }
                        }
                        app.settings_dirty = true;
                        app.cache = Cache::default();
                        app.bump_refresh();
                        if app.tabs.is_empty() || app.tabs.iter().all(|t| matches!(t.place(), Place::File(_) | Place::Settings(_))) {
                            app.navigate(Nav::Definitions, NavMode::NewTab);
                        }
                        app.toasts.push(ToastKind::Success, t!("toast-connected", version = v.version.clone()));
                        app.ensure_definitions();
                        for nav in std::mem::take(&mut app.startup_nav) {
                            app.navigate(nav, NavMode::Here);
                        }
                    }
                    Err(e) => {
                        app.conn = ConnStatus::Error(e.short());
                        app.connect.error = Some(match &e {
                            camunda::ApiError::Status { status: 401, .. } | camunda::ApiError::Status { status: 403, .. } => t!("error-auth"),
                            camunda::ApiError::Status { status: 404, .. } => t!("error-not-camunda"),
                            other => other.short(),
                        });
                        app.net = None;
                    }
                }
            },
        );
        self.net = Some(net);
    }

    pub fn disconnect(&mut self) {
        self.net = None;
        self.conn = ConnStatus::Disconnected;
        self.tabs.retain(|t| matches!(t.place(), Place::File(_)));
        self.active = 0;
        self.cache = Cache::default();
        self.connect.connecting = false;
        self.connect.error = None;
    }

    fn health_check(&mut self) {
        if self.net.is_none() || self.health_in_flight {
            return;
        }
        let due = self.last_health.map(|t| t.elapsed() > Duration::from_secs(15)).unwrap_or(true);
        if !due {
            return;
        }
        self.last_health = Some(Instant::now());
        self.health_in_flight = true;
        let started = Instant::now();
        if let Some(net) = &self.net {
            net.run(
                |c| async move { c.version().await },
                move |app: &mut App, r| {
                    app.health_in_flight = false;
                    match r {
                        Ok(v) => {
                            app.latency_ms = Some(started.elapsed().as_millis());
                            app.conn = ConnStatus::Connected { version: v.version };
                        }
                        Err(e) => app.conn = ConnStatus::Error(e.short()),
                    }
                },
            );
        }
    }

    // ---------- MCP ----------

    pub fn start_mcp(&mut self) {
        if self.mcp.is_some() {
            return;
        }
        let Some(ctx) = self.ctx.clone() else { return };
        let token = if self.settings.mcp_token.trim().is_empty() { None } else { Some(self.settings.mcp_token.trim().to_string()) };
        match crate::mcp::McpServer::start(&self.mcp_host, self.settings.mcp_port, token, self.mcp_queue.clone(), ctx) {
            Ok(s) => {
                self.mcp_error = None;
                self.toasts.push(ToastKind::Success, t!("toast-mcp-listening", url = s.url()));
                self.mcp = Some(s);
            }
            Err(e) => {
                self.mcp_error = Some(e.to_string());
                self.toasts.error(t!("toast-mcp-failed", port = self.settings.mcp_port), Some(e.to_string()));
            }
        }
    }

    pub fn stop_mcp(&mut self) {
        if let Some(s) = self.mcp.take() {
            s.stop();
            self.toasts.push(ToastKind::Info, t!("toast-mcp-stopped"));
        }
    }

    fn drain_mcp(&mut self) {
        let calls: Vec<crate::mcp::McpCall> = {
            let mut q = self.mcp_queue.lock().unwrap();
            q.drain(..).collect()
        };
        for c in calls {
            crate::mcp::tools::dispatch(self, c);
        }
    }

    // ---------- navigation ----------

    pub fn navigate(&mut self, nav: Nav, mode: NavMode) {
        self.pending_nav.push((nav, mode));
    }

    /// (Re)discover language packs in `locale_dirs` and activate the one the settings ask for.
    /// Called once at start-up and whenever the language setting changes.
    pub fn apply_language(&mut self) {
        let (packs, report) = crate::i18n::load(&self.locale_dirs, &self.settings.language);
        self.language_packs = packs;
        self.language_report = report;
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
    }

    /// Settings never replace what you were looking at: switch to an existing settings tab, or
    /// open one.
    pub fn open_settings(&mut self) {
        if let Some(i) = self.tabs.iter().position(|t| matches!(t.place(), Place::Settings(_))) {
            self.active = i;
        } else {
            self.navigate(Nav::Settings, NavMode::NewTab);
        }
    }

    fn build_place(&mut self, nav: Nav) -> Option<Place> {
        Some(match nav {
            Nav::Definitions => Place::Definitions(DefinitionsPlace::default()),
            Nav::Definition { id } => Place::Definition(Box::new(DefinitionPlace::new(id))),
            Nav::DefinitionKey { key } => {
                // Resolve latest version by key through the cache or a request.
                if let Some(d) = self.cache.defs.data.as_ref().and_then(|v| v.iter().find(|d| d.key == key)) {
                    Place::Definition(Box::new(DefinitionPlace::new(d.id.clone())))
                } else {
                    let k = key.clone();
                    if let Some(net) = &self.net {
                        net.run(
                            move |c| async move { c.process_definition_by_key(&k).await },
                            move |app: &mut App, r| match r {
                                Ok(d) => app.navigate(Nav::Definition { id: d.id }, NavMode::Here),
                                Err(e) => {
                                    app.toasts.error(t!("toast-no-definition-key", key = key.clone()), Some(e.to_string()));
                                }
                            },
                        );
                    }
                    return None;
                }
            }
            Nav::Instance { id } => Place::Instance(Box::new(InstancePlace::new(id))),
            Nav::Search { query } => Place::Search(Box::new(SearchPlace::new(query))),
            Nav::Incidents => Place::Incidents(Box::default()),
            Nav::Batches => Place::Batches(BatchesPlace::default()),
            Nav::Settings => Place::Settings(SettingsPlace::default()),
            Nav::File { path } => Place::File(Box::new(FilePlace::new(path))),
        })
    }

    fn apply_pending_nav(&mut self) {
        let pending = std::mem::take(&mut self.pending_nav);
        for (nav, mode) in pending {
            let Some(place) = self.build_place(nav) else { continue };
            match mode {
                NavMode::NewTab => {
                    let id = self.next_tab_id;
                    self.next_tab_id += 1;
                    self.tabs.push(Tab::new(id, place));
                    self.active = self.tabs.len() - 1;
                }
                NavMode::Here => {
                    if let Some(t) = self.tabs.get_mut(self.active) {
                        t.push(place);
                    } else {
                        let id = self.next_tab_id;
                        self.next_tab_id += 1;
                        self.tabs.push(Tab::new(id, place));
                        self.active = self.tabs.len() - 1;
                    }
                }
                NavMode::Replace => {
                    if let Some(t) = self.tabs.get_mut(self.active) {
                        t.replace(place);
                    }
                }
            }
        }
    }

    pub fn close_tab(&mut self, i: usize) {
        if self.tabs.len() <= 1 || i >= self.tabs.len() {
            return;
        }
        self.tabs.remove(i);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if i < self.active {
            self.active -= 1;
        }
    }

    pub fn bump_refresh(&mut self) {
        self.refresh_epoch += 1;
        self.cache.defs.invalidate();
        self.cache.all_defs.invalidate();
        self.cache.def_stats.invalidate();
    }

    pub fn copy_cockpit_link(&mut self) {
        let base = self.profile.cockpit_base();
        let link = match self.tabs.get(self.active).map(|t| t.place()) {
            Some(Place::Definition(p)) => Some(format!("{base}/process-definition/{}/runtime", p.def_id)),
            Some(Place::Instance(p)) => Some(format!("{base}/process-instance/{}/runtime", p.id)),
            _ => None,
        };
        if let (Some(l), Some(ctx)) = (link, &self.ctx) {
            ctx.copy_text(l.clone());
            self.toasts.push(ToastKind::Info, t!("toast-copied", what = l));
        }
    }

    pub fn copy_place_id(&mut self) {
        let id = match self.tabs.get(self.active).map(|t| t.place()) {
            Some(Place::Definition(p)) => Some(p.def_id.clone()),
            Some(Place::Instance(p)) => Some(p.id.clone()),
            _ => None,
        };
        if let (Some(id), Some(ctx)) = (id, &self.ctx) {
            ctx.copy_text(id.clone());
            self.toasts.push(ToastKind::Info, t!("toast-copied", what = id));
        }
    }

    // ---------- toolbar bits used by places ----------

    pub fn refresh_controls(&mut self, ui: &mut egui::Ui, age: Option<Duration>, loading: bool) {
        let paused = self.paused;
        if ui.add(egui::Button::new(if paused { "▶" } else { "‖" }).small()).on_hover_text(if paused { t!("refresh-resume-tip") } else { t!("refresh-pause-tip") }).clicked() {
            self.paused = !self.paused;
        }
        if ui.add(egui::Button::new("⟳").small()).on_hover_text(t!("refresh-now-tip")).clicked() {
            self.bump_refresh();
        }
        let txt = if loading {
            t!("refresh-updating")
        } else {
            match age {
                Some(a) => t!("refresh-updated", ago = crate::util::human_duration(a)),
                None => String::new(),
            }
        };
        ui.label(RichText::new(txt).weak().small());
        if age.is_some() {
            ui.ctx().request_repaint_after(Duration::from_secs(1));
        }
    }

    // ---------- actions shared by places ----------

    pub fn action_set_job_retries(&mut self, job_id: String, retries: i64) {
        let Some(net) = &self.net else { return };
        let j = job_id.clone();
        net.run(
            move |c| async move { c.set_job_retries(&j, retries).await },
            move |app, r| match r {
                Ok(()) => {
                    app.toasts.push(ToastKind::Success, t!("toast-job-retries", n = retries));
                    app.bump_refresh();
                }
                Err(e) => {
                    app.toasts.error(t!("toast-job-retries-failed"), Some(e.to_string()));
                }
            },
        );
    }

    pub fn action_set_external_task_retries(&mut self, id: String, retries: i64) {
        let Some(net) = &self.net else { return };
        let j = id.clone();
        net.run(
            move |c| async move { c.set_external_task_retries(&j, retries).await },
            move |app, r| match r {
                Ok(()) => {
                    app.toasts.push(ToastKind::Success, t!("toast-external-retries", n = retries));
                    app.bump_refresh();
                }
                Err(e) => {
                    app.toasts.error(t!("toast-external-retries-failed"), Some(e.to_string()));
                }
            },
        );
    }

    pub fn action_suspend_definition(&mut self, id: String, suspend: bool) {
        let short = crate::util::ellipsize(&id, 60);
        let (title, body, verb) = if suspend {
            (t!("definition-suspend-title"), t!("definition-suspend-body", id = short), t!("common-suspend"))
        } else {
            (t!("definition-activate-title"), t!("definition-activate-body", id = short), t!("common-activate"))
        };
        self.confirm = Some(Confirm::new(title, body, verb, move |app: &mut App| {
            let Some(net) = &app.net else { return };
            let i = id.clone();
            net.run(
                move |c| async move { c.suspend_definition(&i, suspend, true).await },
                move |app, r| match r {
                    Ok(()) => {
                        app.toasts.push(ToastKind::Success, if suspend { t!("toast-definition-suspended") } else { t!("toast-definition-activated") });
                        app.bump_refresh();
                    }
                    Err(e) => {
                        app.toasts.error(t!("toast-suspension-failed"), Some(e.to_string()));
                    }
                },
            );
        }));
    }

    pub fn run_action<T, F, Fut>(&mut self, f: F, success: impl Into<String>, failure: impl Into<String>)
    where
        T: Send + 'static,
        F: FnOnce(Arc<Client>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, camunda::ApiError>> + Send + 'static,
    {
        let Some(net) = &self.net else { return };
        let s = success.into();
        let fmsg = failure.into();
        net.run(f, move |app, r| match r {
            Ok(_) => {
                app.toasts.push(ToastKind::Success, s);
                app.bump_refresh();
            }
            Err(e) => {
                app.toasts.error(fmsg, Some(e.to_string()));
            }
        });
    }

    // ---------- frame ----------

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let tokens = self.tokens.clone();
        egui::Panel::top("topbar").frame(egui::Frame::new().fill(tokens.bg1).inner_margin(egui::Margin::symmetric(8, 4))).show(ui, |ui| {
            ui.horizontal(|ui| {
                // engine pill
                let (color, text) = match &self.conn {
                    ConnStatus::Connected { version } => (tokens.completed, format!("● {}  ·  {}{}", self.profile.name, version, self.latency_ms.map(|l| format!("  ·  {l} ms")).unwrap_or_default())),
                    ConnStatus::Connecting => (tokens.suspended, format!("● {}", t!("status-connecting"))),
                    ConnStatus::Error(e) => (tokens.incident, format!("● {}", crate::util::ellipsize(e, 40))),
                    ConnStatus::Disconnected => (tokens.fg2, format!("● {}", t!("status-offline"))),
                };
                let pill = ui.add(egui::Button::new(RichText::new(text).color(color).size(12.0)).frame(false));
                egui::Popup::menu(&pill).show(|ui| {
                    ui.label(RichText::new(&self.profile.url).monospace().small());
                    if ui.button(t!("topbar-refresh-everything")).clicked() {
                        self.bump_refresh();
                    }
                    if ui.button(t!("topbar-disconnect")).clicked() {
                        self.disconnect();
                    }
                });
                ui.separator();
                // tabs
                let mut close: Option<usize> = None;
                let mut activate: Option<usize> = None;
                egui::ScrollArea::horizontal().id_salt("tabs-scroll").max_width(ui.available_width() - 260.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (i, t) in self.tabs.iter().enumerate() {
                            let selected = i == self.active;
                            let title = t.place().title();
                            let fill = if selected { tokens.bg2 } else { egui::Color32::TRANSPARENT };
                            let frame = egui::Frame::new().fill(fill).corner_radius(5.0).inner_margin(egui::Margin::symmetric(8, 3));
                            let r = frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let label = ui.add(egui::Label::new(RichText::new(&title).color(if selected { tokens.fg0 } else { tokens.fg1 })).sense(egui::Sense::click()));
                                    if label.clicked() {
                                        activate = Some(i);
                                    }
                                    if label.middle_clicked() && self.tabs.len() > 1 {
                                        close = Some(i);
                                    }
                                    if self.tabs.len() > 1 && ui.add(egui::Button::new(RichText::new("×").size(11.0)).frame(false)).on_hover_text(t!("tab-close-tip")).clicked() {
                                        close = Some(i);
                                    }
                                })
                            });
                            if selected {
                                let rr = r.response.rect;
                                ui.painter().hline(rr.x_range(), rr.max.y + 1.0, egui::Stroke::new(2.0, tokens.accent));
                            }
                        }
                        if ui.add(egui::Button::new("+").frame(false)).on_hover_text(t!("tab-new-tip")).clicked() {
                            Cmd::NewTab.run(self);
                        }
                    });
                });
                if let Some(i) = activate {
                    self.active = i;
                }
                if let Some(i) = close {
                    self.close_tab(i);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new("?").frame(false)).on_hover_text(t!("topbar-shortcuts-tip")).clicked() {
                        self.help_open = !self.help_open;
                    }
                    let (mcp_color, mcp_text) = match &self.mcp {
                        Some(s) => (tokens.external, format!("MCP :{}", s.port)),
                        None => (tokens.fg2, t!("topbar-mcp-off")),
                    };
                    let mcp_btn = ui.add(egui::Button::new(RichText::new(mcp_text).color(mcp_color).size(12.0)).frame(false)).on_hover_text(t!("topbar-mcp-tip"));
                    if mcp_btn.clicked() {
                        if self.mcp.is_some() {
                            self.stop_mcp();
                            self.settings.mcp_enabled = false;
                        } else {
                            self.start_mcp();
                            self.settings.mcp_enabled = self.mcp.is_some();
                        }
                        self.settings_dirty = true;
                    }
                    if ui.add(egui::Button::new("⚙").frame(false)).on_hover_text(t!("topbar-settings-tip")).clicked() {
                        self.open_settings();
                    }
                    let theme_btn = ui.add(egui::Button::new(if self.tokens.dark { "☾" } else { "☀" }).frame(false)).on_hover_text(t!("topbar-theme-tip"));
                    egui::Popup::menu(&theme_btn).show(|ui| {
                        for (mode, label) in [(ThemeMode::System, t!("theme-system")), (ThemeMode::Light, t!("theme-light")), (ThemeMode::Dark, t!("theme-dark"))] {
                            if ui.selectable_label(self.theme_mode() == mode, label).clicked() {
                                self.set_theme_mode(mode);
                            }
                        }
                    });
                    let search = egui::Button::new(RichText::new(format!("🔍  {}   Ctrl+K", t!("topbar-search"))).color(tokens.fg1)).fill(tokens.bg2).min_size(egui::Vec2::new(200.0, 24.0));
                    if ui.add(search).clicked() {
                        self.palette.open();
                    }
                    // back / forward
                    let (cb, cf) = self.tabs.get(self.active).map(|t| (t.can_back(), t.can_forward())).unwrap_or((false, false));
                    if ui.add_enabled(cf, egui::Button::new("→").frame(false)).on_hover_text(t!("nav-forward-tip")).clicked() {
                        Cmd::Forward.run(self);
                    }
                    if ui.add_enabled(cb, egui::Button::new("←").frame(false)).on_hover_text(t!("nav-back-tip")).clicked() {
                        Cmd::Back.run(self);
                    }
                });
            });
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let tokens = self.tokens.clone();
        egui::Panel::bottom("statusbar").frame(egui::Frame::new().fill(tokens.bg1).inner_margin(egui::Margin::symmetric(10, 3))).show(ui, |ui| {
            ui.horizontal(|ui| {
                // Right side first so the hint gets only the remaining width and is truncated
                // (full text on hover) instead of running underneath the status labels.
                let hint = self.status_hint.clone();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let inflight = self.net.as_ref().map(|n| n.in_flight()).unwrap_or(0);
                    if inflight > 0 {
                        ui.spinner();
                        ui.label(RichText::new(t!("status-requests", n = inflight)).weak().small());
                    }
                    if self.paused {
                        ui.label(RichText::new(t!("status-paused")).color(tokens.suspended).small());
                    } else if self.settings.pause_when_unfocused && !self.window_focused {
                        ui.label(RichText::new(t!("status-paused-unfocused")).weak().small());
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(egui::Label::new(RichText::new(&hint).weak().small()).truncate()).on_hover_text(&hint);
                    });
                });
            });
        });
    }

    fn request_screenshot(&mut self, ctx: &egui::Context) {
        let Some(plan) = &mut self.screenshot else { return };
        if plan.requested || self.frame < plan.after_frames {
            ctx.request_repaint();
            return;
        }
        plan.requested = true;
        let path = plan.path.clone();
        let exit = plan.exit;
        let ctx2 = ctx.clone();
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |img: Arc<egui::ColorImage>| {
            let [w, h] = img.size;
            let raw: Vec<u8> = img.as_raw().to_vec();
            match image::RgbaImage::from_raw(w as u32, h as u32, raw) {
                Some(im) => {
                    if let Err(e) = im.save(&path) {
                        log::error!("screenshot save failed: {e}");
                    } else {
                        log::info!("screenshot saved to {}", path.display());
                    }
                }
                None => log::error!("screenshot: bad image buffer"),
            }
            if exit {
                ctx2.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        })));
    }
}

// Fields that need interior access patterns; kept outside the main struct literal for clarity.
impl App {
    pub fn set_ctx(&mut self, ctx: &egui::Context) {
        if self.ctx.is_none() {
            self.ctx = Some(ctx.clone());
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame(root);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.persist_settings();
    }

    fn on_exit(&mut self) {
        self.report_stats();
    }
}

impl App {
    /// Write settings to the configured file, if any. Headless tests have no path and so can
    /// never touch a developer's real configuration.
    pub fn persist_settings(&self) {
        if let Some(p) = &self.settings_path {
            self.settings.save_to(p);
        }
    }

    /// Warn (with a one-click fix) when the configuration directory or any file in it is readable
    /// by other users. Run once after start-up.
    pub fn audit_settings_permissions(&mut self) {
        let Some(dir) = self.settings_path.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf())) else { return };
        let issues = crate::settings::perms::audit(&dir);
        if issues.is_empty() {
            return;
        }
        let details = issues.iter().map(|i| i.describe()).collect::<Vec<_>>().join("\n");
        let n = issues.len();
        let fix_list = issues.clone();
        let fix_label = t!("perms-fix");
        self.toasts.push_with_action(
            ToastKind::Warning,
            t!("perms-warning", n = n),
            Some(details),
            &fix_label,
            Some(Box::new(move |app: &mut App| {
                let failed = crate::settings::perms::fix(&fix_list);
                if failed.is_empty() {
                    app.toasts.push(ToastKind::Success, t!("perms-fixed"));
                } else {
                    let d = failed.iter().map(|(i, e)| format!("{}: {e}", i.path.display())).collect::<Vec<_>>().join("\n");
                    app.toasts.error(t!("perms-fix-failed"), Some(d));
                }
            })),
        );
    }

    fn report_stats(&self) {
        if self.stats && !self.frame_times.is_empty() {
            let mut t = self.frame_times.clone();
            t.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let p = |q: f32| t[((t.len() - 1) as f32 * q) as usize];
            log::info!("frames: {} · dt ms p50 {:.2} p90 {:.2} p99 {:.2} max {:.2}", t.len(), p(0.5), p(0.9), p(0.99), t[t.len() - 1]);
        }
    }

    /// One frame of the whole application. Called by eframe and by headless tests.
    pub fn frame(&mut self, root: &mut egui::Ui) {
        let ctx = &root.ctx().clone();
        self.frame += 1;
        if self.frame == 1 {
            // The context that actually paints may differ from the one given to `new` (tests).
            self.ctx = Some(ctx.clone());
            crate::theme::install_fonts(ctx);
            self.apply_theme(ctx);
            self.audit_settings_permissions();
        }
        self.set_ctx(ctx);
        crate::platform::pump();
        self.window_focused = ctx.input(|i| i.focused);
        if self.theme_mode() == ThemeMode::System && self.frame.is_multiple_of(60) && Self::wants_dark(ThemeMode::System, ctx) != self.tokens.dark {
            self.frame_theme_dirty = true;
        }
        if self.frame_theme_dirty {
            self.frame_theme_dirty = false;
            self.apply_theme(ctx);
        }
        if (self.ui_scale_applied - self.scale).abs() > 1e-3 {
            ctx.set_zoom_factor(self.scale);
            self.ui_scale_applied = self.scale;
        }
        // synthetic input from MCP (ui_key / ui_type)
        if !self.synthetic_events.is_empty() {
            let evs = std::mem::take(&mut self.synthetic_events);
            // Real keystrokes only reach a focused window, so egui ignores widget focus while the
            // window is unfocused. Synthetic input targets the window explicitly (an agent drives
            // it, often while Windows keeps it in the background), so this frame counts as focused.
            ctx.input_mut(|i| {
                i.focused = true;
                i.events.extend(evs);
            });
            ctx.request_repaint();
        }
        // drain action callbacks
        if let Some(net) = &self.net {
            let cbs = net.drain();
            for cb in cbs {
                cb(self);
            }
        }
        self.drain_mcp();
        self.apply_pending_nav();
        self.health_check();
        // global cache polling
        if self.cache.defs.poll() {
            if let Some(d) = &self.cache.defs.data {
                for x in d {
                    self.cache.known_defs.insert(x.id.clone(), x.clone());
                }
            }
        }
        if self.cache.all_defs.poll() {
            if let Some(d) = &self.cache.all_defs.data {
                for x in d {
                    self.cache.known_defs.insert(x.id.clone(), x.clone());
                }
            }
        }
        self.cache.def_stats.poll();
        for l in self.cache.diagrams.values_mut() {
            l.poll();
        }

        // shortcuts (before UI so places see consumed keys)
        if self.confirm.is_none() && !self.palette.open {
            commands::handle_shortcuts(ctx, self);
        }

        let connected_ui = self.net.is_some() && !matches!(self.conn, ConnStatus::Connecting) || !self.tabs.is_empty();
        if connected_ui {
            self.top_bar(root);
            self.status_bar(root);
            self.status_hint.clear();
            if self.tabs.is_empty() {
                self.navigate(if self.net.is_some() { Nav::Definitions } else { Nav::Settings }, NavMode::NewTab);
                self.apply_pending_nav();
            }
            // take the active tab out so the place can borrow the app mutably
            let active = self.active.min(self.tabs.len() - 1);
            self.active = active;
            let mut tab = self.tabs.remove(active);
            let req = self.place_request.take();
            if let Some(d) = self.startup_dock {
                match tab.place_mut() {
                    Place::Definition(p) => {
                        p.dock_tab = d;
                        self.startup_dock = None;
                    }
                    Place::Instance(p) => {
                        p.dock_tab = d;
                        self.startup_dock = None;
                    }
                    _ => {}
                }
            }
            match tab.place_mut() {
                Place::Definitions(p) => {
                    if req == Some(PlaceRequest::FocusFilter) {
                        p.filter_focus = true;
                    }
                    p.ui(root, self)
                }
                Place::Definition(p) => {
                    apply_diagram_request(&mut p.view, &req);
                    if req == Some(PlaceRequest::FocusFilter) {
                        p.dock_tab = 1;
                        p.filter_focus = true;
                        self.dock_visible = true;
                    }
                    p.ui(root, self)
                }
                Place::Instance(p) => p.ui(root, self),
                Place::Search(p) => p.ui(root, self),
                Place::Incidents(p) => p.ui(root, self),
                Place::Batches(p) => p.ui(root, self),
                Place::Settings(p) => p.ui(root, self),
                Place::File(p) => {
                    apply_diagram_request(&mut p.view, &req);
                    p.ui(root, self)
                }
            }
            self.tabs.insert(active, tab);
        } else {
            connect::ui(root, self);
        }

        self.apply_pending_nav();

        // overlays
        let mut palette = std::mem::take(&mut self.palette);
        palette.ui(ctx, self);
        self.palette = palette;
        commands::help_overlay(ctx, self);
        if let Some(mut c) = self.confirm.take() {
            match c.ui(ctx, &self.tokens) {
                Some(true) => {
                    if let Some(a) = c.action.take() {
                        a(self);
                    }
                }
                Some(false) => {}
                None => self.confirm = Some(c),
            }
        }
        let tokens = self.tokens.clone();
        for a in self.toasts.ui(ctx, &tokens) {
            match a {
                crate::widgets::toast::ToastAction::Undo(f) => f(self),
            }
        }
        if self.settings_dirty {
            self.settings_dirty = false;
            self.persist_settings();
        }
        self.request_screenshot(ctx);
        if self.stats {
            let dt = ctx.input(|i| i.unstable_dt);
            if self.frame > 5 {
                self.frame_times.push(dt * 1000.0);
            }
        }
    }
}

fn apply_diagram_request(view: &mut crate::viewport::DiagramView, req: &Option<PlaceRequest>) {
    match req {
        Some(PlaceRequest::OpenFind) => view.open_find(),
        Some(PlaceRequest::FitDiagram) => view.fit_all(),
        Some(PlaceRequest::Zoom(f)) => {
            view.camera.target.zoom = (view.camera.target.zoom * f).clamp(crate::viewport::camera::MIN_ZOOM, crate::viewport::camera::MAX_ZOOM);
        }
        _ => {}
    }
}

/// Context menu items shared by every diagram (element-level, engine-independent).
pub fn diagram_context_menu(ui: &mut egui::Ui, model: &Model, target: Option<bpmn::Ref>, actions: &mut Vec<crate::inspector::InspectorAction>) {
    use crate::inspector::InspectorAction as A;
    match target {
        Some(bpmn::Ref::Element(i)) => {
            let e = &model.elements[i];
            ui.label(RichText::new(e.display_name()).strong());
            ui.label(RichText::new(&e.id).monospace().small().weak());
            ui.separator();
            if ui.button(format!("{:<22} Enter", t!("ctx-inspect"))).clicked() {
                actions.push(A::Select(bpmn::Ref::Element(i)));
                ui.close();
            }
            if ui.button(format!("{:<22} Ctrl+C", t!("ctx-copy-id"))).clicked() {
                ui.ctx().copy_text(e.id.clone());
                ui.close();
            }
            if let Some(n) = &e.name {
                if ui.button(t!("ctx-copy-name")).clicked() {
                    ui.ctx().copy_text(n.clone());
                    ui.close();
                }
            }
            if e.kind.is_flow_node() {
                ui.separator();
                if ui.button(t!("ctx-instances-waiting")).clicked() {
                    actions.push(A::InstancesAt(e.id.clone()));
                    ui.close();
                }
                if ui.button(t!("ctx-instances-executed")).clicked() {
                    actions.push(A::ExecutedAt(e.id.clone()));
                    ui.close();
                }
            }
            if e.kind == bpmn::ElementKind::CallActivity {
                if let Some(k) = e.attr("calledElement") {
                    if ui.button(t!("ctx-open-called", key = k)).clicked() {
                        actions.push(A::OpenDefinitionKey(k.to_string()));
                        ui.close();
                    }
                }
            }
        }
        Some(bpmn::Ref::Flow(i)) => {
            let f = &model.flows[i];
            ui.label(RichText::new(f.display_name()).strong());
            if ui.button(t!("ctx-inspect")).clicked() {
                actions.push(A::Select(bpmn::Ref::Flow(i)));
                ui.close();
            }
            if ui.button(t!("ctx-copy-id")).clicked() {
                ui.ctx().copy_text(f.id.clone());
                ui.close();
            }
            if let Some(s) = f.source {
                if ui.button(t!("ctx-go-source", name = crate::util::ellipsize(model.elements[s].display_name(), 30))).clicked() {
                    actions.push(A::Select(bpmn::Ref::Element(s)));
                    ui.close();
                }
            }
            if let Some(t) = f.target {
                if ui.button(t!("ctx-go-target", name = crate::util::ellipsize(model.elements[t].display_name(), 30))).clicked() {
                    actions.push(A::Select(bpmn::Ref::Element(t)));
                    ui.close();
                }
            }
        }
        None => {
            ui.label(RichText::new(t!("ctx-diagram")).weak());
            if ui.button(format!("{:<22} 0", t!("ctx-fit-diagram"))).clicked() {
                actions.push(A::Select(bpmn::Ref::Element(usize::MAX)));
                ui.close();
            }
        }
    }
}

impl App {
    /// Parsed diagram for a definition id; starts the download on first use.
    pub fn diagram(&mut self, id: &str) -> Option<Arc<Model>> {
        let l = self.cache.diagrams.entry(id.to_string()).or_default();
        if let Some(m) = &l.data {
            return Some(m.clone());
        }
        if l.is_empty() || (l.error.is_some() && l.stale(Duration::from_secs(10))) {
            if let Some(net) = &self.net {
                let id2 = id.to_string();
                net.fetch(l, move |c| async move {
                    let xml = c.process_definition_xml(&id2).await?;
                    let parsed = tokio::task::spawn_blocking(move || bpmn::parse(&xml).map(Arc::new).map_err(|e| camunda::ApiError::Decode(format!("BPMN parse error: {e}"))))
                        .await
                        .map_err(|e| camunda::ApiError::Decode(e.to_string()))?;
                    parsed
                });
            }
        }
        None
    }

    /// Keep the definition list and per-definition statistics fresh.
    pub fn ensure_definitions(&mut self) {
        let Some(net) = &self.net else { return };
        let max_age = Duration::from_secs(self.settings.refresh_list_secs);
        let auto = self.auto_refresh_allowed();
        if self.cache.defs.stale(max_age) && (self.cache.defs.age().is_none() || auto) {
            net.fetch(&mut self.cache.defs, |c| async move { c.process_definitions(true).await });
        }
        if self.cache.all_defs.stale(max_age) && (self.cache.all_defs.age().is_none() || auto) {
            net.fetch(&mut self.cache.all_defs, |c| async move { c.process_definitions(false).await });
        }
        if self.cache.def_stats.stale(max_age) && (self.cache.def_stats.age().is_none() || auto) {
            net.fetch(&mut self.cache.def_stats, |c| async move { c.process_definition_statistics().await });
        }
    }
}
