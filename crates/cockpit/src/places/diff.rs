//! Visual diff of two BPMN documents (local files or engine definitions): one diagram with every
//! added, removed, changed and moved element and flow marked git-style, or the two versions
//! side by side, plus a list of all changes with their property-level differences.

use super::source::{DocSlot, DocSource, DocState};
use super::xml::XmlDoc;
use crate::app::App;
use crate::inspector::{self, InspectorAction, RuntimeFacts};
use crate::state::{Nav, NavMode};
use crate::viewport::{self, DiagramView, DiffMark, Overlays};
use bpmn::diff::{ItemDiff, LineTag, Merged, ModelDiff, PropChange, Status};
use bpmn::{Model, Ref};
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Key, Modifiers, RichText, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffMode {
    /// The new diagram with removed items drawn as ghosts.
    Unified,
    Before,
    After,
    SideBySide,
}

impl DiffMode {
    pub const ALL: [DiffMode; 4] = [DiffMode::Unified, DiffMode::Before, DiffMode::After, DiffMode::SideBySide];
    pub fn key(self) -> &'static str {
        match self {
            DiffMode::Unified => "unified",
            DiffMode::Before => "before",
            DiffMode::After => "after",
            DiffMode::SideBySide => "side-by-side",
        }
    }
    pub fn from_key(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.key() == s)
    }
    fn label(self) -> String {
        crate::t!(match self {
            DiffMode::Unified => "diff-mode-unified",
            DiffMode::Before => "diff-mode-before",
            DiffMode::After => "diff-mode-after",
            DiffMode::SideBySide => "diff-mode-side-by-side",
        })
    }
}

/// Dock tab names (also the MCP identifiers).
pub const DOCK_TABS: [&str; 2] = ["Changes", "Inspector"];

/// Indices of the three diagram views.
const OLD: usize = 0;
const NEW: usize = 1;
const MERGED: usize = 2;

pub struct DiffData {
    pub old: Arc<Model>,
    pub new: Arc<Model>,
    pub diff: ModelDiff,
    pub merged: Merged,
    /// Element/flow id (in any of the three models) → item index.
    by_id: HashMap<String, usize>,
}

