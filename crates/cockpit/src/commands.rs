//! Global commands: one registry feeding shortcuts, the palette, menus and the help overlay.

use crate::app::App;
use crate::state::{Nav, NavMode};
use egui::{Key, KeyboardShortcut, Modifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cmd {
    Palette,
    SearchInstances,
    GoDefinitions,
    GoIncidents,
    GoBatches,
    GoSettings,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    Back,
    Forward,
    Refresh,
    TogglePause,
    ToggleDock,
    ToggleTheme,
    Help,
    FindInDiagram,
    FitDiagram,
    CopyCockpitLink,
    CopyPlaceId,
    Disconnect,
    ZoomIn,
    ZoomOut,
    FocusFilter,
}

pub struct CmdInfo {
    pub name: &'static str,
    pub shortcut: Option<KeyboardShortcut>,
    pub keys_label: &'static str,
    pub category: &'static str,
}

const fn sc(m: Modifiers, k: Key) -> Option<KeyboardShortcut> {
    Some(KeyboardShortcut::new(m, k))
}

impl Cmd {
    pub const ALL: [Cmd; 25] = [
        Cmd::Palette,
        Cmd::SearchInstances,
        Cmd::FindInDiagram,
        Cmd::GoDefinitions,
        Cmd::GoIncidents,
        Cmd::GoBatches,
        Cmd::GoSettings,
        Cmd::NewTab,
        Cmd::CloseTab,
        Cmd::NextTab,
        Cmd::PrevTab,
        Cmd::Back,
        Cmd::Forward,
        Cmd::Refresh,
        Cmd::TogglePause,
        Cmd::ToggleDock,
        Cmd::ToggleTheme,
        Cmd::FitDiagram,
        Cmd::ZoomIn,
        Cmd::ZoomOut,
        Cmd::CopyCockpitLink,
        Cmd::CopyPlaceId,
        Cmd::FocusFilter,
        Cmd::Help,
        Cmd::Disconnect,
    ];

    pub fn info(self) -> CmdInfo {
        match self {
            Cmd::Palette => CmdInfo { name: "Command palette", shortcut: sc(Modifiers::COMMAND, Key::K), keys_label: "Ctrl+K", category: "General" },
            Cmd::SearchInstances => CmdInfo { name: "Search process instances", shortcut: sc(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::F), keys_label: "Ctrl+Shift+F", category: "Navigate" },
            Cmd::FindInDiagram => CmdInfo { name: "Find element in diagram", shortcut: sc(Modifiers::COMMAND, Key::F), keys_label: "Ctrl+F", category: "Diagram" },
            Cmd::GoDefinitions => CmdInfo { name: "Go to process definitions", shortcut: sc(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::D), keys_label: "Ctrl+Shift+D", category: "Navigate" },
            Cmd::GoIncidents => CmdInfo { name: "Go to incidents", shortcut: sc(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::I), keys_label: "Ctrl+Shift+I", category: "Navigate" },
            Cmd::GoBatches => CmdInfo { name: "Go to batches", shortcut: sc(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::B), keys_label: "Ctrl+Shift+B", category: "Navigate" },
            Cmd::GoSettings => CmdInfo { name: "Open settings", shortcut: sc(Modifiers::COMMAND, Key::Comma), keys_label: "Ctrl+,", category: "General" },
            Cmd::NewTab => CmdInfo { name: "New tab", shortcut: sc(Modifiers::COMMAND, Key::T), keys_label: "Ctrl+T", category: "Tabs" },
            Cmd::CloseTab => CmdInfo { name: "Close tab", shortcut: sc(Modifiers::COMMAND, Key::W), keys_label: "Ctrl+W", category: "Tabs" },
            Cmd::NextTab => CmdInfo { name: "Next tab", shortcut: sc(Modifiers::CTRL, Key::Tab), keys_label: "Ctrl+Tab", category: "Tabs" },
            Cmd::PrevTab => CmdInfo { name: "Previous tab", shortcut: sc(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::Tab), keys_label: "Ctrl+Shift+Tab", category: "Tabs" },
            Cmd::Back => CmdInfo { name: "Back", shortcut: sc(Modifiers::ALT, Key::ArrowLeft), keys_label: "Alt+←", category: "Navigate" },
            Cmd::Forward => CmdInfo { name: "Forward", shortcut: sc(Modifiers::ALT, Key::ArrowRight), keys_label: "Alt+→", category: "Navigate" },
            Cmd::Refresh => CmdInfo { name: "Refresh now", shortcut: sc(Modifiers::NONE, Key::F5), keys_label: "F5 / R", category: "General" },
            Cmd::TogglePause => CmdInfo { name: "Pause / resume auto-refresh", shortcut: sc(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::P), keys_label: "Ctrl+Shift+P", category: "General" },
            Cmd::ToggleDock => CmdInfo { name: "Toggle side dock", shortcut: sc(Modifiers::COMMAND, Key::B), keys_label: "Ctrl+B", category: "View" },
            Cmd::ToggleTheme => CmdInfo { name: "Cycle theme: system → light → dark", shortcut: sc(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::T), keys_label: "Ctrl+Shift+T", category: "View" },
            Cmd::FitDiagram => CmdInfo { name: "Fit diagram", shortcut: None, keys_label: "0", category: "Diagram" },
            Cmd::ZoomIn => CmdInfo { name: "Zoom in", shortcut: None, keys_label: "+", category: "Diagram" },
            Cmd::ZoomOut => CmdInfo { name: "Zoom out", shortcut: None, keys_label: "−", category: "Diagram" },
            Cmd::CopyCockpitLink => CmdInfo { name: "Copy link for Camunda Cockpit", shortcut: sc(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::C), keys_label: "Ctrl+Shift+C", category: "General" },
            Cmd::CopyPlaceId => CmdInfo { name: "Copy current id (definition / instance)", shortcut: None, keys_label: "", category: "General" },
            Cmd::FocusFilter => CmdInfo { name: "Focus filter / search field", shortcut: None, keys_label: "/", category: "General" },
            Cmd::Help => CmdInfo { name: "Keyboard shortcuts", shortcut: sc(Modifiers::NONE, Key::F1), keys_label: "F1 / ?", category: "General" },
            Cmd::Disconnect => CmdInfo { name: "Disconnect from engine", shortcut: None, keys_label: "", category: "General" },
        }
    }

    pub fn enabled(self, app: &App) -> bool {
        let tab = app.tabs.get(app.active);
        match self {
            Cmd::CloseTab => app.tabs.len() > 1,
            Cmd::Back => tab.map(|t| t.can_back()).unwrap_or(false),
            Cmd::Forward => tab.map(|t| t.can_forward()).unwrap_or(false),
            Cmd::NextTab | Cmd::PrevTab => app.tabs.len() > 1,
            Cmd::FindInDiagram | Cmd::FitDiagram | Cmd::ZoomIn | Cmd::ZoomOut => tab.map(|t| t.place().has_diagram()).unwrap_or(false),
            Cmd::CopyCockpitLink | Cmd::CopyPlaceId => tab.map(|t| matches!(t.place(), crate::state::Place::Definition(_) | crate::state::Place::Instance(_))).unwrap_or(false),
            Cmd::Disconnect | Cmd::SearchInstances | Cmd::GoDefinitions | Cmd::GoIncidents | Cmd::GoBatches => app.net.is_some(),
            _ => true,
        }
    }

    pub fn run(self, app: &mut App) {
        match self {
            Cmd::Palette => app.palette.open(),
            Cmd::SearchInstances => app.navigate(Nav::Search { query: String::new() }, NavMode::Here),
            Cmd::FindInDiagram => app.place_request = Some(PlaceRequest::OpenFind),
            Cmd::GoDefinitions => app.navigate(Nav::Definitions, NavMode::Here),
            Cmd::GoIncidents => app.navigate(Nav::Incidents, NavMode::Here),
            Cmd::GoBatches => app.navigate(Nav::Batches, NavMode::Here),
            Cmd::GoSettings => app.navigate(Nav::Settings, NavMode::Here),
            Cmd::NewTab => app.navigate(if app.net.is_some() { Nav::Definitions } else { Nav::Settings }, NavMode::NewTab),
            Cmd::CloseTab => app.close_tab(app.active),
            Cmd::NextTab => {
                if !app.tabs.is_empty() {
                    app.active = (app.active + 1) % app.tabs.len();
                }
            }
            Cmd::PrevTab => {
                if !app.tabs.is_empty() {
                    app.active = (app.active + app.tabs.len() - 1) % app.tabs.len();
                }
            }
            Cmd::Back => {
                if let Some(t) = app.tabs.get_mut(app.active) {
                    t.back();
                }
            }
            Cmd::Forward => {
                if let Some(t) = app.tabs.get_mut(app.active) {
                    t.forward();
                }
            }
            Cmd::Refresh => app.bump_refresh(),
            Cmd::TogglePause => {
                app.paused = !app.paused;
                let m = if app.paused { "Auto-refresh paused" } else { "Auto-refresh resumed" };
                app.toasts.push(crate::widgets::ToastKind::Info, m);
            }
            Cmd::ToggleDock => app.dock_visible = !app.dock_visible,
            Cmd::ToggleTheme => app.toggle_theme(),
            Cmd::FitDiagram => app.place_request = Some(PlaceRequest::FitDiagram),
            Cmd::ZoomIn => app.place_request = Some(PlaceRequest::Zoom(1.25)),
            Cmd::ZoomOut => app.place_request = Some(PlaceRequest::Zoom(0.8)),
            Cmd::CopyCockpitLink => app.copy_cockpit_link(),
            Cmd::CopyPlaceId => app.copy_place_id(),
            Cmd::FocusFilter => app.place_request = Some(PlaceRequest::FocusFilter),
            Cmd::Help => app.help_open = !app.help_open,
            Cmd::Disconnect => app.disconnect(),
        }
    }
}

/// A request the app forwards to the active place on its next frame.
#[derive(Clone, Debug, PartialEq)]
pub enum PlaceRequest {
    OpenFind,
    FitDiagram,
    Zoom(f32),
    FocusFilter,
}

/// Consume global shortcuts. Single-letter ones only when no text field is focused.
pub fn handle_shortcuts(ctx: &egui::Context, app: &mut App) {
    let text_focus = ctx.memory(|m| m.focused()).map(|id| egui::text_edit::TextEditState::load(ctx, id).is_some()).unwrap_or(false);
    let mut run: Vec<Cmd> = vec![];
    ctx.input_mut(|i| {
        for c in Cmd::ALL {
            if let Some(s) = c.info().shortcut {
                // Plain F-keys and modifier combos are safe even inside text fields.
                if i.consume_shortcut(&s) {
                    run.push(c);
                }
            }
        }
        if !text_focus {
            if i.consume_key(Modifiers::NONE, Key::R) {
                run.push(Cmd::Refresh);
            }
            if i.consume_key(Modifiers::NONE, Key::Slash) {
                run.push(Cmd::FocusFilter);
            }
            // '?' arrives as text
            if i.events.iter().any(|e| matches!(e, egui::Event::Text(t) if t == "?")) {
                run.push(Cmd::Help);
            }
        }
    });
    // mouse back/forward buttons
    ctx.input(|i| {
        if i.pointer.button_pressed(egui::PointerButton::Extra1) {
            run.push(Cmd::Back);
        }
        if i.pointer.button_pressed(egui::PointerButton::Extra2) {
            run.push(Cmd::Forward);
        }
    });
    for c in run {
        if c.enabled(app) {
            c.run(app);
        }
    }
}

// ---------------- palette ----------------

#[derive(Default)]
pub struct Palette {
    pub open: bool,
    pub query: String,
    pub cursor: usize,
    want_focus: bool,
}

enum Item {
    Cmd(Cmd),
    Nav(Nav),
    Search(String),
}

impl Palette {
    pub fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.cursor = 0;
        self.want_focus = true;
    }

    fn items(&self, app: &App) -> Vec<(Item, String, String, i32)> {
        let q = self.query.trim();
        let (mode, q) = match q.chars().next() {
            Some('>') => ('>', q[1..].trim()),
            Some('#') => ('#', q[1..].trim()),
            _ => (' ', q),
        };
        let ql = q.to_lowercase();
        let mut out: Vec<(Item, String, String, i32)> = vec![];
        if mode != '#' {
            for c in Cmd::ALL {
                if !c.enabled(app) {
                    continue;
                }
                let info = c.info();
                let hay = format!("{} {}", info.name, info.category).to_lowercase();
                if let Some(score) = fuzzy(&hay, &ql) {
                    out.push((Item::Cmd(c), info.name.to_string(), info.keys_label.to_string(), score));
                }
            }
        }
        if mode != '>' && app.net.is_some() {
            if let Some(defs) = &app.cache.defs.data {
                for d in defs {
                    let hay = format!("{} {}", d.key, d.name.clone().unwrap_or_default()).to_lowercase();
                    if let Some(score) = fuzzy(&hay, &ql) {
                        out.push((Item::Nav(Nav::Definition { id: d.id.clone() }), format!("{}  ·  v{}", d.display_name(), d.version), d.key.clone(), score - 5));
                    }
                }
            }
            if crate::util::is_uuid(q) {
                out.push((Item::Nav(Nav::Instance { id: q.to_string() }), format!("Open instance {}", crate::util::short_uuid(q)), "process instance id".into(), 1000));
                out.push((Item::Nav(Nav::Definition { id: q.to_string() }), "Open as definition id".into(), String::new(), 990));
            } else if q.contains(':') && q.len() > 40 && !q.contains(' ') {
                out.push((Item::Nav(Nav::Definition { id: q.to_string() }), format!("Open definition {}", crate::util::ellipsize(q, 40)), "process definition id".into(), 1000));
            }
            if !q.is_empty() {
                let label = if mode == '#' { format!("Search instances for business key “{q}”") } else { format!("Search instances: {q}") };
                let query = if mode == '#' { format!("bk:{q}") } else { q.to_string() };
                out.push((Item::Search(query), label, "Enter".into(), if mode == '#' { 2000 } else { -50 }));
            }
        }
        out.sort_by(|a, b| b.3.cmp(&a.3).then_with(|| a.1.cmp(&b.1)));
        out.truncate(14);
        out
    }

    pub fn ui(&mut self, ctx: &egui::Context, app: &mut App) {
        if !self.open {
            return;
        }
        let items = self.items(app);
        self.cursor = self.cursor.min(items.len().saturating_sub(1));
        let mut chosen: Option<usize> = None;
        let mut close = false;
        let screen = ctx.content_rect();
        let w = 620.0_f32.min(screen.width() - 40.0);
        egui::Area::new(egui::Id::unique("palette")).order(egui::Order::Foreground).fixed_pos(egui::Pos2::new(screen.center().x - w / 2.0, screen.min.y + 80.0)).show(ctx, |ui| {
            egui::Frame::popup(ui.style()).inner_margin(10.0).show(ui, |ui| {
                ui.set_width(w);
                let edit = egui::TextEdit::singleline(&mut self.query).id(egui::Id::unique("palette-input")).hint_text("Type a command, a definition key or name, an instance id or business key…   (> commands, # business key)").desired_width(f32::INFINITY).font(egui::TextStyle::Heading).frame(egui::Frame::NONE);
                let r = ui.add(edit);
                if self.want_focus {
                    r.request_focus();
                    self.want_focus = false;
                }
                ui.separator();
                ui.input_mut(|i| {
                    if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                        self.cursor = (self.cursor + 1).min(items.len().saturating_sub(1));
                    }
                    if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                        self.cursor = self.cursor.saturating_sub(1);
                    }
                    if i.consume_key(Modifiers::NONE, Key::Enter) {
                        chosen = Some(self.cursor);
                    }
                    if i.consume_key(Modifiers::NONE, Key::Escape) {
                        close = true;
                    }
                });
                if r.lost_focus() && !r.has_focus() && !ctx.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Escape)) && !ui.ui_contains_pointer() {
                    close = true;
                }
                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    for (i, (_, label, hint, _)) in items.iter().enumerate() {
                        let selected = i == self.cursor;
                        let resp = ui.horizontal(|ui| {
                            let fill = if selected { app.tokens.accent.gamma_multiply(0.2) } else { egui::Color32::TRANSPARENT };
                            egui::Frame::new().fill(fill).corner_radius(4.0).inner_margin(egui::Margin::symmetric(8, 5)).show(ui, |ui| {
                                ui.set_width(w - 20.0);
                                ui.horizontal(|ui| {
                                    ui.label(label);
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(hint).weak().monospace().small());
                                    });
                                });
                            })
                        });
                        let rr = ui.interact(resp.response.rect, egui::Id::unique(("palette-item", i)), egui::Sense::click());
                        if rr.hovered() {
                            self.cursor = i;
                        }
                        if rr.clicked() {
                            chosen = Some(i);
                        }
                        if selected {
                            rr.scroll_to_me(None);
                        }
                    }
                    if items.is_empty() {
                        ui.label(egui::RichText::new("No matches").weak());
                    }
                });
            });
        });
        if let Some(i) = chosen {
            if let Some((item, ..)) = items.into_iter().nth(i) {
                self.open = false;
                match item {
                    Item::Cmd(c) => c.run(app),
                    Item::Nav(nav) => app.navigate(nav, NavMode::Here),
                    Item::Search(q) => app.navigate(Nav::Search { query: q }, NavMode::Here),
                }
            }
        }
        if close {
            self.open = false;
        }
    }
}

