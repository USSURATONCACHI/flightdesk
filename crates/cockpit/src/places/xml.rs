//! The XML viewer: a BPMN document's source (or a git-style line diff of two documents) with
//! syntax colouring, line numbers, a removable highlight of the element it was opened for,
//! find, folding of unchanged runs, and links back to the diagram.

use super::source::{path_on_disk, DocSlot, DocSource, DocState};
use crate::app::App;
use crate::state::{Nav, NavMode};
use crate::widgets::ToastKind;
use crate::xmltext::{self, LineIndex, State, Tok};
use bpmn::diff::{LineTag, TextRow};
use bpmn::Model;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Key, Modifiers, RichText, Sense, Stroke, Vec2};
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;

/// What an XML tab shows.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum XmlDoc {
    Single(DocSource),
    /// `left` is the old (before) document, `right` the new (after) one.
    Diff { left: DocSource, right: DocSource },
}

impl XmlDoc {
    /// The diagram place that belongs to this document.
    pub fn diagram_nav(&self) -> Nav {
        match self {
            XmlDoc::Single(s) => s.diagram_nav(),
            XmlDoc::Diff { left, right } => Nav::Diff { left: Some(left.clone()), right: Some(right.clone()) },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Old,
    New,
}

/// The highlighted element: a byte range in one side's text.
#[derive(Clone, Debug, PartialEq)]
pub struct Highlight {
    pub side: Side,
    pub range: Range<usize>,
    pub id: String,
}

struct SideText {
    model: Arc<Model>,
    index: LineIndex,
    states: Vec<State>,
}

impl SideText {
    fn new(model: Arc<Model>) -> Self {
        let index = LineIndex::new(&model.source);
        let states = xmltext::line_states(&model.source, &index);
        Self { model, index, states }
    }
    fn line(&self, i: usize) -> &str {
        self.index.line(&self.model.source, i)
    }
}

struct Doc {
    new: SideText,
    old: Option<SideText>,
    rows: Vec<TextRow>,
    /// For a deleted or inserted row: the row it replaces (for intra-line emphasis).
    pairs: Vec<Option<usize>>,
    max_chars: usize,
    added: usize,
    deleted: usize,
}

impl Doc {
    fn side(&self, s: Side) -> &SideText {
        match s {
            Side::New => &self.new,
            Side::Old => self.old.as_ref().unwrap_or(&self.new),
        }
    }
    /// Which side and line a row displays.
    fn row_line(&self, r: usize) -> (Side, usize) {
        let row = self.rows[r];
        match row.tag {
            LineTag::Delete => (Side::Old, row.old_line.unwrap_or(0)),
            _ => (Side::New, row.new_line.unwrap_or(0)),
        }
    }
    fn row_text(&self, r: usize) -> &str {
        let (s, l) = self.row_line(r);
        self.side(s).line(l)
    }
    /// The row showing `line` of `side` (equal rows show both sides).
    fn row_of(&self, side: Side, line: usize) -> Option<usize> {
        if self.old.is_none() {
            return (line < self.rows.len()).then_some(line);
        }
        self.rows.iter().position(|r| match side {
            Side::New => r.new_line == Some(line),
            Side::Old => r.old_line == Some(line),
        })
    }
    fn line_on(&self, r: usize, side: Side) -> Option<usize> {
        match side {
            Side::New => self.rows[r].new_line,
            Side::Old => self.rows[r].old_line,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DRow {
    Row(usize),
    /// Hidden unchanged rows `from..to` (indices into `Doc::rows`).
    Fold { from: usize, to: usize },
}

#[derive(Default)]
struct Find {
    open: bool,
    query: String,
    want_focus: bool,
    /// (row, byte range in the row's text)
    matches: Vec<(usize, Range<usize>)>,
    cursor: usize,
    computed_for: Option<String>,
}

const CONTEXT: usize = 3;
const MAX_LINE_CHARS: usize = 6000;

pub struct XmlPlace {
    pub doc: XmlDoc,
    new_slot: DocSlot,
    old_slot: Option<DocSlot>,
    data: Option<Doc>,
    error: Option<String>,
    /// Element to bring into view and highlight once the text is loaded.
    pub focus: Option<String>,
    pub highlight: Option<Highlight>,
    /// Diff mode: show unchanged lines instead of folding them.
    pub whole_file: bool,
    expanded: HashSet<usize>,
    display: Vec<DRow>,
    display_dirty: bool,
    scroll_to: Option<(usize, usize)>,
    find: Find,
    inline_cache: HashMap<usize, Vec<Range<usize>>>,
    inline_done: bool,
    /// Diff mode: the change hunk last jumped to (for n / N).
    hunk_cursor: Option<usize>,
}

impl XmlPlace {
    pub fn new(doc: XmlDoc, focus: Option<String>) -> Self {
        let (new_slot, old_slot) = match &doc {
            XmlDoc::Single(s) => (DocSlot::new(s.clone()), None),
            XmlDoc::Diff { left, right } => (DocSlot::new(right.clone()), Some(DocSlot::new(left.clone()))),
        };
        Self {
            doc,
            new_slot,
            old_slot,
            data: None,
            error: None,
            focus,
            highlight: None,
            whole_file: false,
            expanded: HashSet::new(),
            display: vec![],
            display_dirty: true,
            scroll_to: None,
            find: Find::default(),
            inline_cache: HashMap::new(),
            inline_done: false,
            hunk_cursor: None,
        }
    }

    pub fn title(&self) -> String {
        // Titles cannot reach the app cache; use the bare source names.
        let name = |s: &DocSource| match s {
            DocSource::File(p) => p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(),
            DocSource::Definition(id) => super::source::definition_key_version(id).map(|(k, v)| format!("{k} v{v}")).unwrap_or_else(|| crate::util::short_uuid(id)),
        };
        match &self.doc {
            XmlDoc::Single(s) => format!("{} · XML", name(s)),
            XmlDoc::Diff { left, right } => format!("{} → {} · XML", name(left), name(right)),
        }
    }

    pub fn is_diff(&self) -> bool {
        matches!(self.doc, XmlDoc::Diff { .. })
    }

    /// Point the view at another element (an already open tab being reused).
    pub fn retarget(&mut self, element: Option<String>) {
        if element.is_some() {
            self.focus = element;
        }
    }

    pub fn open_find(&mut self) {
        self.find.open = true;
        self.find.want_focus = true;
    }

    /// Search for `q` (empty closes the find bar) and jump to the first match.
    pub fn set_find(&mut self, q: &str) {
        self.find.open = !q.is_empty();
        self.find.query = q.to_string();
        self.recompute_find();
        if !self.find.matches.is_empty() {
            self.find.cursor = 0;
            self.go_to_match(0);
        }
    }
    pub fn step_find(&mut self, forward: bool) {
        self.go_to_match(if forward { 1 } else { -1 });
    }
    pub fn step_change(&mut self, forward: bool) {
        self.go_to_hunk(forward);
    }
    pub fn set_whole_file(&mut self, on: bool) {
        self.whole_file = on;
        self.display_dirty = true;
    }

    /// State for agents and tests (MCP `ui_state`).
    pub fn describe(&self) -> serde_json::Value {
        use serde_json::json;
        let src = |s: &DocSource| s.detail();
        let (left, right) = match &self.doc {
            XmlDoc::Single(s) => (None, src(s)),
            XmlDoc::Diff { left, right } => (Some(src(left)), src(right)),
        };
        let d = self.data.as_ref();
        let hl = self.highlight.as_ref().map(|h| {
            let (line, col) = d.map(|d| {
                let s = d.side(h.side);
                s.index.line_col(&s.model.source, h.range.start)
            }).unwrap_or((0, 0));
            json!({"id": h.id, "side": if h.side == Side::New { "new" } else { "old" }, "line": line, "col": col})
        });
        let folds = self.display.iter().filter(|r| matches!(r, DRow::Fold { .. })).count();
        json!({
            "kind": "xml", "diff": self.is_diff(), "left": left, "right": right, "loaded": d.is_some(), "error": self.error,
            "lines": d.map(|d| d.rows.len()), "added": d.map(|d| d.added), "deleted": d.map(|d| d.deleted),
            "displayed_rows": self.display.len(), "folds": folds, "whole_file": self.whole_file, "highlight": hl,
            "find": self.find.open.then(|| json!({"query": self.find.query, "matches": self.find.matches.len(), "current": self.find.cursor})),
            "pending_focus": self.focus,
        })
    }

    fn ensure_loaded(&mut self, app: &mut App) {
        if self.data.is_some() || self.error.is_some() {
            return;
        }
        let new = match self.new_slot.state(app) {
            DocState::Ready(m) => m,
            DocState::Failed(e) => {
                self.error = Some(e);
                return;
            }
            DocState::Loading => return,
        };
        let old = match self.old_slot.as_mut().map(|s| s.state(app)) {
            None => None,
            Some(DocState::Ready(m)) => Some(m),
            Some(DocState::Failed(e)) => {
                self.error = Some(e);
                return;
            }
            Some(DocState::Loading) => return,
        };
        self.data = Some(build_doc(new, old));
        self.display_dirty = true;
    }

    fn rebuild_display(&mut self) {
        let Some(d) = &self.data else { return };
        self.display = if d.old.is_none() || self.whole_file || (d.added == 0 && d.deleted == 0) {
            (0..d.rows.len()).map(DRow::Row).collect()
        } else {
            fold_rows(&d.rows, &self.expanded)
        };
        self.display_dirty = false;
    }

    /// Display index of a row, unfolding it first when hidden.
    fn reveal_row(&mut self, row: usize) -> Option<usize> {
        if self.display_dirty {
            self.rebuild_display();
        }
        if let Some(i) = self.display.iter().position(|d| *d == DRow::Row(row)) {
            return Some(i);
        }
        let fold = self.display.iter().find_map(|d| match d {
            DRow::Fold { from, to } if (*from..*to).contains(&row) => Some(*from),
            _ => None,
        })?;
        self.expanded.insert(fold);
        self.rebuild_display();
        self.display.iter().position(|d| *d == DRow::Row(row))
    }

    /// Resolve `focus` (an element id) to a highlight and a scroll target.
    fn apply_focus(&mut self) {
        let Some(id) = self.focus.take() else { return };
        let Some(d) = &self.data else {
            self.focus = Some(id);
            return;
        };
        let find = |m: &Model| match m.lookup(&id) {
            Some(bpmn::Ref::Element(i)) => Some(m.elements[i].xml_range.clone()),
            Some(bpmn::Ref::Flow(i)) => Some(m.flows[i].xml_range.clone()),
            None => None,
        };
        let hit = find(&d.new.model).map(|r| (Side::New, r)).or_else(|| d.old.as_ref().and_then(|o| find(&o.model)).map(|r| (Side::Old, r)));
        let Some((side, range)) = hit else { return };
        let st = d.side(side);
        let line = st.index.line_of(range.start);
        let col = st.index.line_col(&st.model.source, range.start).1 - 1;
        let row = d.row_of(side, line);
        // Show the whole element: unfold unchanged runs inside it.
        let last = st.index.line_of(range.end.saturating_sub(1).max(range.start));
        if let (Some(a), Some(b)) = (row, d.row_of(side, last)) {
            if self.display_dirty {
                self.rebuild_display();
            }
            let inside: Vec<usize> = self.display.iter().filter_map(|x| match x {
                DRow::Fold { from, to } if *from <= b && a < *to => Some(*from),
                _ => None,
            }).collect();
            if !inside.is_empty() {
                self.expanded.extend(inside);
                self.display_dirty = true;
            }
        }
        self.highlight = Some(Highlight { side, range, id });
        if let Some(row) = row {
            if let Some(di) = self.reveal_row(row) {
                self.scroll_to = Some((di, col));
            }
        }
    }

    fn recompute_find(&mut self) {
        let Some(d) = &self.data else { return };
        if self.find.computed_for.as_deref() == Some(self.find.query.as_str()) {
            return;
        }
        let q = self.find.query.to_lowercase();
        self.find.matches.clear();
        if !q.is_empty() {
            for r in 0..d.rows.len() {
                for m in xmltext::find_in_line(d.row_text(r), &q) {
                    self.find.matches.push((r, m));
                    if self.find.matches.len() >= 100_000 {
                        break;
                    }
                }
            }
        }
        self.find.cursor = 0;
        self.find.computed_for = Some(self.find.query.clone());
    }

    fn go_to_match(&mut self, delta: isize) {
        if self.find.matches.is_empty() {
            return;
        }
        let n = self.find.matches.len() as isize;
        self.find.cursor = (self.find.cursor as isize + delta).rem_euclid(n) as usize;
        let (row, range) = self.find.matches[self.find.cursor].clone();
        let col = self.data.as_ref().map(|d| d.row_text(row)[..range.start].chars().count()).unwrap_or(0);
        if let Some(di) = self.reveal_row(row) {
            self.scroll_to = Some((di, col));
        }
    }

    /// Diff mode: jump to the next / previous block of changed rows.
    fn go_to_hunk(&mut self, forward: bool) {
        let Some(d) = &self.data else { return };
        let starts: Vec<usize> = (0..d.rows.len()).filter(|&r| d.rows[r].tag != LineTag::Equal && (r == 0 || d.rows[r - 1].tag == LineTag::Equal)).collect();
        if starts.is_empty() {
            return;
        }
        let next = match (self.hunk_cursor, forward) {
            (None, true) => 0,
            (None, false) => starts.len() - 1,
            (Some(c), true) => (c + 1) % starts.len(),
            (Some(c), false) => (c + starts.len() - 1) % starts.len(),
        };
        self.hunk_cursor = Some(next);
        if let Some(di) = self.reveal_row(starts[next]) {
            self.scroll_to = Some((di, 0));
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        self.ensure_loaded(app);
        let tokens = app.tokens.clone();
        if self.data.is_some() && self.focus.is_some() {
            self.apply_focus();
        }
        if self.display_dirty {
            self.rebuild_display();
        }
        self.toolbar(ui, app);
        if self.find.open {
            self.find_bar(ui, &tokens);
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(tokens.bg0)).show(ui, |ui| {
            if let Some(e) = &self.error {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| ui.label(RichText::new(crate::t!("xml-load-failed", error = e.clone())).color(tokens.incident)));
                return;
            }
            if self.data.is_none() {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.spinner();
                    ui.label(RichText::new(crate::t!("common-loading")).weak());
                });
                return;
            }
            self.keys(ui);
            self.text_area(ui, app);
        });
        app.status_hint = crate::t!(if self.is_diff() { "hint-xml-diff" } else { "hint-xml" });
    }

    fn keys(&mut self, ui: &egui::Ui) {
        let ctx = ui.ctx();
        let text_focus = ctx.memory(|m| m.focused()).map(|id| egui::text_edit::TextEditState::load(ctx, id).is_some()).unwrap_or(false);
        if text_focus {
            return;
        }
        let (esc, next, prev, f3, sf3) = ctx.input_mut(|i| {
            // Shifted variants first: `consume_key` lets extra Shift through.
            let prev = i.consume_key(Modifiers::SHIFT, Key::N) || i.consume_key(Modifiers::SHIFT, Key::F7);
            let next = i.consume_key(Modifiers::NONE, Key::N) || i.consume_key(Modifiers::NONE, Key::F7);
            let sf3 = i.consume_key(Modifiers::SHIFT, Key::F3);
            let f3 = i.consume_key(Modifiers::NONE, Key::F3);
            (i.consume_key(Modifiers::NONE, Key::Escape), next, prev, f3, sf3)
        });
        if esc {
            if self.find.open {
                self.find.open = false;
            } else {
                self.highlight = None;
            }
        }
        if self.is_diff() && (next || prev) {
            self.go_to_hunk(next);
        }
        if f3 || sf3 {
            self.go_to_match(if sf3 { -1 } else { 1 });
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let tokens = app.tokens.clone();
        egui::Panel::top(("xml-toolbar", &self.doc)).frame(egui::Frame::new().fill(tokens.bg1).inner_margin(egui::Margin::symmetric(12, 6))).show(ui, |ui| {
            ui.horizontal(|ui| {
                match self.doc.clone() {
                    XmlDoc::Single(s) => {
                        ui.label(RichText::new(s.label(app)).strong().size(15.0)).on_hover_text(s.detail());
                        ui.label(RichText::new("XML").weak());
                    }
                    XmlDoc::Diff { left, right } => {
                        ui.label(RichText::new(left.label(app)).strong().color(tokens.incident)).on_hover_text(left.detail());
                        ui.label(RichText::new("→").weak());
                        ui.label(RichText::new(right.label(app)).strong().color(tokens.completed)).on_hover_text(right.detail());
                        if let Some(d) = &self.data {
                            if d.added == 0 && d.deleted == 0 {
                                ui.label(RichText::new(crate::t!("xml-identical")).weak());
                            } else {
                                ui.label(RichText::new(format!("+{}", d.added)).color(tokens.completed).monospace());
                                ui.label(RichText::new(format!("−{}", d.deleted)).color(tokens.incident).monospace());
                                if ui.checkbox(&mut self.whole_file, crate::t!("xml-whole-file")).on_hover_text(crate::t!("xml-whole-file-tip")).changed() {
                                    self.display_dirty = true;
                                }
                                if ui.small_button("↑").on_hover_text(crate::t!("xml-prev-change-tip")).clicked() {
                                    self.go_to_hunk(false);
                                }
                                if ui.small_button("↓").on_hover_text(crate::t!("xml-next-change-tip")).clicked() {
                                    self.go_to_hunk(true);
                                }
                            }
                        }
                    }
                }
                if let Some(h) = self.highlight.clone() {
                    ui.add_space(6.0);
                    let chip = egui::Button::new(RichText::new(format!("◆ {}   ✕", crate::util::ellipsize(&h.id, 40))).color(tokens.fg0).size(12.0)).fill(tokens.accent.gamma_multiply(0.25)).corner_radius(9.0);
                    if ui.add(chip).on_hover_text(crate::t!("xml-clear-highlight-tip")).clicked() {
                        self.highlight = None;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(format!("🔍 {}", crate::t!("xml-find"))).on_hover_text("Ctrl+F").clicked() {
                        self.open_find();
                    }
                    if ui.button(crate::t!("xml-show-diagram")).on_hover_text(crate::t!("xml-show-diagram-tip")).clicked() {
                        let id = self.highlight.as_ref().map(|h| h.id.clone());
                        reveal_in_diagram(app, self.doc.diagram_nav(), id);
                    }
                    if let Some(d) = &self.data {
                        let side = (self.new_slot.source.clone(), d.new.model.clone());
                        let el = self.highlight.as_ref().filter(|h| h.side == Side::New).map(|h| h.id.clone());
                        external_buttons(ui, app, &side, el);
                    }
                });
            });
        });
    }

    fn find_bar(&mut self, ui: &mut egui::Ui, tokens: &crate::theme::Tokens) {
        egui::Panel::top(("xml-find", &self.doc)).frame(egui::Frame::new().fill(tokens.bg1).inner_margin(egui::Margin::symmetric(12, 4))).show(ui, |ui| {
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut self.find.query).hint_text(crate::t!("xml-find-hint")).desired_width(320.0).id(egui::Id::unique(("xml-find-input", &self.doc))));
                if self.find.want_focus {
                    r.request_focus();
                    self.find.want_focus = false;
                }
                self.recompute_find();
                let (enter, shift, esc) = ui.input(|i| (i.key_pressed(Key::Enter), i.modifiers.shift, i.key_pressed(Key::Escape)));
                if r.changed() && !self.find.matches.is_empty() {
                    self.find.cursor = 0;
                    self.go_to_match(0);
                }
                if r.lost_focus() && enter {
                    self.go_to_match(if shift { -1 } else { 1 });
                    r.request_focus();
                }
                if r.lost_focus() && esc {
                    self.find.open = false;
                }
                let n = self.find.matches.len();
                let label = if self.find.query.is_empty() { String::new() } else if n == 0 { crate::t!("find-no-matches") } else { format!("{}/{}", self.find.cursor + 1, n) };
                ui.label(RichText::new(label).weak().monospace());
                if ui.small_button("↑").on_hover_text("Shift+Enter").clicked() {
                    self.go_to_match(-1);
                }
                if ui.small_button("↓").on_hover_text("Enter").clicked() {
                    self.go_to_match(1);
                }
                if ui.small_button("✕").on_hover_text("Esc").clicked() {
                    self.find.open = false;
                }
            });
        });
    }

