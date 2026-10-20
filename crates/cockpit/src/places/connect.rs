//! Connection screen: a list of saved connections on the left (multi-select, drag to the bin to
//! delete), the connection form on the right, centred as one group.

use crate::app::App;
use crate::t;
use crate::settings::EngineProfile;
use crate::widgets::Confirm;
use egui::{Color32, Key, Modifiers, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};
use std::collections::BTreeSet;
use std::sync::Arc;

/// Multi-selection over list indices: click selects one, Ctrl toggles, Shift extends from the
/// anchor, Alt removes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListSelection {
    pub items: BTreeSet<usize>,
    /// The item shown in the form (last plainly clicked).
    pub primary: Option<usize>,
    /// Start of a Shift range.
    pub anchor: Option<usize>,
}

impl ListSelection {
    pub fn click(&mut self, idx: usize, mods: Modifiers) {
        if mods.alt {
            self.items.remove(&idx);
            if self.primary == Some(idx) {
                self.primary = self.items.iter().next().copied();
            }
        } else if mods.shift {
            let a = self.anchor.unwrap_or(idx);
            let (lo, hi) = (a.min(idx), a.max(idx));
            if !mods.ctrl {
                self.items.clear();
            }
            self.items.extend(lo..=hi);
            self.primary = Some(idx);
        } else if mods.ctrl {
            if !self.items.remove(&idx) {
                self.items.insert(idx);
                self.primary = Some(idx);
            } else if self.primary == Some(idx) {
                self.primary = self.items.iter().next().copied();
            }
            self.anchor = Some(idx);
        } else {
            self.items.clear();
            self.items.insert(idx);
            self.primary = Some(idx);
            self.anchor = Some(idx);
        }
    }

    pub fn select_only(&mut self, idx: usize) {
        self.click(idx, Modifiers::NONE);
    }

    /// Keep the selection valid after items `removed` (sorted, unique) were deleted.
    pub fn after_removal(&mut self, removed: &[usize], remaining: usize) {
        let shift = |i: usize| i - removed.iter().filter(|&&r| r < i).count();
        self.items = self.items.iter().filter(|i| !removed.contains(i)).map(|&i| shift(i)).collect();
        self.primary = self.primary.filter(|p| !removed.contains(p)).map(shift).or_else(|| self.items.iter().next().copied());
        self.anchor = self.anchor.filter(|a| !removed.contains(a)).map(shift);
        if self.primary.is_none() && remaining > 0 {
            let idx = removed.first().copied().unwrap_or(0).min(remaining - 1);
            self.select_only(idx);
        }
    }

    pub fn clamp(&mut self, len: usize) {
        self.items.retain(|&i| i < len);
        if self.primary.map(|p| p >= len).unwrap_or(false) {
            self.primary = None;
        }
        if self.primary.is_none() && len > 0 {
            self.select_only(self.items.iter().next().copied().unwrap_or(0).min(len - 1));
        }
    }
}

#[derive(Default)]
pub struct ConnectScreen {
    pub selection: ListSelection,
    pub error: Option<String>,
    pub connecting: bool,
    pub show_password: bool,
    initialised: bool,
    /// Name field of a freshly added connection requests focus once.
    focus_name: bool,
}

/// Payload of a list drag: the selected indices.
#[derive(Clone, Debug)]
struct DragConnections(Vec<usize>);

const LIST_W: f32 = 250.0;
const FORM_W: f32 = 470.0;
const GAP: f32 = 16.0;
const HEIGHT: f32 = 440.0;
const BIN_H: f32 = 72.0;