impl DiffData {
    fn new(old: Arc<Model>, new: Arc<Model>) -> Self {
        let diff = bpmn::diff::diff(&old, &new);
        let merged = bpmn::diff::merge(&old, &new, &diff);
        let mut by_id = HashMap::new();
        for (i, it) in diff.items.iter().enumerate() {
            by_id.insert(it.id.clone(), i);
            if let Some(o) = &it.old_id {
                by_id.entry(o.clone()).or_insert(i);
            }
        }
        Self { old, new, diff, merged, by_id }
    }
    fn model(&self, view: usize) -> &Model {
        match view {
            OLD => &self.old,
            NEW => &self.new,
            _ => &self.merged.model,
        }
    }
    /// (old ref, new ref) of something selected in `view`.
    fn pair_of(&self, view: usize, r: Ref) -> (Option<Ref>, Option<Ref>) {
        match view {
            OLD => (Some(r), self.diff.new_of_old(r)),
            NEW => (self.diff.old_of_new(r), Some(r)),
            _ => match self.merged.ghosts.get(&r) {
                Some(&o) => (Some(o), None),
                None => (self.diff.old_of_new(r), Some(r)),
            },
        }
    }
    /// Where a pair shows up in `view`.
    fn ref_in(&self, view: usize, pair: (Option<Ref>, Option<Ref>)) -> Option<Ref> {
        match view {
            OLD => pair.0,
            NEW => pair.1,
            _ => pair.1.or_else(|| pair.0.and_then(|o| self.merged.removed.get(&o).copied())),
        }
    }
    fn item_pair(it: &ItemDiff) -> (Option<Ref>, Option<Ref>) {
        (it.old, it.new)
    }
    fn item_of(&self, view: usize, r: Ref) -> Option<usize> {
        let (o, n) = self.pair_of(view, r);
        n.and_then(|n| self.diff.for_new(n)).or_else(|| o.and_then(|o| self.diff.for_old(o))).and_then(|it| self.by_id.get(&it.id).copied())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Filter {
    pub added: bool,
    pub removed: bool,
    pub changed: bool,
    pub moved: bool,
}

impl Default for Filter {
    fn default() -> Self {
        Self { added: true, removed: true, changed: true, moved: true }
    }
}

impl Filter {
    pub fn shows(&self, s: Status) -> bool {
        match s {
            Status::Added => self.added,
            Status::Removed => self.removed,
            Status::Changed => self.changed,
            Status::Moved => self.moved,
        }
    }
}

/// Picker state for choosing the two documents.
#[derive(Default)]
struct Picker {
    filter: String,
    path: [String; 2],
    /// A file chosen in the native dialog (side, path), filled by a background thread.
    picked: Arc<Mutex<Option<(usize, PathBuf)>>>,
    dialog_open: bool,
}

pub struct DiffPlace {
    pub left: Option<DocSource>,
    pub right: Option<DocSource>,
    slots: [Option<DocSlot>; 2],
    pub data: Option<DiffData>,
    pub error: Option<String>,
    pub mode: DiffMode,
    pub views: [DiagramView; 3],
    overlays: [Overlays; 3],
    pub dock_tab: usize,
    pub filter: Filter,
    pub dim_unchanged: bool,
    /// Index into `diff.items` of the change last stepped to with n / N.
    cursor: Option<usize>,
    picker: Picker,
    marks_dirty: bool,
    /// The view the user touched last (side-by-side has two).
    active_view: usize,
    last_cameras: [Option<viewport::camera::CameraPose>; 2],
    /// Selection of each view as of the last frame, to mirror changes made by any means.
    last_selection: [Option<Ref>; 3],
    scroll_to_item: Option<usize>,
}

impl DiffPlace {
    pub fn new(left: Option<DocSource>, right: Option<DocSource>) -> Self {
        let id = egui::Id::unique(("diff", &left, &right));
        Self {
            slots: [left.clone().map(DocSlot::new), right.clone().map(DocSlot::new)],
            left,
            right,
            data: None,
            error: None,
            mode: DiffMode::Unified,
            views: [DiagramView::new(id.with("old")), DiagramView::new(id.with("new")), DiagramView::new(id.with("merged"))],
            overlays: Default::default(),
            dock_tab: 0,
            filter: Filter::default(),
            dim_unchanged: true,
            cursor: None,
            picker: Picker::default(),
            marks_dirty: true,
            active_view: MERGED,
            last_cameras: [None, None],
            last_selection: [None; 3],
            scroll_to_item: None,
        }
    }

    pub fn title(&self) -> String {
        let name = |s: &Option<DocSource>| match s {
            Some(DocSource::File(p)) => p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(),
            Some(DocSource::Definition(id)) => super::source::definition_key_version(id).map(|(k, v)| format!("{k} v{v}")).unwrap_or_else(|| crate::util::short_uuid(id)),
            None => "…".into(),
        };
        format!("{} ⇄ {}", name(&self.left), name(&self.right))
    }

    pub fn same_sources(&self, left: &Option<DocSource>, right: &Option<DocSource>) -> bool {
        left.is_some() && right.is_some() && self.left == *left && self.right == *right
    }

    /// The view drawn in the main area (the left one in side-by-side).
    pub fn primary_view(&self) -> usize {
        match self.mode {
            DiffMode::Unified => MERGED,
            DiffMode::Before => OLD,
            DiffMode::After => NEW,
            DiffMode::SideBySide => {
                if self.active_view == NEW {
                    NEW
                } else {
                    OLD
                }
            }
        }
    }
    /// Run `f` on the primary view and its model (MCP selection, fit, find).
    pub fn with_view<R>(&mut self, f: impl FnOnce(&mut DiagramView, &Model) -> R) -> Option<R> {
        let v = self.primary_view();
        let d = self.data.as_ref()?;
        Some(f(&mut self.views[v], d.model(v)))
    }
    pub fn set_filter(&mut self, f: Filter) {
        self.filter = f;
        self.marks_dirty = true;
    }
    pub fn set_dim_unchanged(&mut self, on: bool) {
        self.dim_unchanged = on;
        self.marks_dirty = true;
    }

    /// State for agents and tests (MCP `ui_state`).
    pub fn describe(&self) -> serde_json::Value {
        use serde_json::json;
        let d = self.data.as_ref();
        let sel = d.and_then(|d| {
            let v = self.primary_view();
            let r = self.views[v].selection?;
            let id = match r {
                Ref::Element(i) => d.model(v).elements.get(i)?.id.clone(),
                Ref::Flow(i) => d.model(v).flows.get(i)?.id.clone(),
            };
            let item = d.item_of(v, r).map(|i| &d.diff.items[i]);
            Some(json!({"id": id, "status": item.map(|it| status_key(it.status)), "changes": item.map(|it| it.props.len())}))
        });
        json!({
            "kind": "diff", "left": self.left.as_ref().map(|s| s.detail()), "right": self.right.as_ref().map(|s| s.detail()),
            "loaded": d.is_some(), "error": self.error, "mode": self.mode.key(), "counts": d.map(|d| counts_json(&d.diff)),
            "filter": {"added": self.filter.added, "removed": self.filter.removed, "changed": self.filter.changed, "moved": self.filter.moved},
            "dim_unchanged": self.dim_unchanged, "visible_changes": self.visible_items().len(), "selection": sel,
            "current_change": self.cursor.and_then(|c| d.map(|d| d.diff.items[c].id.clone())),
            "marked": self.overlays[self.primary_view()].diff.len(),
        })
    }

    pub fn view_mut(&mut self) -> &mut DiagramView {
        let v = self.primary_view();
        &mut self.views[v]
    }
    pub fn model_of_primary(&self) -> Option<&Model> {
        self.data.as_ref().map(|d| d.model(self.primary_view()))
    }

    pub fn set_mode(&mut self, mode: DiffMode) {
        if mode == self.mode {
            return;
        }
        let from = self.primary_view();
        self.mode = mode;
        let to: Vec<usize> = match mode {
            DiffMode::Unified => vec![MERGED],
            DiffMode::Before => vec![OLD],
            DiffMode::After => vec![NEW],
            DiffMode::SideBySide => vec![OLD, NEW],
        };
        self.active_view = to[0];
        // Same coordinate space: keep the camera and the selection where they were.
        let cam = self.views[from].camera.clone();
        let sel = self.views[from].selection;
        for &v in &to {
            if v != from {
                self.views[v].camera = cam.clone();
                self.carry_selection(from, sel, v);
            }
        }
        self.last_cameras = [None, None];
    }

    fn carry_selection(&mut self, from: usize, sel: Option<Ref>, to: usize) {
        let Some(d) = &self.data else { return };
        let r = sel.and_then(|r| d.ref_in(to, d.pair_of(from, r)));
        self.views[to].select(r);
    }

    fn set_source(&mut self, side: usize, s: DocSource) {
        if side == 0 {
            self.left = Some(s.clone());
        } else {
            self.right = Some(s.clone());
        }
        self.slots[side] = Some(DocSlot::new(s));
        self.reset();
    }

    pub fn swap(&mut self) {
        std::mem::swap(&mut self.left, &mut self.right);
        self.slots.swap(0, 1);
        self.reset();
    }

    fn reset(&mut self) {
        self.data = None;
        self.error = None;
        self.marks_dirty = true;
        self.cursor = None;
        for v in &mut self.views {
            v.select(None);
            v.fit_all();
        }
    }

    fn ensure_loaded(&mut self, app: &mut App) {
        if self.data.is_some() || self.error.is_some() {
            return;
        }
        let mut models: [Option<Arc<Model>>; 2] = [None, None];
        for (i, slot) in self.slots.iter_mut().enumerate() {
            let Some(slot) = slot else { return };
            match slot.state(app) {
                DocState::Ready(m) => models[i] = Some(m),
                DocState::Failed(e) => {
                    self.error = Some(format!("{}: {e}", slot.source.label(app)));
                    return;
                }
                DocState::Loading => return,
            }
        }
        if let [Some(old), Some(new)] = models {
            self.data = Some(DiffData::new(old, new));
            self.marks_dirty = true;
            for v in &mut self.views {
                v.fit_all();
            }
        }
    }

    fn rebuild_marks(&mut self) {
        let Some(d) = &self.data else { return };
        for o in &mut self.overlays {
            o.diff.clear();
            o.diff_ghosts.clear();
            o.dim_unchanged = if self.dim_unchanged { 1.0 } else { 0.0 };
        }
        for it in &d.diff.items {
            if !self.filter.shows(it.status) {
                continue;
            }
            let mark = match it.status {
                Status::Added => DiffMark::Added,
                Status::Removed => DiffMark::Removed,
                Status::Changed => DiffMark::Changed(it.props.len()),
                Status::Moved => DiffMark::Moved,
            };
            for v in [OLD, NEW, MERGED] {
                if let Some(r) = d.ref_in(v, DiffData::item_pair(it)) {
                    self.overlays[v].diff.insert(r, mark);
                }
            }
            // Where a moved element used to be, drawn in the views that show the new layout.
            if it.layout_changed && !it.is_flow && self.filter.moved {
                if let Some(Ref::Element(o)) = it.old {
                    let b = d.old.elements[o].bounds;
                    if b.is_valid() {
                        self.overlays[NEW].diff_ghosts.push(b);
                        self.overlays[MERGED].diff_ghosts.push(b);
                    }
                }
            }
        }
        self.marks_dirty = false;
    }

    /// Items in list order that the filter shows.
    fn visible_items(&self) -> Vec<usize> {
        let Some(d) = &self.data else { return vec![] };
        (0..d.diff.items.len()).filter(|&i| self.filter.shows(d.diff.items[i].status)).collect()
    }

    /// Select change `item` in the visible view(s) and bring it into view.
    pub fn go_to_item(&mut self, item: usize) {
        let Some(d) = &self.data else { return };
        let Some(it) = d.diff.items.get(item) else { return };
        let pair = DiffData::item_pair(it);
        self.cursor = Some(item);
        self.scroll_to_item = Some(item);
        let views: Vec<usize> = if self.mode == DiffMode::SideBySide { vec![OLD, NEW] } else { vec![self.primary_view()] };
        for v in views {
            let r = d.ref_in(v, pair);
            self.views[v].select(r);
            if let Some(r) = r {
                let m = d.model(v);
                match r {
                    Ref::Element(i) => self.views[v].fit_element(m, i),
                    Ref::Flow(_) => self.views[v].fit_refs(m, std::iter::once(r)),
                }
            }
        }
    }

    pub fn step(&mut self, forward: bool) {
        let items = self.visible_items();
        if items.is_empty() {
            return;
        }
        let pos = self.cursor.and_then(|c| items.iter().position(|&i| i == c));
        let next = match (pos, forward) {
            (None, true) => 0,
            (None, false) => items.len() - 1,
            (Some(p), true) => (p + 1) % items.len(),
            (Some(p), false) => (p + items.len() - 1) % items.len(),
        };
        self.go_to_item(items[next]);
    }

    /// The XML document of this diff and the element selected in the active view.
    pub fn xml_target(&self) -> Option<(XmlDoc, Option<String>)> {
        let (l, r) = (self.left.clone()?, self.right.clone()?);
        let id = self.data.as_ref().and_then(|d| {
            let v = self.primary_view();
            self.views[v].selection.map(|r| match r {
                Ref::Element(i) => d.model(v).elements[i].id.clone(),
                Ref::Flow(i) => d.model(v).flows[i].id.clone(),
            })
        });
        Some((XmlDoc::Diff { left: l, right: r }, id))
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let picked = self.picker.picked.lock().unwrap().take();
        if let Some((side, path)) = picked {
            self.picker.dialog_open = false;
            app.remember_file(&path);
            self.set_source(side, DocSource::File(path));
        }
        self.ensure_loaded(app);
        if self.marks_dirty {
            self.rebuild_marks();
        }
        self.toolbar(ui, app);
        if self.left.is_none() || self.right.is_none() || self.error.is_some() {
            self.setup_screen(ui, app);
            return;
        }
        if self.data.is_none() {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    ui.spinner();
                    ui.label(RichText::new(crate::t!("common-loading")).weak());
                });
            });
            return;
        }
        self.keys(ui);
        let mut actions: Vec<InspectorAction> = vec![];
        let mut dock_tab = self.dock_tab;
        let (main_rect, _) = super::dock(ui, app, "diff-dock", &DOCK_TABS, &mut dock_tab, |ui, tab, app| match tab {
            0 => self.changes_dock(ui, app),
            _ => self.inspector_dock(ui, app, &mut actions),
        });
        self.dock_tab = dock_tab;
        let mut main_ui = crate::widgets::split::child_in(ui, main_rect);
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(&mut main_ui, |ui| self.diagrams(ui, app, &mut actions));
        self.handle_actions(app, actions);
        app.status_hint = crate::t!("hint-diff");
    }

    fn keys(&mut self, ui: &egui::Ui) {
        let ctx = ui.ctx();
        let text_focus = ctx.memory(|m| m.focused()).map(|id| egui::text_edit::TextEditState::load(ctx, id).is_some()).unwrap_or(false);
        if text_focus {
            return;
        }
        let (prev, next) = ctx.input_mut(|i| {
            let prev = i.consume_key(Modifiers::SHIFT, Key::N) || i.consume_key(Modifiers::SHIFT, Key::F7);
            let next = i.consume_key(Modifiers::NONE, Key::N) || i.consume_key(Modifiers::NONE, Key::F7);
            (prev, next)
        });
        if prev || next {
            self.step(next);
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let tokens = app.tokens.clone();
        egui::Panel::top(("diff-toolbar", &self.left, &self.right)).frame(egui::Frame::new().fill(tokens.bg1).inner_margin(egui::Margin::symmetric(12, 6))).show(ui, |ui| {
            // Row 1: the two documents, and actions on the right.
            ui.horizontal(|ui| {
                self.source_chip(ui, app, 0, tokens.incident);
                if ui.add(egui::Button::new("⇄").frame(false)).on_hover_text(crate::t!("diff-swap-tip")).clicked() {
                    self.swap();
                }
                self.source_chip(ui, app, 1, tokens.completed);
                if self.data.is_none() {
                    return;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(crate::t!("diff-xml")).on_hover_text("Ctrl+U").clicked() {
                        if let Some((doc, id)) = self.xml_target() {
                            app.navigate(Nav::Xml { doc, element: id }, NavMode::Reuse);
                        }
                    }
                    if ui.small_button("↓").on_hover_text(crate::t!("diff-next-tip")).clicked() {
                        self.step(true);
                    }
                    if ui.small_button("↑").on_hover_text(crate::t!("diff-prev-tip")).clicked() {
                        self.step(false);
                    }
                });
            });
            if self.data.is_none() {
                return;
            }
            // Row 2: how to show it.
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                for m in DiffMode::ALL {
                    if ui.selectable_label(self.mode == m, m.label()).clicked() {
                        self.set_mode(m);
                    }
                }
                ui.separator();
                let c = self.data.as_ref().map(|d| d.diff.counts()).unwrap_or_default();
                let mut changed = false;
                for (on, n, color, text) in [
                    (&mut self.filter.added, c.added, tokens.completed, crate::t!("diff-added", n = c.added)),
                    (&mut self.filter.removed, c.removed, tokens.incident, crate::t!("diff-removed", n = c.removed)),
                    (&mut self.filter.changed, c.changed, tokens.suspended, crate::t!("diff-changed", n = c.changed)),
                    (&mut self.filter.moved, c.moved, app.tokens.diagram_style().diff_moved, crate::t!("diff-moved", n = c.moved)),
                ] {
                    if n == 0 {
                        continue;
                    }
                    let col = if *on { color } else { tokens.fg2 };
                    let pill = egui::Button::new(RichText::new(text).color(col).size(12.0)).fill(if *on { color.gamma_multiply(0.15) } else { Color32::TRANSPARENT }).stroke(egui::Stroke::new(1.0, col.gamma_multiply(0.6))).corner_radius(9.0);
                    if ui.add(pill).on_hover_text(crate::t!("diff-filter-tip")).clicked() {
                        *on = !*on;
                        changed = true;
                    }
                }
                if c.added + c.removed + c.changed + c.moved == 0 {
                    ui.label(RichText::new(crate::t!("diff-no-changes")).weak());
                }
                ui.separator();
                changed |= ui.checkbox(&mut self.dim_unchanged, crate::t!("diff-dim-unchanged")).changed();
                if changed {
                    self.marks_dirty = true;
                }
            });
        });
    }

    /// The button showing one side's document, with a menu to change it.
    fn source_chip(&mut self, ui: &mut egui::Ui, app: &mut App, side: usize, color: Color32) {
        let src = if side == 0 { self.left.clone() } else { self.right.clone() };
        let caption = crate::t!(if side == 0 { "diff-before" } else { "diff-after" });
        let text = match &src {
            Some(s) => format!("{caption}: {}  ▾", s.label(app)),
            None => format!("{caption}: {}  ▾", crate::t!("diff-choose")),
        };
        let b = ui.add(egui::Button::new(RichText::new(text).color(color).strong()).frame(true)).on_hover_text(src.as_ref().map(|s| s.detail()).unwrap_or_default());
        egui::Popup::menu(&b).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            ui.set_min_width(360.0);
            if let Some(s) = self.source_picker(ui, app, side) {
                self.set_source(side, s);
                ui.close();
            }
        });
    }

    /// Ways to pick a document: native dialog, typed path, recent files, open tabs and the
    /// engine's definitions. Returns the choice once made.
    fn source_picker(&mut self, ui: &mut egui::Ui, app: &mut App, side: usize) -> Option<DocSource> {
        let mut out = None;
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.picker.dialog_open, egui::Button::new(crate::t!("diff-pick-file"))).clicked() {
                self.picker.dialog_open = true;
                let slot = self.picker.picked.clone();
                let title = crate::t!("diff-pick-file-title");
                let start = app.settings.recent_files.first().and_then(|f| std::path::Path::new(f).parent().map(|p| p.to_path_buf()));
                let ctx = ui.ctx().clone();
                std::thread::spawn(move || {
                    let picked = crate::platform::files::pick_bpmn_file(&title, start.as_deref());
                    // Even a cancel must re-enable the button.
                    *slot.lock().unwrap() = picked.map(|p| (side, p));
                    ctx.request_repaint();
                });
            }
            if self.picker.dialog_open {
                ui.spinner();
            }
        });
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut self.picker.path[side]).hint_text(crate::t!("diff-path-hint")).desired_width(280.0));
            let go = ui.button(crate::t!("diff-use-path")).clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)));
            if go && !self.picker.path[side].trim().is_empty() {
                let p = crate::paths::expand_tilde(self.picker.path[side].trim());
                if p.is_file() {
                    app.remember_file(&p);
                    out = Some(DocSource::File(p));
                } else {
                    app.toasts.error(crate::t!("diff-no-such-file"), Some(p.display().to_string()));
                }
            }
        });
        // Documents open in tabs and recent files.
        let mut quick: Vec<DocSource> = vec![];
        for t in &app.tabs {
            match t.place() {
                crate::state::Place::File(p) => quick.push(DocSource::File(p.path.clone())),
                crate::state::Place::Definition(p) => quick.push(DocSource::Definition(p.def_id.clone())),
                _ => {}
            }
        }
        for f in &app.settings.recent_files {
            let s = DocSource::File(PathBuf::from(f));
            if !quick.contains(&s) {
                quick.push(s);
            }
        }
        if !quick.is_empty() {
            ui.separator();
            ui.label(RichText::new(crate::t!("diff-recent")).weak().small());
            for s in quick.into_iter().take(10) {
                if ui.button(s.label(app)).on_hover_text(s.detail()).clicked() {
                    out = Some(s);
                }
            }
        }
        if app.net.is_some() {
            ui.separator();
            ui.label(RichText::new(crate::t!("diff-engine-definitions")).weak().small());
            app.ensure_definitions();
            ui.add(egui::TextEdit::singleline(&mut self.picker.filter).hint_text(crate::t!("diff-filter-hint")).desired_width(f32::INFINITY));
            let f = self.picker.filter.to_lowercase();
            let mut defs: Vec<camunda::ProcessDefinition> = app.cache.all_defs.data.clone().unwrap_or_default().into_iter().filter(|d| f.is_empty() || d.key.to_lowercase().contains(&f) || d.name.as_deref().unwrap_or("").to_lowercase().contains(&f)).collect();
            defs.sort_by(|a, b| a.key.cmp(&b.key).then(b.version.cmp(&a.version)));
            egui::ScrollArea::vertical().max_height(260.0).id_salt(("diff-defs", side)).show(ui, |ui| {
                for d in defs.iter().take(400) {
                    let label = format!("{}  v{}{}", d.key, d.version, d.name.as_deref().map(|n| format!("  · {n}")).unwrap_or_default());
                    if ui.selectable_label(false, label).clicked() {
                        app.cache.remember_def(d.clone());
                        out = Some(DocSource::Definition(d.id.clone()));
                    }
                }
            });
        }
        out
    }

    fn setup_screen(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let tokens = app.tokens.clone();
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(crate::t!("diff-setup-title")).size(18.0).strong());
                ui.label(RichText::new(crate::t!("diff-setup-body")).weak());
                if let Some(e) = &self.error {
                    ui.add_space(6.0);
                    ui.label(RichText::new(e).color(tokens.incident));
                }
            });
            ui.add_space(12.0);
            ui.columns(2, |cols| {
                for (side, ui) in cols.iter_mut().enumerate() {
                    let color = if side == 0 { tokens.incident } else { tokens.completed };
                    egui::Frame::new().fill(tokens.bg1).corner_radius(8.0).inner_margin(12.0).stroke(egui::Stroke::new(1.0, color.gamma_multiply(0.5))).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let caption = crate::t!(if side == 0 { "diff-before" } else { "diff-after" });
                        let cur = if side == 0 { self.left.clone() } else { self.right.clone() };
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(caption).strong().color(color));
                            if let Some(s) = &cur {
                                ui.label(RichText::new(s.label(app)).strong()).on_hover_text(s.detail());
                            }
                        });
                        ui.add_space(6.0);
                        if let Some(s) = self.source_picker(ui, app, side) {
                            self.set_source(side, s);
                        }
                    });
                }
            });
        });
    }

    fn diagrams(&mut self, ui: &mut egui::Ui, app: &mut App, actions: &mut Vec<InspectorAction>) {
        let style = app.tokens.diagram_style();
        let settings = app.diagram_settings();
        let editor = !app.settings.xml_editor.trim().is_empty();
        let rect = ui.available_rect_before_wrap();
        let views: Vec<(usize, egui::Rect)> = match self.mode {
            DiffMode::SideBySide => {
                let w = (rect.width() - 2.0) / 2.0;
                vec![(OLD, egui::Rect::from_min_size(rect.min, Vec2::new(w, rect.height()))), (NEW, egui::Rect::from_min_size(rect.min + Vec2::new(w + 2.0, 0.0), Vec2::new(w, rect.height())))]
            }
            _ => vec![(self.primary_view(), rect)],
        };
        let Some(d) = &self.data else { return };
        let mut changed_selection: Option<(usize, Option<Ref>)> = None;
        for (v, r) in &views {
            let model = d.model(*v);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(*r));
            let items = &d.diff.items;
            let by_id = &d.by_id;
            let extra = |ui: &mut egui::Ui, id: &str| {
                if let Some(it) = by_id.get(id).and_then(|&i| items.get(i)) {
                    tooltip_change(ui, it);
                }
            };
            let out = viewport::Diagram { model, style: &style, settings: &settings }.show(&mut child, &mut self.views[*v], &mut self.overlays[*v], &extra, &mut |ui, target| {
                crate::app::diagram_context_menu(ui, model, target, editor, actions);
            });
            if out.selection_changed || out.clicked.is_some() {
                self.active_view = *v;
            }
            // Clicks, keys and agents all count: compare with the last frame.
            if self.views[*v].selection != self.last_selection[*v] {
                changed_selection = Some((*v, self.views[*v].selection));
            }
            if out.copied {
                app.toasts.push(crate::widgets::ToastKind::Info, crate::t!("toast-element-id-copied"));
            }
            if *v == self.primary_view() {
                app.last_diagram_viewport = out.viewport;
            }
            if self.mode == DiffMode::SideBySide {
                let label = crate::t!(if *v == OLD { "diff-before" } else { "diff-after" });
                let color = if *v == OLD { style.diff_removed } else { style.diff_added };
                let p = r.left_top() + Vec2::new(10.0, 8.0);
                let g = ui.painter().layout_no_wrap(label, FontId::proportional(12.0), color);
                ui.painter().rect_filled(egui::Rect::from_min_size(p - Vec2::new(6.0, 3.0), g.size() + Vec2::new(12.0, 6.0)), 6.0, style.canvas.gamma_multiply(0.9));
                ui.painter().galley(p, g, color);
            }
        }
        if self.mode == DiffMode::SideBySide {
            ui.painter().vline(rect.center().x, rect.y_range(), egui::Stroke::new(2.0, app.tokens.bg3));
            self.sync_cameras();
        }
        if let Some((v, sel)) = changed_selection {
            for other in [OLD, NEW, MERGED] {
                if other != v {
                    self.carry_selection(v, sel, other);
                }
            }
            self.active_view = if self.mode == DiffMode::SideBySide { v } else { self.active_view };
            if let (Some(d), Some(r)) = (&self.data, sel) {
                if let Some(i) = d.item_of(v, r) {
                    if self.cursor != Some(i) {
                        self.scroll_to_item = Some(i);
                    }
                    self.cursor = Some(i);
                }
            }
        }
        for v in [OLD, NEW, MERGED] {
            self.last_selection[v] = self.views[v].selection;
        }
    }

    /// Side by side: whichever camera moved since the last frame drives the other.
    fn sync_cameras(&mut self) {
        let a = self.views[OLD].camera.target;
        let b = self.views[NEW].camera.target;
        let moved_a = self.last_cameras[0].is_some_and(|l| l != a);
        let moved_b = self.last_cameras[1].is_some_and(|l| l != b);
        if moved_a && !moved_b {
            self.views[NEW].camera.target = a;
        } else if moved_b && !moved_a {
            self.views[OLD].camera.target = b;
        } else if self.last_cameras[0].is_none() {
            self.views[NEW].camera = self.views[OLD].camera.clone();
        }
        self.last_cameras = [Some(self.views[OLD].camera.target), Some(self.views[NEW].camera.target)];
    }

    fn changes_dock(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let tokens = app.tokens.clone();
        let Some(d) = &self.data else { return };
        let mut go: Option<usize> = None;
        if !d.diff.document.is_empty() {
            crate::widgets::section(ui, "diff-document", "Document", true, |ui| {
                for c in &d.diff.document {
                    prop_change(ui, c, &tokens);
                }
            });
        }
        let visible = self.visible_items();
        if visible.is_empty() {
            crate::widgets::empty_state(ui, &crate::t!(if d.diff.items.is_empty() { "diff-no-changes" } else { "diff-all-filtered" }));
        }
        let style = tokens.diagram_style();
        let scroll_to = self.scroll_to_item.take();
        for i in visible {
            let it = &d.diff.items[i];
            let selected = self.cursor == Some(i);
            let (sign, color) = match it.status {
                Status::Added => ("+", style.diff_added),
                Status::Removed => ("−", style.diff_removed),
                Status::Changed => ("~", style.diff_changed),
                Status::Moved => ("↔", style.diff_moved),
            };
            let frame = egui::Frame::new().fill(if selected { tokens.bg2 } else { Color32::TRANSPARENT }).corner_radius(5.0).inner_margin(egui::Margin::symmetric(6, 3)).stroke(egui::Stroke::new(1.0, if selected { color.gamma_multiply(0.6) } else { Color32::TRANSPARENT }));
            let r = frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                // Right-hand bits first so the label truncates into the space that is left.
                let head = ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(crate::util::ellipsize(&it.id, 24)).monospace().size(10.5).weak());
                        if it.layout_changed && it.status == Status::Changed {
                            ui.label(RichText::new("↔").color(style.diff_moved)).on_hover_text(crate::t!("diff-also-moved"));
                        }
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.label(RichText::new(sign).color(color).monospace().strong());
                            ui.add(egui::Label::new(RichText::new(&it.label).color(tokens.fg0)).truncate().sense(egui::Sense::click()))
                        })
                        .inner
                    })
                    .inner
                });
                if head.inner.clicked() {
                    go = Some(i);
                }
                if selected {
                    if let Some(o) = &it.old_id {
                        ui.label(RichText::new(crate::t!("diff-id-changed", old = o.clone())).weak().small());
                    }
                    for c in &it.props {
                        prop_change(ui, c, &tokens);
                    }
                    if it.status == Status::Moved {
                        ui.label(RichText::new(crate::t!("diff-only-layout")).weak().small());
                    }
                }
            });
            if scroll_to == Some(i) {
                r.response.scroll_to_me(Some(egui::Align::Center));
            }
        }
        if let Some(i) = go {
            self.go_to_item(i);
        }
    }

    fn inspector_dock(&mut self, ui: &mut egui::Ui, app: &mut App, actions: &mut Vec<InspectorAction>) {
        let tokens = app.tokens.clone();
        let v = self.primary_view();
        let Some(d) = &self.data else { return };
        let Some(sel) = self.views[v].selection else {
            crate::widgets::empty_state(ui, &crate::t!("inspector-empty-file"));
            return;
        };
        if let Some(i) = d.item_of(v, sel) {
            let it = &d.diff.items[i];
            crate::widgets::section(ui, ("diff-item", &it.id), "Changes", true, |ui| {
                ui.label(RichText::new(crate::t!(match it.status {
                    Status::Added => "diff-status-added",
                    Status::Removed => "diff-status-removed",
                    Status::Changed => "diff-status-changed",
                    Status::Moved => "diff-status-moved",
                })).weak());
                for c in &it.props {
                    prop_change(ui, c, &tokens);
                }
            });
        }
        actions.extend(inspector::show(ui, d.model(v), sel, &RuntimeFacts::default(), false));
    }

    fn handle_actions(&mut self, app: &mut App, actions: Vec<InspectorAction>) {
        let Some(d) = &self.data else { return };
        let v = self.primary_view();
        let sides = [(self.right.clone(), d.new.clone()), (self.left.clone(), d.old.clone())];
        let sides: Vec<(DocSource, Arc<Model>)> = sides.into_iter().filter_map(|(s, m)| s.map(|s| (s, m))).collect();
        for a in actions {
            if let Some((doc, _)) = self.xml_target() {
                if super::xml::handle_action(app, doc, &sides, &a) {
                    continue;
                }
            }
            let Some(d) = &self.data else { return };
            match a {
                InspectorAction::Select(r) => {
                    self.views[v].select(Some(r));
                    self.views[v].fit_refs(d.model(v), std::iter::once(r));
                }
                InspectorAction::FitDiagram => self.views[v].fit_all(),
                InspectorAction::OpenDefinitionKey(_) | InspectorAction::InstancesAt(_) | InspectorAction::ExecutedAt(_) | InspectorAction::IncidentsAt(_) => {
                    app.toasts.push(crate::widgets::ToastKind::Info, crate::t!("file-needs-engine"));
                }
                InspectorAction::ViewXml(_) | InspectorAction::XmlExternal(..) => {}
            }
        }
    }
}

