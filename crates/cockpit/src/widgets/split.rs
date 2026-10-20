//! Resizable split panes with exact, finite rectangles. egui's `Panel` measures its content
//! against the panel's *maximum* size, which makes fill-width widgets and remainder table
//! columns push the panel to that maximum; these helpers avoid that by giving the content
//! ui the current pane rect and nothing more.

use egui::{CursorIcon, Id, Rect, Sense, Ui, Vec2};

const HANDLE: f32 = 6.0;

/// Where the secondary pane sits relative to the main one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Right,
    Bottom,
}

/// Result of a split: the main pane rect and the secondary pane rect (without the handle).
#[derive(Clone, Copy, Debug)]
pub struct Split {
    pub main: Rect,
    pub pane: Rect,
}

/// Carve a resizable pane of `*size` logical pixels off `side` of the available rect, draw its
/// drag handle, update `*size` on drag and return both rects. `size` is clamped to
/// `min ..= max_fraction * available`.
pub fn split(ui: &mut Ui, id: Id, side: Side, size: &mut f32, min: f32, max_fraction: f32) -> Split {
    let avail = ui.available_rect_before_wrap();
    let total = match side {
        Side::Right => avail.width(),
        Side::Bottom => avail.height(),
    };
    let max = (total * max_fraction).max(min);
    *size = size.clamp(min, max.max(min));
    let (main, pane, handle) = match side {
        Side::Right => {
            let x = avail.max.x - *size;
            (
                Rect::from_min_max(avail.min, egui::pos2(x - HANDLE / 2.0, avail.max.y)),
                Rect::from_min_max(egui::pos2(x, avail.min.y), avail.max),
                Rect::from_center_size(egui::pos2(x, avail.center().y), Vec2::new(HANDLE, avail.height())),
            )
        }
        Side::Bottom => {
            let y = avail.max.y - *size;
            (
                Rect::from_min_max(avail.min, egui::pos2(avail.max.x, y - HANDLE / 2.0)),
                Rect::from_min_max(egui::pos2(avail.min.x, y), avail.max),
                Rect::from_center_size(egui::pos2(avail.center().x, y), Vec2::new(avail.width(), HANDLE)),
            )
        }
    };
    let resp = ui.interact(handle, id, Sense::drag());
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(match side {
            Side::Right => CursorIcon::ResizeHorizontal,
            Side::Bottom => CursorIcon::ResizeVertical,
        });
    }
    if resp.dragged() {
        let d = resp.drag_delta();
        *size = (*size - match side {
            Side::Right => d.x,
            Side::Bottom => d.y,
        })
        .clamp(min, max);
    }
    let line_color = if resp.hovered() || resp.dragged() { ui.visuals().widgets.hovered.bg_stroke.color } else { ui.visuals().widgets.noninteractive.bg_stroke.color };
    match side {
        Side::Right => ui.painter().vline(pane.min.x - 0.5, pane.y_range(), egui::Stroke::new(1.0, line_color)),
        Side::Bottom => ui.painter().hline(pane.x_range(), pane.min.y - 0.5, egui::Stroke::new(1.0, line_color)),
    };
    Split { main, pane }
}

/// A child ui confined (and clipped) to `rect`.
pub fn child_in(ui: &mut Ui, rect: Rect) -> Ui {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down(egui::Align::Min)));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    child
}
