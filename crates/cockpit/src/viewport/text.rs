//! Zoom-stable diagram labels.
//!
//! egui rasterises fonts at whole physical pixels and lays text out in screen space, so a label
//! that is re-laid-out at every zoom level changes its line breaks and jitters by a pixel as the
//! camera moves. Here the line breaks are computed once in *world* units (so they never depend on
//! the zoom), each line is laid out at one of a few cached "band" sizes, and the resulting glyph
//! mesh is scaled geometrically to the exact zoom. Within a band the scale factor stays within
//! about ±9 %, which is visually indistinguishable from a native rasterisation.

use egui::epaint::{Mesh, Tessellator, TextShape};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Vec2};
use std::collections::HashMap;
use std::sync::Arc;

/// BPMN's nominal label size in world units.
pub const FONT_WORLD: f32 = 12.0;
/// Reference size (world units × this) used to compute line breaks with sub-pixel precision.
const REF_SCALE: f32 = 4.0;
/// Bands per octave of zoom: 4 → neighbouring bands differ by 2^(1/4) ≈ 1.19.
const BANDS_PER_OCTAVE: f32 = 4.0;

/// Line breaks and fitting decision for one label, independent of zoom.
#[derive(Clone, Debug, PartialEq)]
pub struct Breaks {
    pub lines: Vec<String>,
    /// Multiplier on [`FONT_WORLD`] chosen so the block fits its box (1.0 = no shrink).
    pub font_scale: f32,
    /// Height of one line in world units at `font_scale`.
    pub line_height: f32,
}

impl Breaks {
    pub fn block_height(&self) -> f32 {
        self.lines.len() as f32 * self.line_height
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    text: String,
    wrap_w: i32,
    max_h: i32,
}

#[derive(Default)]
pub struct TextCache {
    breaks: HashMap<Key, Arc<Breaks>>,
    pending: Vec<Pending>,
}

impl std::fmt::Debug for TextCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextCache").field("labels", &self.breaks.len()).field("pending", &self.pending.len()).finish()
    }
}

struct Pending {
    galley: Arc<egui::Galley>,
    pos: Pos2,
    scale: f32,
    angle: f32,
    pivot: Pos2,
    clip: Rect,
}

/// Zoom band: the layout size is `FONT_WORLD * band`, drawn scaled by `zoom / band`.
pub fn band_for_zoom(zoom: f32) -> f32 {
    let z = zoom.max(1e-4);
    let steps = (z.log2() * BANDS_PER_OCTAVE).round();
    (steps / BANDS_PER_OCTAVE).exp2()
}

impl TextCache {
    /// Line breaks for `text` wrapped into `wrap_w` world units, shrinking the font (down to 60 %)
    /// when the block would be taller than `max_h` (pass `f32::INFINITY` for free labels).
    pub fn breaks(&mut self, painter: &Painter, text: &str, wrap_w: f32, max_h: f32) -> Arc<Breaks> {
        let key = Key { text: text.to_string(), wrap_w: (wrap_w * 10.0) as i32, max_h: if max_h.is_finite() { (max_h * 10.0) as i32 } else { -1 } };
        if let Some(b) = self.breaks.get(&key) {
            return b.clone();
        }
        let mut chosen: Option<Breaks> = None;
        for &font_scale in &[1.0, 0.9, 0.8, 0.7, 0.6] {
            let size = FONT_WORLD * font_scale * REF_SCALE;
            let galley = painter.layout(text.to_string(), FontId::proportional(size), Color32::WHITE, wrap_w * REF_SCALE);
            let line_height = galley.rows.first().map(|r| r.size.y).unwrap_or(size * 1.3) / REF_SCALE;
            let lines: Vec<String> = galley.rows.iter().map(|r| r.text().trim_end().to_string()).filter(|l| !l.is_empty()).collect();
            let b = Breaks { lines, font_scale, line_height };
            let fits = b.block_height() <= max_h + 0.01;
            chosen = Some(b);
            if fits {
                break;
            }
        }
        let b = Arc::new(chosen.unwrap_or(Breaks { lines: vec![], font_scale: 1.0, line_height: FONT_WORLD * 1.3 }));
        self.breaks.insert(key, b.clone());
        b
    }