/// Hover tooltip lines for a changed element or flow.
fn tooltip_change(ui: &mut egui::Ui, it: &ItemDiff) {
    let key = match it.status {
        Status::Added => "diff-status-added",
        Status::Removed => "diff-status-removed",
        Status::Changed => "diff-status-changed",
        Status::Moved => "diff-status-moved",
    };
    ui.separator();
    ui.label(RichText::new(crate::t!(key)).strong());
    for c in it.props.iter().take(5) {
        let one = |s: &Option<String>| s.as_deref().map(|v| crate::util::ellipsize(v.lines().next().unwrap_or(""), 40)).unwrap_or_else(|| "∅".into());
        ui.label(RichText::new(format!("{}: {} → {}", c.key, one(&c.old), one(&c.new))).monospace().size(11.0));
    }
    if it.props.len() > 5 {
        ui.label(RichText::new(crate::t!("diff-more-changes", n = it.props.len() - 5)).weak().small());
    }
}

/// One property change, git style: the key, then `-` old and `+` new lines (a line diff for
/// multi-line values, with the changed words emphasised).
pub fn prop_change(ui: &mut egui::Ui, c: &PropChange, tokens: &crate::theme::Tokens) {
    ui.add_space(3.0);
    ui.label(RichText::new(&c.key).size(12.0).strong().color(tokens.fg1));
    let old = c.old.as_deref().unwrap_or("");
    let new = c.new.as_deref().unwrap_or("");
    let font = FontId::monospace(11.5);
    let ins_bg = tokens.completed.gamma_multiply(if tokens.dark { 0.16 } else { 0.13 });
    let del_bg = tokens.incident.gamma_multiply(if tokens.dark { 0.16 } else { 0.11 });
    let ins_em = tokens.completed.gamma_multiply(if tokens.dark { 0.40 } else { 0.32 });
    let del_em = tokens.incident.gamma_multiply(if tokens.dark { 0.40 } else { 0.28 });
    let old_lines = bpmn::diff::split_lines(old);
    let new_lines = bpmn::diff::split_lines(new);
    let rows = if c.old.is_none() {
        (0..new_lines.len()).map(|i| bpmn::diff::TextRow { tag: LineTag::Insert, old_line: None, new_line: Some(i) }).collect()
    } else if c.new.is_none() {
        (0..old_lines.len()).map(|i| bpmn::diff::TextRow { tag: LineTag::Delete, old_line: Some(i), new_line: None }).collect()
    } else {
        bpmn::diff::text_rows(old, new)
    };
    // Pair single replaced lines for word emphasis.
    let mut emph: HashMap<usize, Vec<std::ops::Range<usize>>> = HashMap::new();
    let mut i = 0;
    while i < rows.len() {
        if rows[i].tag == LineTag::Equal {
            i += 1;
            continue;
        }
        let s = i;
        while i < rows.len() && rows[i].tag != LineTag::Equal {
            i += 1;
        }
        let dels: Vec<usize> = (s..i).filter(|&k| rows[k].tag == LineTag::Delete).collect();
        let ins: Vec<usize> = (s..i).filter(|&k| rows[k].tag == LineTag::Insert).collect();
        for (a, b) in dels.iter().zip(ins.iter()) {
            let (ol, nl) = (old_lines[rows[*a].old_line.unwrap()], new_lines[rows[*b].new_line.unwrap()]);
            let (x, y) = bpmn::diff::inline_ranges(ol, nl);
            emph.insert(*a, tidy_emphasis(ol, x));
            emph.insert(*b, tidy_emphasis(nl, y));
        }
    }
    // Long unchanged stretches inside a value collapse to a marker.
    let shown: Vec<bool> = (0..rows.len()).map(|k| rows[k].tag != LineTag::Equal || (k.saturating_sub(2)..(k + 3).min(rows.len())).any(|j| rows[j].tag != LineTag::Equal)).collect();
    let mut skipped = false;
    let w = ui.available_width();
    for (k, row) in rows.iter().enumerate() {
        if !shown[k] {
            if !skipped {
                ui.label(RichText::new("  ⋯").weak().monospace());
                skipped = true;
            }
            continue;
        }
        skipped = false;
        let (sign, text, bg, em) = match row.tag {
            LineTag::Insert => ("+ ", new_lines[row.new_line.unwrap()], ins_bg, ins_em),
            LineTag::Delete => ("- ", old_lines[row.old_line.unwrap()], del_bg, del_em),
            LineTag::Equal => ("  ", new_lines[row.new_line.unwrap()], Color32::TRANSPARENT, Color32::TRANSPARENT),
        };
        let color = match row.tag {
            LineTag::Insert => tokens.completed,
            LineTag::Delete => tokens.incident,
            LineTag::Equal => tokens.fg2,
        };
        let mut job = LayoutJob::default();
        job.append(sign, 0.0, TextFormat { font_id: font.clone(), color, ..Default::default() });
        let marks = emph.get(&k).cloned().unwrap_or_default();
        let mut pos = 0;
        let txt_color = if row.tag == LineTag::Equal { tokens.fg1 } else { tokens.fg0 };
        for m in marks.iter().filter(|m| m.end <= text.len() && text.is_char_boundary(m.start) && text.is_char_boundary(m.end)) {
            if m.start > pos {
                job.append(&text[pos..m.start], 0.0, TextFormat { font_id: font.clone(), color: txt_color, ..Default::default() });
            }
            job.append(&text[m.start..m.end], 0.0, TextFormat { font_id: font.clone(), color: txt_color, background: em, ..Default::default() });
            pos = m.end;
        }
        if pos < text.len() {
            job.append(&text[pos..], 0.0, TextFormat { font_id: font.clone(), color: txt_color, ..Default::default() });
        }
        if text.is_empty() && row.tag != LineTag::Equal {
            job.append(" ", 0.0, TextFormat { font_id: font.clone(), color: txt_color, ..Default::default() });
        }
        job.wrap.max_width = w - 8.0;
        egui::Frame::new().fill(bg).inner_margin(egui::Margin::symmetric(4, 1)).show(ui, |ui| {
            ui.set_width(w - 8.0);
            ui.add(egui::Label::new(job).selectable(true));
        });
    }
}