pub fn ui(ui: &mut egui::Ui, app: &mut App) {
    let ctx = ui.ctx().clone();
    if !app.connect.initialised {
        app.connect.initialised = true;
        let n = app.settings.engines.len();
        if n > 0 {
            app.connect.selection.select_only(app.settings.last_engine.min(n - 1));
        }
    }
    app.connect.selection.clamp(app.settings.engines.len());

    egui::CentralPanel::default().frame(egui::Frame::new().fill(app.tokens.bg0)).show(ui, |ui| {
        let avail = ui.available_rect_before_wrap();
        let total_w = (LIST_W + GAP + FORM_W).min(avail.width() - 32.0);
        let form_w = (total_w - GAP - LIST_W).max(300.0);
        let origin = Pos2::new(avail.center().x - total_w / 2.0, (avail.center().y - HEIGHT / 2.0 - 30.0).max(avail.min.y + 16.0));
        let list_rect = Rect::from_min_size(origin, Vec2::new(LIST_W, HEIGHT - BIN_H - GAP));
        let bin_rect = Rect::from_min_size(Pos2::new(origin.x, list_rect.max.y + GAP), Vec2::new(LIST_W, BIN_H));
        let form_rect = Rect::from_min_size(Pos2::new(origin.x + LIST_W + GAP, origin.y), Vec2::new(form_w, HEIGHT));

        let mut list_ui = crate::widgets::split::child_in(ui, list_rect);
        connection_list(&mut list_ui, app, list_rect);
        let mut bin_ui = crate::widgets::split::child_in(ui, bin_rect);
        trash_bin(&mut bin_ui, app, bin_rect);
        let mut form_ui = crate::widgets::split::child_in(ui, form_rect);
        form_card(&mut form_ui, app, form_rect);

        // Enter connects when no text field is focused (text fields are single line: Enter is free).
        if !app.connect.connecting && ui.input(|i| i.key_pressed(Key::Enter)) && app.connect.selection.primary.is_some() {
            start_connect(app);
        }
        if ui.input(|i| i.key_pressed(Key::Delete)) && !text_focused(&ctx) {
            request_delete(app, app.connect.selection.items.iter().copied().collect());
        }
    });
}

fn text_focused(ctx: &egui::Context) -> bool {
    ctx.memory(|m| m.focused()).map(|id| egui::text_edit::TextEditState::load(ctx, id).is_some()).unwrap_or(false)
}

fn card<R>(ui: &mut Ui, rect: Rect, tokens: &crate::theme::Tokens, stroke: Stroke, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.painter().rect(rect, 8.0, tokens.bg2, stroke, StrokeKind::Inside);
    let mut inner = crate::widgets::split::child_in(ui, rect.shrink(1.0));
    egui::Frame::new().inner_margin(16.0).show(&mut inner, |ui| {
        ui.set_width(rect.width() - 34.0);
        add(ui)
    })
    .inner
}