    fn text_area(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let tokens = app.tokens.clone();
        let font = FontId::monospace(12.5);
        let gutter_font = FontId::monospace(11.0);
        let (row_h, char_w) = ui.fonts_mut(|f| (f.row_height(&font) + 3.0, f.glyph_width(&font, 'M')));
        let diff = self.is_diff();
        let Some(d) = &self.data else { return };
        let digits = d.rows.len().max(10).to_string().len() as f32;
        let num_w = digits * char_w * 0.9 + 10.0;
        let gutter_w = if diff { num_w * 2.0 + 16.0 } else { num_w + 6.0 };
        let text_x = gutter_w + 8.0;
        let content_w = text_x + d.max_chars.min(MAX_LINE_CHARS) as f32 * char_w + 40.0;
        let palette = Palette::new(&tokens);

        let mut scroll = egui::ScrollArea::both().id_salt(("xml-scroll", &self.doc)).auto_shrink([false, false]);
        if let Some((di, col)) = self.scroll_to.take() {
            let view_h = ui.available_height();
            let view_w = ui.available_width();
            scroll = scroll.vertical_scroll_offset((di as f32 * row_h - view_h * 0.3).max(0.0));
            let x = col as f32 * char_w;
            scroll = scroll.horizontal_scroll_offset(if x + text_x > view_w * 0.8 { (x - view_w * 0.3).max(0.0) } else { 0.0 });
        }
        let display = std::mem::take(&mut self.display);
        let mut clicked_fold: Option<usize> = None;
        let mut action: Option<RowAction> = None;
        // show_rows sizes rows with the spacing of the ui it is given: make it zero there.
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        scroll.show_rows(ui, row_h, display.len(), |ui, range| {
            let width = content_w.max(ui.available_width());
            ui.set_min_width(width);
            let clip = ui.clip_rect();
            let d = self.data.as_ref().expect("loaded");
            for di in range {
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, row_h), Sense::click());
                match display[di] {
                    DRow::Fold { from, to } => {
                        ui.painter().rect_filled(rect, 0.0, tokens.bg1);
                        let label = crate::t!("xml-fold", n = to - from);
                        let pos = egui::pos2(clip.min.x + text_x, rect.center().y);
                        let color = if resp.hovered() { tokens.accent } else { tokens.fg2 };
                        ui.painter().text(pos, egui::Align2::LEFT_CENTER, format!("⋯  {label}"), gutter_font.clone(), color);
                        if resp.clicked() {
                            clicked_fold = Some(from);
                        }
                        resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    }
                    DRow::Row(r) => {
                        let row = d.rows[r];
                        let (side, line) = d.row_line(r);
                        let st = d.side(side);
                        // Row background: diff colour, then the element highlight band.
                        let bg = match row.tag {
                            LineTag::Insert => Some(palette.ins_bg),
                            LineTag::Delete => Some(palette.del_bg),
                            LineTag::Equal => None,
                        };
                        if let Some(c) = bg {
                            ui.painter().rect_filled(rect, 0.0, c);
                        }
                        let hl = self.highlight.as_ref().and_then(|h| {
                            let hl_line = d.line_on(r, h.side)?;
                            let s = d.side(h.side);
                            let lr = s.index.range(&s.model.source, hl_line);
                            let a = h.range.start.max(lr.start);
                            let b = h.range.end.min(lr.end.max(lr.start));
                            (a < b || (h.range.start <= lr.start && h.range.end > lr.start)).then(|| a.saturating_sub(lr.start)..b.saturating_sub(lr.start))
                        });
                        if hl.is_some() && row.tag == LineTag::Equal {
                            ui.painter().rect_filled(rect, 0.0, palette.highlight_band);
                        }
                        // Text.
                        let text = st.line(line);
                        let text = if text.len() > MAX_LINE_CHARS { &text[..text.char_indices().nth(MAX_LINE_CHARS).map(|(i, _)| i).unwrap_or(text.len())] } else { text };
                        let state = st.states.get(line).copied().unwrap_or_default();
                        let mut marks: Vec<(Range<usize>, Color32)> = vec![];
                        if let Some(h) = &hl {
                            marks.push((h.clone(), palette.highlight));
                        }
                        if row.tag != LineTag::Equal {
                            if let Some(ranges) = self.inline_cache.get(&r) {
                                marks.extend(ranges.iter().map(|x| (x.clone(), if row.tag == LineTag::Insert { palette.ins_em } else { palette.del_em })));
                            }
                        }
                        if !self.find.matches.is_empty() {
                            let start = self.find.matches.partition_point(|(mr, _)| *mr < r);
                            for (k, (mr, m)) in self.find.matches.iter().enumerate().skip(start) {
                                if *mr != r {
                                    break;
                                }
                                marks.push((m.clone(), if k == self.find.cursor { palette.current_match } else { palette.find }));
                            }
                        }
                        let job = layout_line(text, state, &palette, &font, &marks);
                        let text_rect = egui::Rect::from_min_size(egui::pos2(rect.min.x + text_x, rect.min.y + 1.5), Vec2::new(width - text_x, row_h - 1.5));
                        let label = ui.scope_builder(egui::UiBuilder::new().max_rect(text_rect).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| ui.add(egui::Label::new(job).extend().selectable(true))).inner;
                        // Sticky gutter over the text when scrolled sideways.
                        let g = egui::Rect::from_min_size(egui::pos2(clip.min.x, rect.min.y), Vec2::new(gutter_w, row_h));
                        ui.painter().rect_filled(g, 0.0, bg.map(|c| palette.gutter_bg.blend(c)).unwrap_or(palette.gutter_bg));
                        let num_color = if hl.is_some() { tokens.accent } else { tokens.fg2 };
                        let fmt = |n: Option<usize>| n.map(|n| (n + 1).to_string()).unwrap_or_default();
                        if diff {
                            ui.painter().text(egui::pos2(g.min.x + num_w - 4.0, g.center().y), egui::Align2::RIGHT_CENTER, fmt(row.old_line), gutter_font.clone(), num_color);
                            ui.painter().text(egui::pos2(g.min.x + num_w * 2.0 - 4.0, g.center().y), egui::Align2::RIGHT_CENTER, fmt(row.new_line), gutter_font.clone(), num_color);
                            let (sign, c) = match row.tag {
                                LineTag::Insert => ("+", tokens.completed),
                                LineTag::Delete => ("−", tokens.incident),
                                LineTag::Equal => ("", tokens.fg2),
                            };
                            ui.painter().text(egui::pos2(g.min.x + num_w * 2.0 + 8.0, g.center().y), egui::Align2::CENTER_CENTER, sign, font.clone(), c);
                        } else {
                            ui.painter().text(egui::pos2(g.min.x + num_w - 4.0, g.center().y), egui::Align2::RIGHT_CENTER, fmt(Some(line)), gutter_font.clone(), num_color);
                        }
                        ui.painter().vline(g.max.x, g.y_range(), Stroke::new(1.0, tokens.bg3));
                        if hl.is_some() {
                            ui.painter().vline(g.max.x + 1.5, g.y_range(), Stroke::new(3.0, tokens.accent));
                        }
                        resp.context_menu(|ui| row_menu(ui, d, r, &mut action));
                        label.context_menu(|ui| row_menu(ui, d, r, &mut action));
                    }
                }
            }
        });
        self.display = display;
        if let Some(f) = clicked_fold {
            self.expanded.insert(f);
            self.display_dirty = true;
        }
        self.fill_inline_cache();
        if let Some(a) = action {
            self.run_row_action(a, app);
        }
    }

    /// Intra-line emphasis for every changed row that has a partner (computed once per row).
    fn fill_inline_cache(&mut self) {
        let Some(d) = &self.data else { return };
        if d.old.is_none() || self.inline_done {
            return;
        }
        for r in 0..d.rows.len() {
            if let Some(p) = d.pairs[r] {
                if d.rows[r].tag == LineTag::Delete {
                    let (a, b) = bpmn::diff::inline_ranges(d.row_text(r), d.row_text(p));
                    self.inline_cache.insert(r, super::diff::tidy_emphasis(d.row_text(r), a));
                    self.inline_cache.insert(p, super::diff::tidy_emphasis(d.row_text(p), b));
                }
            }
        }
        self.inline_done = true;
    }

    fn run_row_action(&mut self, a: RowAction, app: &mut App) {
        let Some(d) = &self.data else { return };
        match a {
            RowAction::CopyLine(r) => {
                app.copy_with_toast(d.row_text(r).trim().to_string());
            }
            RowAction::ShowInDiagram(r) => {
                let (side, line) = d.row_line(r);
                let st = d.side(side);
                let id = element_at_line(&st.model, &st.index, line);
                reveal_in_diagram(app, self.doc.diagram_nav(), id);
            }
            RowAction::HighlightElement(r) => {
                let (side, line) = d.row_line(r);
                let st = d.side(side);
                if let Some(id) = element_at_line(&st.model, &st.index, line) {
                    self.focus = Some(id);
                }
            }
            RowAction::CopyLocation(r) => {
                let (side, line) = d.row_line(r);
                let st = d.side(side);
                let source = match side {
                    Side::New => self.new_slot.source.clone(),
                    Side::Old => self.old_slot.as_ref().map(|s| s.source.clone()).unwrap_or_else(|| self.new_slot.source.clone()),
                };
                let text = st.line(line);
                let col = text.chars().take_while(|c| c.is_whitespace()).count() + 1;
                match path_on_disk(app, &source, &st.model) {
                    Ok(p) => app.copy_with_toast(crate::platform::files::location(&p, line + 1, col)),
                    Err(e) => {
                        app.toasts.error(crate::t!("xml-export-failed"), Some(e));
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum RowAction {
    CopyLine(usize),
    CopyLocation(usize),
    ShowInDiagram(usize),
    HighlightElement(usize),
}

fn row_menu(ui: &mut egui::Ui, d: &Doc, r: usize, action: &mut Option<RowAction>) {
    let (side, line) = d.row_line(r);
    let st = d.side(side);
    let el = element_at_line(&st.model, &st.index, line);
    if let Some(id) = &el {
        ui.label(RichText::new(id).monospace().small().weak());
        if ui.button(crate::t!("xml-ctx-show-in-diagram")).clicked() {
            *action = Some(RowAction::ShowInDiagram(r));
            ui.close();
        }
        if ui.button(crate::t!("xml-ctx-highlight-element")).clicked() {
            *action = Some(RowAction::HighlightElement(r));
            ui.close();
        }
        ui.separator();
    }
    if ui.button(crate::t!("xml-ctx-copy-line")).clicked() {
        *action = Some(RowAction::CopyLine(r));
        ui.close();
    }
    if ui.button(crate::t!("ctx-copy-location")).clicked() {
        *action = Some(RowAction::CopyLocation(r));
        ui.close();
    }
}

/// The innermost element or flow whose XML covers `line`; in the diagram-interchange part, the
/// element a `bpmnElement="…"` attribute on or just above the line points at.
pub fn element_at_line(m: &Model, index: &LineIndex, line: usize) -> Option<String> {
    let text = &m.source;
    let lr = index.range(text, line);
    let first = lr.start + text[lr.clone()].len() - text[lr.clone()].trim_start().len();
    let spans = m.elements.iter().map(|e| (&e.xml_range, e.id.as_str())).chain(m.flows.iter().map(|f| (&f.xml_range, f.id.as_str())));
    let best = spans.filter(|(r, _)| r.start <= first && first < r.end.max(r.start + 1)).min_by_key(|(r, _)| r.end - r.start);
    if let Some((_, id)) = best {
        return Some(id.to_string());
    }
    for l in (line.saturating_sub(6)..=line).rev() {
        let t = index.line(text, l);
        if let Some(p) = t.find("bpmnElement=\"") {
            let rest = &t[p + 13..];
            let id = rest.split('"').next().unwrap_or("");
            if m.lookup(id).is_some() {
                return Some(id.to_string());
            }
        }
        if l < line && (t.contains("</bpmndi:BPMNShape") || t.contains("</bpmndi:BPMNEdge")) {
            break;
        }
    }
    None
}

fn build_doc(new: Arc<Model>, old: Option<Arc<Model>>) -> Doc {
    let new = SideText::new(new);
    let old = old.map(SideText::new);
    let rows: Vec<TextRow> = match &old {
        None => (0..new.index.lines()).map(|i| TextRow { tag: LineTag::Equal, old_line: None, new_line: Some(i) }).collect(),
        Some(o) => bpmn::diff::text_rows(&o.model.source, &new.model.source),
    };
    // Pair the k-th deleted row of a changed block with its k-th inserted row.
    let mut pairs = vec![None; rows.len()];
    let mut r = 0;
    while r < rows.len() {
        if rows[r].tag == LineTag::Equal {
            r += 1;
            continue;
        }
        let start = r;
        while r < rows.len() && rows[r].tag != LineTag::Equal {
            r += 1;
        }
        let dels: Vec<usize> = (start..r).filter(|&i| rows[i].tag == LineTag::Delete).collect();
        let ins: Vec<usize> = (start..r).filter(|&i| rows[i].tag == LineTag::Insert).collect();
        for (a, b) in dels.iter().zip(ins.iter()) {
            pairs[*a] = Some(*b);
            pairs[*b] = Some(*a);
        }
    }
    let added = rows.iter().filter(|r| r.tag == LineTag::Insert).count();
    let deleted = rows.iter().filter(|r| r.tag == LineTag::Delete).count();
    let mut doc = Doc { new, old, rows, pairs, max_chars: 0, added, deleted };
    doc.max_chars = (0..doc.rows.len()).map(|r| doc.row_text(r).chars().count()).max().unwrap_or(0);
    doc
}

/// Display rows with long unchanged runs folded, keeping `CONTEXT` lines around each change.
fn fold_rows(rows: &[TextRow], expanded: &HashSet<usize>) -> Vec<DRow> {
    let mut out = vec![];
    let mut r = 0;
    while r < rows.len() {
        if rows[r].tag != LineTag::Equal {
            out.push(DRow::Row(r));
            r += 1;
            continue;
        }
        let start = r;
        while r < rows.len() && rows[r].tag == LineTag::Equal {
            r += 1;
        }
        let end = r;
        let keep_head = if start == 0 { 0 } else { CONTEXT };
        let keep_tail = if end == rows.len() { 0 } else { CONTEXT };
        let hide_from = start + keep_head;
        let hide_to = end.saturating_sub(keep_tail);
        if hide_to > hide_from + 1 && !expanded.contains(&hide_from) {
            out.extend((start..hide_from).map(DRow::Row));
            out.push(DRow::Fold { from: hide_from, to: hide_to });
            out.extend((hide_to..end).map(DRow::Row));
        } else {
            out.extend((start..end).map(DRow::Row));
        }
    }
    out
}

struct Palette {
    text: Color32,
    punct: Color32,
    tag: Color32,
    prefix: Color32,
    attr: Color32,
    value: Color32,
    comment: Color32,
    entity: Color32,
    ins_bg: Color32,
    del_bg: Color32,
    ins_em: Color32,
    del_em: Color32,
    highlight: Color32,
    highlight_band: Color32,
    find: Color32,
    current_match: Color32,
    gutter_bg: Color32,
}

impl Palette {
    fn new(t: &crate::theme::Tokens) -> Self {
        let dark = t.dark;
        Self {
            text: t.fg0,
            punct: t.fg1,
            tag: if dark { Color32::from_rgb(0x7D, 0xB4, 0xFF) } else { Color32::from_rgb(0x1F, 0x5F, 0xBF) },
            prefix: t.fg2,
            attr: if dark { Color32::from_rgb(0x5C, 0xD0, 0xC0) } else { Color32::from_rgb(0x0B, 0x7A, 0x6E) },
            value: if dark { Color32::from_rgb(0xE8, 0xB8, 0x7A) } else { Color32::from_rgb(0x9A, 0x50, 0x00) },
            comment: t.fg2,
            entity: t.accent,
            ins_bg: t.completed.gamma_multiply(if dark { 0.16 } else { 0.13 }),
            del_bg: t.incident.gamma_multiply(if dark { 0.16 } else { 0.11 }),
            ins_em: t.completed.gamma_multiply(if dark { 0.38 } else { 0.32 }),
            del_em: t.incident.gamma_multiply(if dark { 0.38 } else { 0.28 }),
            highlight: t.accent.gamma_multiply(if dark { 0.30 } else { 0.20 }),
            highlight_band: t.accent.gamma_multiply(if dark { 0.10 } else { 0.07 }),
            find: t.matched.gamma_multiply(0.45),
            current_match: if dark { Color32::from_rgb(0xFF, 0xA5, 0x2B) } else { Color32::from_rgb(0xE8, 0x8A, 0x00) },
            gutter_bg: t.bg1,
        }
    }
    fn color(&self, t: Tok) -> Color32 {
        match t {
            Tok::Text | Tok::CData => self.text,
            Tok::Punct => self.punct,
            Tok::Tag => self.tag,
            Tok::Prefix | Tok::Decl => self.prefix,
            Tok::Attr => self.attr,
            Tok::Value => self.value,
            Tok::Comment => self.comment,
            Tok::Entity => self.entity,
        }
    }
}

/// One line as a layout job: token colours plus background marks (later marks win).
fn layout_line(text: &str, state: State, p: &Palette, font: &FontId, marks: &[(Range<usize>, Color32)]) -> LayoutJob {
    let toks = xmltext::tokens(text, state);
    let mut cuts: Vec<usize> = vec![0, text.len()];
    for (r, _) in &toks {
        cuts.push(r.start);
        cuts.push(r.end);
    }
    for (r, _) in marks {
        cuts.push(r.start.min(text.len()));
        cuts.push(r.end.min(text.len()));
    }
    cuts.sort_unstable();
    cuts.dedup();
    cuts.retain(|&c| text.is_char_boundary(c));
    let mut job = LayoutJob::default();
    let mut ti = 0;
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a >= b {
            continue;
        }
        while ti < toks.len() && toks[ti].0.end <= a {
            ti += 1;
        }
        let kind = toks.get(ti).filter(|(r, _)| r.start <= a).map(|(_, k)| *k).unwrap_or(Tok::Text);
        let bg = marks.iter().rev().find(|(r, _)| r.start <= a && b <= r.end).map(|(_, c)| *c).unwrap_or(Color32::TRANSPARENT);
        let mut fmt = TextFormat { font_id: font.clone(), color: p.color(kind), background: bg, ..Default::default() };
        if kind == Tok::Comment {
            fmt.italics = true;
        }
        job.append(&text[a..b], 0.0, fmt);
    }
    job.wrap.max_width = f32::INFINITY;
    job
}

// ---------------------------------------------------------------------------------------------
// Opening XML from elsewhere

/// Switch to (or open) the diagram of `nav` and select `element` there.
pub fn reveal_in_diagram(app: &mut App, nav: Nav, element: Option<String>) {
    app.navigate(nav, NavMode::Reuse);
    if let Some(id) = element {
        app.place_request = Some(crate::commands::PlaceRequest::Reveal(id));
    }
}

/// `Open with…` / editor / copy-location buttons for a document side (toolbar of the XML view).
fn external_buttons(ui: &mut egui::Ui, app: &mut App, side: &(DocSource, Arc<Model>), element: Option<String>) {
    if !app.settings.xml_editor.trim().is_empty() && ui.button(crate::t!("xml-open-editor")).on_hover_text(app.settings.xml_editor.clone()).clicked() {
        xml_external(app, side, element.as_deref(), External::Editor);
    }
    if crate::platform::files::open_with_supported() && ui.button(crate::t!("xml-open-with")).on_hover_text(crate::t!("xml-open-with-tip")).clicked() {
        xml_external(app, side, element.as_deref(), External::OpenWith);
    }
    if ui.button(crate::t!("ctx-copy-location")).on_hover_text(crate::t!("ctx-copy-location-tip")).clicked() {
        xml_external(app, side, element.as_deref(), External::CopyLocation);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum External {
    OpenWith,
    Editor,
    CopyLocation,
}

/// Put `side`'s document on disk if needed, then copy `path:line:col` of `element` (or the
/// document start), open the system chooser, or start the configured editor there.
pub fn xml_external(app: &mut App, side: &(DocSource, Arc<Model>), element: Option<&str>, what: External) {
    let (source, model) = side;
    let path = match path_on_disk(app, source, model) {
        Ok(p) => p,
        Err(e) => {
            app.toasts.error(crate::t!("xml-export-failed"), Some(e));
            return;
        }
    };
    let offset = element
        .and_then(|id| match model.lookup(id) {
            Some(bpmn::Ref::Element(i)) => Some(model.elements[i].xml_range.start),
            Some(bpmn::Ref::Flow(i)) => Some(model.flows[i].xml_range.start),
            None => None,
        })
        .unwrap_or(0);
    let (line, col) = LineIndex::new(&model.source).line_col(&model.source, offset);
    match what {
        External::CopyLocation => app.copy_with_toast(crate::platform::files::location(&path, line, col)),
        External::Editor => {
            if let Err(e) = crate::platform::files::run_editor(&app.settings.xml_editor, &path, line, col) {
                app.toasts.error(crate::t!("xml-editor-failed"), Some(e));
            }
        }
        External::OpenWith => app.spawn_bg(move || {
            crate::platform::files::open_with(&path).err().map(|e| -> crate::widgets::AppAction { Box::new(move |app: &mut App| {
                app.toasts.error(crate::t!("xml-open-with-failed"), Some(e));
            }) })
        }),
    }
}

/// The XML document of a diagram place and the id of its selected element (for Ctrl+U).
pub fn xml_target(place: &crate::state::Place, app: &App) -> Option<(XmlDoc, Option<String>)> {
    use crate::state::Place;
    let id_in = |m: Option<&Model>, sel: Option<bpmn::Ref>| -> Option<String> {
        let m = m?;
        Some(match sel? {
            bpmn::Ref::Element(i) => m.elements.get(i)?.id.clone(),
            bpmn::Ref::Flow(i) => m.flows.get(i)?.id.clone(),
        })
    };
    let cached = |def: &str| app.cache.diagrams.get(def).and_then(|l| l.data.clone());
    match place {
        Place::Definition(p) => Some((XmlDoc::Single(DocSource::Definition(p.def_id.clone())), id_in(cached(&p.def_id).as_deref(), p.view.selection))),
        Place::Instance(p) => {
            let def = p.definition_id_pub()?;
            let id = id_in(cached(&def).as_deref(), p.view.selection);
            Some((XmlDoc::Single(DocSource::Definition(def)), id))
        }
        Place::File(p) => Some((XmlDoc::Single(DocSource::File(p.path.clone())), id_in(p.model.as_deref(), p.view.selection))),
        Place::Diff(p) => p.xml_target(),
        _ => None,
    }
}

/// Context-menu items for viewing an element's XML, shared by every diagram.
pub fn context_items(ui: &mut egui::Ui, element: Option<&str>, editor: bool, actions: &mut Vec<crate::inspector::InspectorAction>) {
    use crate::inspector::InspectorAction as A;
    let id = element.map(String::from);
    if ui.button(format!("{:<22} Ctrl+U", crate::t!("ctx-view-xml"))).clicked() {
        actions.push(A::ViewXml(id.clone()));
        ui.close();
    }
    if crate::platform::files::open_with_supported() && ui.button(crate::t!("ctx-open-xml-with")).on_hover_text(crate::t!("xml-open-with-tip")).clicked() {
        actions.push(A::XmlExternal(id.clone(), External::OpenWith));
        ui.close();
    }
    if editor && ui.button(crate::t!("ctx-open-xml-editor")).clicked() {
        actions.push(A::XmlExternal(id.clone(), External::Editor));
        ui.close();
    }
    if ui.button(crate::t!("ctx-copy-location")).on_hover_text(crate::t!("ctx-copy-location-tip")).clicked() {
        actions.push(A::XmlExternal(id, External::CopyLocation));
        ui.close();
    }
}

/// Handle the XML actions of a diagram place. `sides` lists the documents to look the element
/// up in, preferred first (a diff passes after, then before). Returns true when handled.
pub fn handle_action(app: &mut App, doc: XmlDoc, sides: &[(DocSource, Arc<Model>)], a: &crate::inspector::InspectorAction) -> bool {
    use crate::inspector::InspectorAction as A;
    match a {
        A::ViewXml(id) => {
            app.navigate(Nav::Xml { doc, element: id.clone() }, NavMode::Reuse);
            true
        }
        A::XmlExternal(id, what) => {
            let side = id.as_deref().and_then(|id| sides.iter().find(|(_, m)| m.lookup(id).is_some())).or(sides.first()).cloned();
            match side {
                Some(s) => xml_external(app, &s, id.as_deref(), *what),
                None => {
                    app.toasts.push(ToastKind::Warning, crate::t!("xml-not-loaded"));
                }
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(tags: &str) -> Vec<TextRow> {
        let (mut o, mut n) = (0, 0);
        tags.chars()
            .map(|c| {
                let tag = match c {
                    '=' => LineTag::Equal,
                    '-' => LineTag::Delete,
                    _ => LineTag::Insert,
                };
                let r = TextRow { tag, old_line: (tag != LineTag::Insert).then_some(o), new_line: (tag != LineTag::Delete).then_some(n) };
                if tag != LineTag::Insert {
                    o += 1;
                }
                if tag != LineTag::Delete {
                    n += 1;
                }
                r
            })
            .collect()
    }

    #[test]
    fn unchanged_runs_fold_with_context_and_unfold_on_request() {
        let r = rows("==========-+==========");
        let d = fold_rows(&r, &HashSet::new());
        // Leading run: no head context; 3 lines of context before the change.
        assert_eq!(d[0], DRow::Fold { from: 0, to: 7 });
        assert_eq!(d[1..4], [DRow::Row(7), DRow::Row(8), DRow::Row(9)]);
        assert_eq!(d[4], DRow::Row(10));
        assert_eq!(d[5], DRow::Row(11));
        assert_eq!(d[6..9], [DRow::Row(12), DRow::Row(13), DRow::Row(14)]);
        assert_eq!(*d.last().unwrap(), DRow::Fold { from: 15, to: 22 });
        let open: HashSet<usize> = [0].into();
        let d = fold_rows(&r, &open);
        assert_eq!(d[0], DRow::Row(0), "an expanded fold shows its rows");
        // Short runs between changes are never folded.
        let d = fold_rows(&rows("-=======+"), &HashSet::new());
        assert_eq!(d.len(), 9);
    }

    #[test]
    fn element_at_line_finds_the_innermost_element_and_di_shapes() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../bpmn/tests/fixtures/order.bpmn")).unwrap();
        let m = bpmn::parse(&src).unwrap();
        let ix = LineIndex::new(&m.source);
        let e = m.element_by_id("Task_Approve").unwrap();
        let start_line = ix.line_of(e.xml_range.start);
        assert_eq!(element_at_line(&m, &ix, start_line).as_deref(), Some("Task_Approve"));
        let last_line = ix.line_of(e.xml_range.end - 1);
        assert_eq!(element_at_line(&m, &ix, last_line).as_deref(), Some("Task_Approve"));
        let di = (0..ix.lines()).find(|&l| ix.line(&m.source, l).contains("bpmnElement=\"Task_Approve\"")).unwrap();
        assert_eq!(element_at_line(&m, &ix, di).as_deref(), Some("Task_Approve"));
    }

    #[test]
    fn layout_marks_split_tokens_and_keep_all_text() {
        let p = Palette::new(&crate::theme::Tokens::dark());
        let line = r#"<bpmn:task id="Task_1" name="Do it">"#;
        let marks = vec![(10..20, Color32::RED)];
        let job = layout_line(line, State::Text, &p, &FontId::monospace(12.0), &marks);
        assert_eq!(job.text, line);
        let red: String = job.sections.iter().filter(|s| s.format.background == Color32::RED).map(|s| &job.text[s.byte_range.start.0..s.byte_range.end.0]).collect();
        assert_eq!(red, &line[10..20]);
    }
}