/// Word emphasis that helps reading: ranges separated only by spaces or punctuation are joined,
/// and none at all when most of the line changed (the line colour already says so).
pub fn tidy_emphasis(text: &str, ranges: Vec<std::ops::Range<usize>>) -> Vec<std::ops::Range<usize>> {
    let mut out: Vec<std::ops::Range<usize>> = vec![];
    for r in ranges {
        if let Some(last) = out.last_mut() {
            if r.start >= last.end && text.get(last.end..r.start).is_some_and(|gap| gap.chars().all(|c| c.is_whitespace() || c.is_ascii_punctuation())) {
                last.end = r.end;
                continue;
            }
        }
        out.push(r);
    }
    let changed: usize = out.iter().map(|r| text.get(r.clone()).map(|s| s.trim().chars().count()).unwrap_or(0)).sum();
    let total = text.trim().chars().count().max(1);
    if changed * 10 > total * 6 {
        return vec![];
    }
    out
}

/// Status filter set for tests and MCP.
pub fn status_key(s: Status) -> &'static str {
    match s {
        Status::Added => "added",
        Status::Removed => "removed",
        Status::Changed => "changed",
        Status::Moved => "moved",
    }
}

/// Item indices of a diff grouped for summaries (used by MCP).
pub fn counts_json(d: &ModelDiff) -> serde_json::Value {
    let c = d.counts();
    serde_json::json!({"added": c.added, "removed": c.removed, "changed": c.changed, "moved": c.moved, "document": c.document})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emphasis_joins_neighbouring_words_and_drops_when_most_changed() {
        let t = "name: paymentReceived now";
        let r = tidy_emphasis(t, vec![6..13, 13..21]);
        assert_eq!(r, vec![6..21]);
        let t = "a b c d e";
        assert!(tidy_emphasis(t, vec![0..1, 2..3, 4..5, 6..7]).is_empty(), "mostly changed: no emphasis");
        let t = "x = 1000, y = 2";
        let only = tidy_emphasis(t, std::iter::once(4..8).collect());
        assert_eq!(only.len(), 1);
        assert_eq!(only[0], 4..8);
    }
}