fn connection_list(ui: &mut Ui, app: &mut App, rect: Rect) {
    let tokens = app.tokens.clone();
    card(ui, rect, &tokens, Stroke::new(1.0, tokens.bg3), |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("connect-title")).strong().size(15.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let n = app.connect.selection.items.len();
                if n > 1 {
                    ui.label(RichText::new(format!("{n} selected")).weak().small());
                }
            });
        });
        ui.label(RichText::new(t!("connect-list-hint")).weak().small());
        ui.add_space(6.0);
        let mut clicked: Option<(usize, Modifiers)> = None;
        let mut double: Option<usize> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).max_height(rect.height() - 110.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            for (i, e) in app.settings.engines.iter().enumerate() {
                let selected = app.connect.selection.items.contains(&i);
                let primary = app.connect.selection.primary == Some(i);
                let fill = if primary {
                    tokens.accent.gamma_multiply(0.28)
                } else if selected {
                    tokens.accent.gamma_multiply(0.14)
                } else {
                    Color32::TRANSPARENT
                };
                let id = ui.make_persistent_id(("conn-row", i));
                let (_, row_rect) = ui.allocate_space(Vec2::new(ui.available_width(), 40.0));
                let resp = ui.interact(row_rect, id, Sense::click_and_drag());
                let hovered = resp.hovered();
                ui.painter().rect_filled(row_rect, 5.0, if hovered && !selected { tokens.bg1 } else { fill });
                if primary {
                    ui.painter().rect_filled(Rect::from_min_size(row_rect.min, Vec2::new(3.0, row_rect.height())), 2.0, tokens.accent);
                }
                let unnamed = t!("connect-unnamed");
                let name = if e.name.trim().is_empty() { unnamed.as_str() } else { e.name.as_str() };
                ui.painter().text(row_rect.min + Vec2::new(12.0, 7.0), egui::Align2::LEFT_TOP, name, egui::FontId::proportional(13.5), tokens.fg0);
                ui.painter().text(row_rect.min + Vec2::new(12.0, 24.0), egui::Align2::LEFT_TOP, crate::util::ellipsize(&e.url, 34), egui::FontId::proportional(10.5), tokens.fg1);
                if resp.clicked() {
                    clicked = Some((i, ui.input(|inp| inp.modifiers)));
                }
                if resp.double_clicked() {
                    double = Some(i);
                }
                // Dragging a row drags the whole selection (or just that row if it is not selected).
                if resp.drag_started() {
                    let mut set: Vec<usize> = app.connect.selection.items.iter().copied().collect();
                    if !set.contains(&i) {
                        set = vec![i];
                    }
                    resp.dnd_set_drag_payload(DragConnections(set));
                }
                if resp.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    if let Some(p) = ui.ctx().pointer_latest_pos() {
                        let n = app.connect.selection.items.len().max(1);
                        let label = if n > 1 { format!("{n} connections") } else { name.to_string() };
                        let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("conn-drag")));
                        let galley = painter.layout_no_wrap(label, egui::FontId::proportional(12.0), Color32::WHITE);
                        let r = Rect::from_min_size(p + Vec2::new(12.0, 12.0), galley.size() + Vec2::new(12.0, 6.0));
                        painter.rect_filled(r, 5.0, tokens.accent);
                        painter.galley(r.min + Vec2::new(6.0, 3.0), galley, Color32::WHITE);
                    }
                }
            }
            // "+" row
            let (_, plus_rect) = ui.allocate_space(Vec2::new(ui.available_width(), 36.0));
            let plus = ui.interact(plus_rect, ui.make_persistent_id("conn-add"), Sense::click()).on_hover_text(t!("connect-add-tip"));
            if plus.hovered() {
                ui.painter().rect_filled(plus_rect, 5.0, tokens.bg1);
            }
            ui.painter().text(plus_rect.min + Vec2::new(12.0, 8.0), egui::Align2::LEFT_TOP, "+  New connection", egui::FontId::proportional(13.0), tokens.accent);
            if plus.clicked() {
                let n = app.settings.engines.len() + 1;
                app.settings.engines.push(EngineProfile { name: t!("connect-new-name", n = n), url: String::new(), ..Default::default() });
                app.connect.selection.select_only(app.settings.engines.len() - 1);
                app.connect.focus_name = true;
                app.connect.error = None;
                app.settings_dirty = true;
            }
            if app.settings.engines.is_empty() {
                ui.add_space(8.0);
                ui.label(RichText::new(t!("connect-empty")).weak());
            }
        });
        if let Some((i, mods)) = clicked {
            let changed_primary = app.connect.selection.primary != Some(i);
            app.connect.selection.click(i, mods);
            if changed_primary && !mods.alt {
                app.connect.error = None;
                app.settings.last_engine = i;
                app.settings_dirty = true;
            }
        }
        if let Some(i) = double {
            app.connect.selection.select_only(i);
            if !app.connect.connecting {
                start_connect(app);
            }
        }
        // keyboard: ↑↓ move the primary selection
        if !text_focused(ui.ctx()) && !app.settings.engines.is_empty() {
            let len = app.settings.engines.len();
            let cur = app.connect.selection.primary.unwrap_or(0);
            ui.input_mut(|i| {
                if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    app.connect.selection.select_only((cur + 1).min(len - 1));
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    app.connect.selection.select_only(cur.saturating_sub(1));
                }
                if i.consume_key(Modifiers::NONE, Key::Escape) && app.connect.selection.items.len() > 1 {
                    app.connect.selection.select_only(cur);
                }
            });
        }
    });
}

fn trash_bin(ui: &mut Ui, app: &mut App, rect: Rect) {
    let tokens = app.tokens.clone();
    let hovering = ui.ctx().dragged_id().is_some() && egui::DragAndDrop::has_payload_of_type::<DragConnections>(ui.ctx()) && ui.rect_contains_pointer(rect);
    let stroke = Stroke::new(if hovering { 2.0 } else { 1.0 }, if hovering { tokens.incident } else { tokens.bg3 });
    let frame = egui::Frame::new().fill(if hovering { tokens.incident.gamma_multiply(0.12) } else { tokens.bg2 }).stroke(stroke).corner_radius(8.0).inner_margin(10.0);
    let mut ui_child = crate::widgets::split::child_in(ui, rect);
    let (_, dropped) = ui_child.dnd_drop_zone::<DragConnections, _>(frame, |ui| {
        ui.set_min_size(rect.size() - Vec2::splat(22.0));
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("🗑").size(20.0).color(if hovering { tokens.incident } else { tokens.fg1 }));
            ui.label(RichText::new(if hovering { t!("connect-bin-release") } else { t!("connect-bin-hint") }).small().color(if hovering { tokens.incident } else { tokens.fg1 }));
        });
    });
    if let Some(payload) = dropped {
        request_delete(app, payload.0.clone());
    }
}