/// Subsequence fuzzy match: higher is better; None when not all query chars appear in order.
pub fn fuzzy(hay: &str, q: &str) -> Option<i32> {
    if q.is_empty() {
        return Some(0);
    }
    if let Some(p) = hay.find(q) {
        return Some(100 - p as i32);
    }
    let mut score = 50;
    let mut it = hay.chars();
    let mut last = 0usize;
    for (i, qc) in q.chars().enumerate() {
        let mut found = false;
        for (j, hc) in it.by_ref().enumerate() {
            if hc == qc {
                found = true;
                if i > 0 && j == 0 {
                    score += 3;
                }
                last += j + 1;
                break;
            }
        }
        if !found {
            return None;
        }
    }
    Some(score - last as i32 / 4)
}

pub fn help_overlay(ctx: &egui::Context, app: &mut App) {
    if !app.help_open {
        return;
    }
    let mut open = true;
    egui::Window::new("Keyboard & mouse").collapsible(false).resizable(false).open(&mut open).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
        ui.set_width(720.0);
        ui.columns(2, |cols| {
            let left = &mut cols[0];
            left.label(egui::RichText::new("Diagram").strong());
            egui::Grid::new("help-diagram").num_columns(2).spacing([16.0, 3.0]).show(left, |ui| {
                for (k, v) in [
                    ("Wheel / pinch", "zoom at cursor"),
                    ("Alt (hold)", "everything twice as fast: pan, zoom, dial"),
                    ("Shift+wheel", "pan horizontally"),
                    ("Drag, middle-drag, Space+drag", "pan (flick for inertia)"),
                    ("↑↓←→  WASD  HJKL", "pan (hold to accelerate, Shift faster)"),
                    ("+ / −", "zoom in / out"),
                    ("0 · 1 · F · Shift+F", "fit all · 100% · fit selection · fit tokens"),
                    ("Double-click", "zoom to element (empty: fit all)"),
                    ("Tab / Shift+Tab", "next / previous element along the flow"),
                    ("Alt+→ / Alt+←", "follow outgoing / incoming flow (1-9 picks)"),
                    ("Enter", "inspect · drill into call activity"),
                    ("Ctrl+F", "find element (Enter/Shift+Enter cycle)"),
                    ("Ctrl+C", "copy element id"),
                    ("M · G · I", "minimap · grid · element ids"),
                    ("Right-click", "context menu"),
                    ("Esc", "close find → clear selection"),
                ] {
                    ui.label(egui::RichText::new(k).monospace().size(12.0));
                    ui.label(egui::RichText::new(v).size(12.0));
                    ui.end_row();
                }
            });
            let right = &mut cols[1];
            right.label(egui::RichText::new("Everywhere").strong());
            egui::Grid::new("help-global").num_columns(2).spacing([16.0, 3.0]).show(right, |ui| {
                for c in Cmd::ALL {
                    let info = c.info();
                    if info.keys_label.is_empty() {
                        continue;
                    }
                    ui.label(egui::RichText::new(info.keys_label).monospace().size(12.0));
                    ui.label(egui::RichText::new(info.name).size(12.0));
                    ui.end_row();
                }
                for (k, v) in [("Ctrl+1…9", "dock tab"), ("↑↓ · Enter · Shift+Enter", "tables: move · open · open in new tab"), ("Middle-click", "open in new tab"), ("/", "focus the filter box"), ("Alt (hold)", "inspect element under the cursor")] {
                    ui.label(egui::RichText::new(k).monospace().size(12.0));
                    ui.label(egui::RichText::new(v).size(12.0));
                    ui.end_row();
                }
            });
        });
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Press ? or Esc to close").weak().small());
    });
    if !open || ctx.input(|i| i.key_pressed(Key::Escape)) {
        app.help_open = false;
    }
}