    /// Queue a label for drawing. `rect` is the screen box, `zoom` the current scale, `align`
    /// where the text block sits inside the box. Rows are laid out at the band size now (cheap,
    /// egui caches galleys) and tessellated in [`Self::flush`].
    #[allow(clippy::too_many_arguments)]
    pub fn label(&mut self, painter: &Painter, text: &str, rect: Rect, zoom: f32, align: Align2, color: Color32, max_h_world: f32, clip: Rect) {
        let wrap_w = rect.width() / zoom;
        let breaks = self.breaks(painter, text, wrap_w, max_h_world);
        if breaks.lines.is_empty() {
            return;
        }
        let band = band_for_zoom(zoom);
        let k = zoom / band;
        let font = FontId::proportional(FONT_WORLD * breaks.font_scale * band);
        let galleys: Vec<Arc<egui::Galley>> = breaks.lines.iter().map(|l| painter.layout_no_wrap(l.clone(), font.clone(), color)).collect();
        let line_h = galleys.first().map(|g| g.size().y).unwrap_or(0.0) * k;
        let block_h = line_h * galleys.len() as f32;
        let block_w = galleys.iter().map(|g| g.size().x * k).fold(0.0, f32::max);
        let block = align.align_size_within_rect(Vec2::new(block_w, block_h), rect);
        for (i, g) in galleys.into_iter().enumerate() {
            let w = g.size().x * k;
            let x = match align.x() {
                egui::Align::Min => block.min.x,
                egui::Align::Center => block.center().x - w / 2.0,
                egui::Align::Max => block.max.x - w,
            };
            let y = block.min.y + i as f32 * line_h;
            self.pending.push(Pending { galley: g, pos: Pos2::new(x, y), scale: k, angle: 0.0, pivot: Pos2::ZERO, clip });
        }
    }

    /// A single-line label rotated by `angle` radians around `pivot` (lane names).
    #[allow(clippy::too_many_arguments)]
    pub fn label_rotated(&mut self, painter: &Painter, text: &str, zoom: f32, font_scale: f32, color: Color32, pos: Pos2, angle: f32, pivot: Pos2, clip: Rect) {
        let band = band_for_zoom(zoom);
        let k = zoom / band;
        let galley = painter.layout_no_wrap(text.to_string(), FontId::proportional(FONT_WORLD * font_scale * band), color);
        self.pending.push(Pending { galley, pos, scale: k, angle, pivot, clip });
    }

    /// Screen size a rotated label will occupy (width along its baseline, height), for centring.
    pub fn measure(&self, painter: &Painter, text: &str, zoom: f32, font_scale: f32) -> Vec2 {
        let band = band_for_zoom(zoom);
        let k = zoom / band;
        painter.layout_no_wrap(text.to_string(), FontId::proportional(FONT_WORLD * font_scale * band), Color32::WHITE).size() * k
    }

    /// Tessellate and emit all queued labels. Called once per frame after every label has been
    /// laid out, so the font atlas size is final for this frame.
    pub fn flush(&mut self, painter: &Painter) {
        if self.pending.is_empty() {
            return;
        }
        let ctx = painter.ctx();
        let (tex_size, opts) = (ctx.fonts(|f| f.font_image_size()), ctx.tessellation_options(|o| *o));
        let mut tess = Tessellator::new(painter.pixels_per_point(), opts, tex_size, Vec::new());
        for p in self.pending.drain(..) {
            let mut mesh = Mesh::default();
            tess.tessellate_text(&TextShape::new(Pos2::ZERO, p.galley, Color32::WHITE), &mut mesh);
            mesh.transform(egui::emath::TSTransform::new(p.pos.to_vec2(), p.scale));
            if p.angle != 0.0 {
                mesh.rotate(egui::emath::Rot2::from_angle(p.angle), p.pivot);
            }
            painter.with_clip_rect(p.clip.intersect(painter.clip_rect())).add(Shape::mesh(mesh));
        }
    }

    pub fn cached_labels(&self) -> usize {
        self.breaks.len()
    }

    /// Drop cached breaks (e.g. when the model changes).
    pub fn clear(&mut self) {
        self.breaks.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bands_are_quarter_octaves_around_the_zoom() {
        for z in [0.05, 0.3, 1.0, 1.1, 2.7, 40.0] {
            let b = band_for_zoom(z);
            let k = z / b;
            assert!((0.9..=1.1).contains(&k), "zoom {z}: band {b}, k {k}");
        }
        assert_eq!(band_for_zoom(1.0), 1.0);
        assert_eq!(band_for_zoom(2.0), 2.0);
    }
}
