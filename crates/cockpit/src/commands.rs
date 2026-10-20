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
    /// Message id of the human name (see `name()`).
    pub name_key: &'static str,
    pub shortcut: Option<KeyboardShortcut>,
    pub keys_label: &'static str,
    /// Message id of the category (see `category()`).
    pub category_key: &'static str,
}

impl CmdInfo {
    pub fn name(&self) -> String {
        crate::t!(self.name_key)
    }
    pub fn category(&self) -> String {
        crate::t!(self.category_key)
    }
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
            Cmd::Palette => CmdInfo { name_key: "cmd-palette", shortcut: sc(Modifiers::COMMAND, Key::K), keys_label: "Ctrl+K", category_key: "cat-general" },
            Cmd::SearchInstances => CmdInfo { name_key: "cmd-search-instances", shortcut: sc(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::F), keys_label: "Ctrl+Shift+F", category_key: "cat-navigate" },
            Cmd::FindInDiagram => CmdInfo { name_key: "cmd-find-in-diagram", shortcut: sc(Modifiers::COMMAND, Key::F), keys_label: "Ctrl+F", category_key: "cat-diagram" },
            Cmd::GoDefinitions => CmdInfo { name_key: "cmd-go-definitions", shortcut: sc(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::D), keys_label: "Ctrl+Shift+D", category_key: "cat-navigate" },
            Cmd::GoIncidents => CmdInfo { name_key: "cmd-go-incidents", shortcut: sc(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::I), keys_label: "Ctrl+Shift+I", category_key: "cat-navigate" },
            Cmd::GoBatches => CmdInfo { name_key: "cmd-go-batches", shortcut: sc(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::B), keys_label: "Ctrl+Shift+B", category_key: "cat-navigate" },
            Cmd::GoSettings => CmdInfo { name_key: "cmd-open-settings", shortcut: sc(Modifiers::COMMAND, Key::Comma), keys_label: "Ctrl+,", category_key: "cat-general" },
            Cmd::NewTab => CmdInfo { name_key: "cmd-new-tab", shortcut: sc(Modifiers::COMMAND, Key::T), keys_label: "Ctrl+T", category_key: "cat-tabs" },
            Cmd::CloseTab => CmdInfo { name_key: "cmd-close-tab", shortcut: sc(Modifiers::COMMAND, Key::W), keys_label: "Ctrl+W", category_key: "cat-tabs" },
            Cmd::NextTab => CmdInfo { name_key: "cmd-next-tab", shortcut: sc(Modifiers::CTRL, Key::Tab), keys_label: "Ctrl+Tab", category_key: "cat-tabs" },
            Cmd::PrevTab => CmdInfo { name_key: "cmd-prev-tab", shortcut: sc(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::Tab), keys_label: "Ctrl+Shift+Tab", category_key: "cat-tabs" },
            Cmd::Back => CmdInfo { name_key: "cmd-back", shortcut: sc(Modifiers::ALT, Key::ArrowLeft), keys_label: "Alt+←", category_key: "cat-navigate" },
            Cmd::Forward => CmdInfo { name_key: "cmd-forward", shortcut: sc(Modifiers::ALT, Key::ArrowRight), keys_label: "Alt+→", category_key: "cat-navigate" },
            Cmd::Refresh => CmdInfo { name_key: "cmd-refresh", shortcut: sc(Modifiers::NONE, Key::F5), keys_label: "F5 / R", category_key: "cat-general" },
            Cmd::TogglePause => CmdInfo { name_key: "cmd-toggle-pause", shortcut: sc(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::P), keys_label: "Ctrl+Shift+P", category_key: "cat-general" },
            Cmd::ToggleDock => CmdInfo { name_key: "cmd-toggle-dock", shortcut: sc(Modifiers::COMMAND, Key::B), keys_label: "Ctrl+B", category_key: "cat-view" },
            Cmd::ToggleTheme => CmdInfo { name_key: "cmd-toggle-theme", shortcut: sc(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::T), keys_label: "Ctrl+Shift+T", category_key: "cat-view" },
            Cmd::FitDiagram => CmdInfo { name_key: "cmd-fit-diagram", shortcut: None, keys_label: "0", category_key: "cat-diagram" },
            Cmd::ZoomIn => CmdInfo { name_key: "cmd-zoom-in", shortcut: None, keys_label: "+", category_key: "cat-diagram" },
            Cmd::ZoomOut => CmdInfo { name_key: "cmd-zoom-out", shortcut: None, keys_label: "−", category_key: "cat-diagram" },
            Cmd::CopyCockpitLink => CmdInfo { name_key: "cmd-copy-cockpit-link", shortcut: sc(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::C), keys_label: "Ctrl+Shift+C", category_key: "cat-general" },
            Cmd::CopyPlaceId => CmdInfo { name_key: "cmd-copy-place-id", shortcut: None, keys_label: "", category_key: "cat-general" },
            Cmd::FocusFilter => CmdInfo { name_key: "cmd-focus-filter", shortcut: None, keys_label: "/", category_key: "cat-general" },
            Cmd::Help => CmdInfo { name_key: "cmd-help", shortcut: sc(Modifiers::NONE, Key::F1), keys_label: "F1 / ?", category_key: "cat-general" },
            Cmd::Disconnect => CmdInfo { name_key: "cmd-disconnect", shortcut: None, keys_label: "", category_key: "cat-general" },
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
            Cmd::GoSettings => app.open_settings(),
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
                let m = if app.paused { crate::t!("toast-refresh-paused") } else { crate::t!("toast-refresh-resumed") };
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
                let hay = format!("{} {}", info.name(), info.category()).to_lowercase();
                if let Some(score) = fuzzy(&hay, &ql) {
                    out.push((Item::Cmd(c), info.name(), info.keys_label.to_string(), score));
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
                out.push((Item::Nav(Nav::Instance { id: q.to_string() }), crate::t!("palette-open-instance", id = crate::util::short_uuid(q)), crate::t!("palette-kind-instance-id"), 1000));
                out.push((Item::Nav(Nav::Definition { id: q.to_string() }), crate::t!("palette-open-as-definition"), String::new(), 990));
            } else if q.contains(':') && q.len() > 40 && !q.contains(' ') {
                out.push((Item::Nav(Nav::Definition { id: q.to_string() }), crate::t!("palette-open-definition", id = crate::util::ellipsize(q, 40)), crate::t!("palette-kind-definition-id"), 1000));
            }
            if !q.is_empty() {
                let label = if mode == '#' { crate::t!("palette-search-business-key", q = q) } else { crate::t!("palette-search-instances", q = q) };
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
                let edit = egui::TextEdit::singleline(&mut self.query).id(egui::Id::unique("palette-input")).hint_text(crate::t!("palette-hint")).desired_width(f32::INFINITY).font(egui::TextStyle::Heading).frame(egui::Frame::NONE);
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
                        ui.label(egui::RichText::new(crate::t!("common-no-matches")).weak());
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
    // Two content-sized tables side by side; the window takes their width instead of forcing
    // one, and scrolls when the screen is shorter than the list.
    let max_h = ctx.content_rect().height() - 80.0;
    egui::Window::new(crate::t!("help-title")).collapsible(false).resizable(false).open(&mut open).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).max_height(max_h).show(ctx, |ui| {
        egui::ScrollArea::vertical().max_height(max_h - 40.0).show(ui, |ui| {
            // Fixed width per table: side-by-side content inside a self-sizing window would
            // otherwise give the second table only the leftover width.
            const TABLE_W: f32 = 540.0;
            let table = |ui: &mut egui::Ui, id: &str, title: &str, rows: &[(String, String)]| {
                ui.vertical(|ui| {
                    ui.set_min_width(TABLE_W);
                    ui.set_max_width(TABLE_W);
                    ui.label(egui::RichText::new(title).strong());
                    // Keys never wrap; descriptions wrap inside a capped column, so neither
                    // column can run into the other whatever the window width.
                    egui::Grid::new(id).num_columns(2).spacing([16.0, 3.0]).striped(true).max_col_width(340.0).show(ui, |ui| {
                        for (k, v) in rows {
                            ui.add(egui::Label::new(egui::RichText::new(k).monospace().size(12.0)).extend());
                            ui.add(egui::Label::new(egui::RichText::new(v).size(12.0)).wrap());
                            ui.end_row();
                        }
                    });
                });
            };
            let diagram: Vec<(String, String)> = [
                ("Wheel / pinch", crate::t!("help-d-wheel")),
                ("Alt (hold)", crate::t!("help-d-alt")),
                ("Shift+wheel", crate::t!("help-d-shift-wheel")),
                ("Drag, middle-drag, Space+drag", crate::t!("help-d-drag")),
                ("↑↓←→  WASD  HJKL", crate::t!("help-d-arrows")),
                ("+ / −", crate::t!("help-d-plus-minus")),
                ("0 · 1 · F · Shift+F", crate::t!("help-d-fit")),
                ("Double-click", crate::t!("help-d-double-click")),
                ("Tab / Shift+Tab", crate::t!("help-d-tab")),
                ("Alt+→ / Alt+←", crate::t!("help-d-alt-arrows")),
                ("Enter", crate::t!("help-d-enter")),
                ("Ctrl+F", crate::t!("help-d-find")),
                ("Ctrl+C", crate::t!("help-d-copy")),
                ("M · G · I", crate::t!("help-d-toggles")),
                ("Right-click", crate::t!("help-d-right-click")),
                ("Esc", crate::t!("help-d-esc")),
            ]
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
            let mut everywhere: Vec<(String, String)> = Cmd::ALL.iter().map(|c| c.info()).filter(|i| !i.keys_label.is_empty()).map(|i| (i.keys_label.to_string(), i.name())).collect();
            everywhere.extend([("Ctrl+1…9", crate::t!("help-g-dock-tab")), ("↑↓ · Enter · Shift+Enter", crate::t!("help-g-tables")), ("Middle-click", crate::t!("help-g-middle-click")), ("/", crate::t!("help-g-slash")), ("Alt (hold)", crate::t!("help-g-alt"))].iter().map(|(k, v)| (k.to_string(), v.clone())));
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 28.0;
                table(ui, "help-diagram", &crate::t!("help-diagram"), &diagram);
                table(ui, "help-global", &crate::t!("help-everywhere"), &everywhere);
            });
        });
        ui.add_space(6.0);
        ui.label(egui::RichText::new(crate::t!("help-close-hint")).weak().small());
    });
    if !open || ctx.input(|i| i.key_pressed(Key::Escape)) {
        app.help_open = false;
    }
}