/// Ask before deleting; the action removes the connections and repairs the selection.
fn request_delete(app: &mut App, mut idx: Vec<usize>) {
    idx.sort_unstable();
    idx.dedup();
    idx.retain(|&i| i < app.settings.engines.len());
    if idx.is_empty() {
        return;
    }
    let names: Vec<String> = idx.iter().map(|&i| app.settings.engines[i].name.clone()).collect();
    let body = if names.len() == 1 {
        t!("connect-delete-one", name = names[0].clone())
    } else {
        t!("connect-delete-many", n = names.len(), list = names.join("\n• "))
    };
    let idx = Arc::new(idx);
    app.confirm = Some(Confirm::new(t!("connect-delete-title"), body, t!("common-delete"), move |app: &mut App| {
        for &i in idx.iter().rev() {
            if i < app.settings.engines.len() {
                app.settings.engines.remove(i);
            }
        }
        let remaining = app.settings.engines.len();
        app.connect.selection.after_removal(&idx, remaining);
        app.settings.last_engine = app.connect.selection.primary.unwrap_or(0);
        app.settings_dirty = true;
        app.toasts.push(crate::widgets::ToastKind::Success, t!("connect-deleted", n = idx.len()));
    }));
}

fn form_card(ui: &mut Ui, app: &mut App, rect: Rect) {
    let tokens = app.tokens.clone();
    card(ui, rect, &tokens, Stroke::new(1.0, tokens.bg3), |ui| {
        ui.label(RichText::new("Flightdeck").size(22.0).strong());
        ui.label(RichText::new(t!("connect-subtitle")).weak());
        ui.add_space(14.0);
        let Some(idx) = app.connect.selection.primary.filter(|&i| i < app.settings.engines.len()) else {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(t!("connect-select-hint")).weak());
            });
            ui.add_space(30.0);
            offline_button(ui, app);
            return;
        };
        let mut dirty = false;
        let focus_name = std::mem::take(&mut app.connect.focus_name);
        let show_password = app.connect.show_password;
        let mut toggle_show = false;
        {
            let p = &mut app.settings.engines[idx];
            egui::Grid::new("connect-grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                let l = ui.label(t!("connect-name"));
                let r = ui.add(egui::TextEdit::singleline(&mut p.name).desired_width(f32::INFINITY)).labelled_by(l.id);
                if focus_name {
                    r.request_focus();
                }
                dirty |= r.changed();
                ui.end_row();
                let l = ui.label(t!("connect-rest-url"));
                dirty |= ui.add(egui::TextEdit::singleline(&mut p.url).hint_text(crate::t!("connect-url-hint")).desired_width(f32::INFINITY)).labelled_by(l.id).changed();
                ui.end_row();
                let l = ui.label(t!("connect-user"));
                dirty |= ui.add(egui::TextEdit::singleline(&mut p.user).hint_text(t!("connect-user-hint")).desired_width(f32::INFINITY)).labelled_by(l.id).changed();
                ui.end_row();
                let l = ui.label(t!("connect-password"));
                ui.horizontal(|ui| {
                    dirty |= ui.add(egui::TextEdit::singleline(&mut p.password).password(!show_password).desired_width(ui.available_width() - 40.0)).labelled_by(l.id).changed();
                    if ui.selectable_label(show_password, "👁").on_hover_text(t!("connect-show-password")).clicked() {
                        toggle_show = true;
                    }
                });
                ui.end_row();
                ui.label("");
                ui.horizontal(|ui| {
                    dirty |= ui.checkbox(&mut p.remember_password, t!("connect-remember-password")).on_hover_text(t!("connect-remember-tip")).changed();
                    dirty |= ui.checkbox(&mut p.accept_invalid_certs, t!("connect-allow-self-signed")).changed();
                });
                ui.end_row();
            });
        }
        if toggle_show {
            app.connect.show_password = !app.connect.show_password;
        }
        if dirty {
            app.settings_dirty = true;
        }
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let btn = egui::Button::new(RichText::new(if app.connect.connecting { t!("connect-connecting") } else { format!("{}  ↵", t!("connect-connect")) }).color(Color32::WHITE)).fill(tokens.accent).min_size(Vec2::new(130.0, 30.0));
            if ui.add_enabled(!app.connect.connecting, btn).clicked() {
                start_connect(app);
            }
            if app.connect.connecting {
                ui.spinner();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| offline_button(ui, app));
        });
        if let Some(e) = &app.connect.error {
            ui.add_space(8.0);
            ui.label(RichText::new(e).color(tokens.incident));
        }
        ui.add_space(10.0);
        ui.label(RichText::new(t!("connect-url-note")).weak().small());
    });
}

fn offline_button(ui: &mut Ui, app: &mut App) {
    if ui.button(t!("connect-open-bpmn")).on_hover_text(t!("connect-open-bpmn-tip")).clicked() {
        if let Some(path) = std::env::var("FLIGHTDECK_BPMN").ok().map(std::path::PathBuf::from) {
            app.navigate(crate::state::Nav::File { path }, crate::state::NavMode::NewTab);
            app.offline = true;
        }
    }
}

pub fn start_connect(app: &mut App) {
    let Some(idx) = app.connect.selection.primary.filter(|&i| i < app.settings.engines.len()) else { return };
    app.connect.error = None;
    app.connect.connecting = true;
    app.settings.engines[idx].url = camunda::normalize_base_url(&app.settings.engines[idx].url);
    app.settings.last_engine = idx;
    app.settings_dirty = true;
    let profile = app.settings.engines[idx].clone();
    app.connect_with(profile, Some(idx));
}

#[cfg(test)]
mod tests {
    use super::*;
    const CTRL: Modifiers = Modifiers::CTRL;
    const SHIFT: Modifiers = Modifiers::SHIFT;
    const ALT: Modifiers = Modifiers::ALT;
    fn items(s: &ListSelection) -> Vec<usize> {
        s.items.iter().copied().collect()
    }

    #[test]
    fn click_ctrl_shift_alt_semantics() {
        let mut s = ListSelection::default();
        s.click(2, Modifiers::NONE);
        assert_eq!((items(&s), s.primary), (vec![2], Some(2)));
        s.click(4, CTRL);
        assert_eq!((items(&s), s.primary), (vec![2, 4], Some(4)));
        s.click(4, CTRL); // toggle off → primary falls back to another selected item
        assert_eq!((items(&s), s.primary), (vec![2], Some(2)));
        s.click(0, Modifiers::NONE);
        s.click(3, SHIFT);
        assert_eq!(items(&s), vec![0, 1, 2, 3], "range from the anchor");
        s.click(5, SHIFT | CTRL);
        assert_eq!(items(&s), vec![0, 1, 2, 3, 4, 5], "Ctrl+Shift extends without clearing");
        s.click(1, ALT);
        s.click(3, ALT);
        assert_eq!(items(&s), vec![0, 2, 4, 5]);
        assert_eq!(s.primary, Some(5));
        s.click(5, ALT);
        assert_eq!(s.primary, Some(0), "removing the primary picks the first remaining");
        s.click(9, Modifiers::NONE);
        assert_eq!((items(&s), s.primary), (vec![9], Some(9)));
    }

    #[test]
    fn selection_is_repaired_after_deleting_items() {
        let mut s = ListSelection::default();
        s.click(1, Modifiers::NONE);
        s.click(4, SHIFT); // 1..=4 selected, primary 4
        s.after_removal(&[1, 2], 4); // five items became three: old 3→1, old 4→2
        assert_eq!((items(&s), s.primary), (vec![1, 2], Some(2)));
        // deleting everything that was selected selects the neighbour at the deletion point
        let mut s = ListSelection::default();
        s.click(2, Modifiers::NONE);
        s.after_removal(&[2], 3);
        assert_eq!((items(&s), s.primary), (vec![2], Some(2)));
        s.after_removal(&[2], 2);
        assert_eq!((items(&s), s.primary), (vec![1], Some(1)), "clamped to the last remaining item");
        s.after_removal(&[0, 1], 0);
        assert_eq!((items(&s), s.primary), (vec![], None));
    }

    #[test]
    fn clamp_handles_shrunken_lists() {
        let mut s = ListSelection::default();
        s.click(7, Modifiers::NONE);
        s.clamp(3);
        assert_eq!((items(&s), s.primary), (vec![0], Some(0)));
        s.clamp(0);
        assert_eq!((items(&s), s.primary), (vec![], None));
    }
}
