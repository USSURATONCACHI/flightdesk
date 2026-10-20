//! Visual tokens (section 10 of DESIGN.md) and how they map onto egui's style.

use crate::viewport::DiagramStyle;
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke, Vec2, Visuals};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug)]
pub struct Tokens {
    pub bg0: Color32,
    pub bg1: Color32,
    pub bg2: Color32,
    pub bg3: Color32,
    pub fg0: Color32,
    pub fg1: Color32,
    pub fg2: Color32,
    pub accent: Color32,
    pub running: Color32,
    pub incident: Color32,
    pub completed: Color32,
    pub suspended: Color32,
    pub matched: Color32,
    pub external: Color32,
    pub dark: bool,
}

pub const ACCENT: Color32 = Color32::from_rgb(0x7C, 0x5C, 0xFF);
pub const RUNNING: Color32 = Color32::from_rgb(0x2F, 0x80, 0xED);
pub const INCIDENT: Color32 = Color32::from_rgb(0xE5, 0x48, 0x4D);
pub const COMPLETED: Color32 = Color32::from_rgb(0x30, 0xA4, 0x6C);
pub const SUSPENDED: Color32 = Color32::from_rgb(0xF5, 0xA5, 0x24);
pub const MATCHED: Color32 = Color32::from_rgb(0xF2, 0xC9, 0x4C);
pub const EXTERNAL: Color32 = Color32::from_rgb(0x12, 0xA5, 0x94);

impl Tokens {
    pub fn light() -> Self {
        Self {
            bg0: Color32::from_rgb(0xFA, 0xFA, 0xFB),
            bg1: Color32::from_rgb(0xF1, 0xF2, 0xF4),
            bg2: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            bg3: Color32::from_rgb(0xDC, 0xDE, 0xE3),
            fg0: Color32::from_rgb(0x1B, 0x1D, 0x22),
            fg1: Color32::from_rgb(0x5B, 0x60, 0x6B),
            fg2: Color32::from_rgb(0x9A, 0x9F, 0xA8),
            accent: ACCENT,
            running: RUNNING,
            incident: INCIDENT,
            completed: COMPLETED,
            suspended: SUSPENDED,
            matched: MATCHED,
            external: EXTERNAL,
            dark: false,
        }
    }
    pub fn dark() -> Self {
        Self {
            bg0: Color32::from_rgb(0x15, 0x17, 0x1B),
            bg1: Color32::from_rgb(0x1C, 0x1F, 0x25),
            bg2: Color32::from_rgb(0x23, 0x27, 0x2E),
            bg3: Color32::from_rgb(0x33, 0x38, 0x42),
            fg0: Color32::from_rgb(0xE8, 0xEA, 0xEE),
            fg1: Color32::from_rgb(0xA6, 0xAB, 0xB5),
            fg2: Color32::from_rgb(0x6C, 0x72, 0x7D),
            accent: Color32::from_rgb(0x9A, 0x82, 0xFF),
            running: Color32::from_rgb(0x5B, 0x9D, 0xF5),
            incident: Color32::from_rgb(0xF0, 0x6A, 0x6E),
            completed: Color32::from_rgb(0x4C, 0xC3, 0x8A),
            suspended: Color32::from_rgb(0xF7, 0xB5, 0x4A),
            matched: Color32::from_rgb(0xF5, 0xD3, 0x6A),
            external: Color32::from_rgb(0x2E, 0xC4, 0xB1),
            dark: true,
        }
    }

    pub fn diagram_style(&self) -> DiagramStyle {
        DiagramStyle {
            canvas: self.bg0,
            grid: if self.dark { Color32::from_rgb(0x2A, 0x2E, 0x36) } else { Color32::from_rgb(0xE3, 0xE5, 0xE9) },
            fill: self.bg2,
            stroke: if self.dark { Color32::from_rgb(0xC9, 0xCE, 0xD8) } else { Color32::from_rgb(0x2B, 0x2E, 0x35) },
            text: self.fg0,
            text_muted: self.fg1,
            lane_fill: if self.dark { Color32::from_rgb(0x19, 0x1C, 0x21) } else { Color32::from_rgb(0xF5, 0xF6, 0xF8) },
            lane_stroke: if self.dark { Color32::from_rgb(0x4A, 0x50, 0x5C) } else { Color32::from_rgb(0xB8, 0xBC, 0xC6) },
            flow: if self.dark { Color32::from_rgb(0xB5, 0xBA, 0xC6) } else { Color32::from_rgb(0x3A, 0x3E, 0x47) },
            running: self.running,
            incident: self.incident,
            completed: self.completed,
            suspended: self.suspended,
            selection: self.accent,
            hover: if self.dark { Color32::from_rgb(0x7D, 0xB4, 0xFF) } else { Color32::from_rgb(0x4A, 0x90, 0xE2) },
            matched: self.matched,
            current_match: if self.dark { Color32::from_rgb(0xFF, 0xA5, 0x2B) } else { Color32::from_rgb(0xE8, 0x8A, 0x00) },
            external: self.external,
            badge_text: Color32::WHITE,
            dark: self.dark,
        }
    }

    pub fn apply(&self, ctx: &egui::Context, scale: f32) {
        let mut v = if self.dark { Visuals::dark() } else { Visuals::light() };
        v.panel_fill = self.bg1;
        v.window_fill = self.bg2;
        v.extreme_bg_color = if self.dark { self.bg0 } else { self.bg2 };
        v.faint_bg_color = if self.dark { Color32::from_rgb(0x20, 0x23, 0x2A) } else { Color32::from_rgb(0xF6, 0xF7, 0xF9) };
        v.window_stroke = Stroke::new(1.0, self.bg3);
        v.widgets.noninteractive.bg_fill = self.bg1;
        v.widgets.noninteractive.weak_bg_fill = self.bg1;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.bg3);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.fg0);
        v.widgets.inactive.bg_fill = self.bg2;
        v.widgets.inactive.weak_bg_fill = self.bg2;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, self.bg3);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, self.fg0);
        v.widgets.hovered.bg_fill = if self.dark { Color32::from_rgb(0x2C, 0x31, 0x3A) } else { Color32::from_rgb(0xE9, 0xEB, 0xEF) };
        v.widgets.hovered.weak_bg_fill = v.widgets.hovered.bg_fill;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, self.accent.gamma_multiply(0.6));
        v.widgets.hovered.fg_stroke = Stroke::new(1.5, self.fg0);
        v.widgets.active.bg_fill = self.accent.gamma_multiply(0.25);
        v.widgets.active.weak_bg_fill = v.widgets.active.bg_fill;
        v.widgets.active.bg_stroke = Stroke::new(1.0, self.accent);
        v.widgets.active.fg_stroke = Stroke::new(2.0, self.fg0);
        v.widgets.open.bg_fill = self.bg2;
        v.widgets.open.weak_bg_fill = self.bg2;
        v.widgets.open.bg_stroke = Stroke::new(1.0, self.accent);
        v.selection.bg_fill = self.accent.gamma_multiply(if self.dark { 0.45 } else { 0.3 });
        v.selection.stroke = Stroke::new(1.0, self.accent);
        v.hyperlink_color = self.accent;
        v.warn_fg_color = self.suspended;
        v.error_fg_color = self.incident;
        v.override_text_color = Some(self.fg0);
        for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
            w.corner_radius = CornerRadius::same(4);
        }
        v.window_corner_radius = CornerRadius::same(6);
        v.menu_corner_radius = CornerRadius::same(6);
        v.popup_shadow = egui::epaint::Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(if self.dark { 120 } else { 40 }) };
        v.window_shadow = v.popup_shadow;
        v.striped = true;
        v.slider_trailing_fill = true;
        v.interact_cursor = Some(egui::CursorIcon::PointingHand);
        ctx.set_visuals(v);
        ctx.all_styles_mut(|s| {
            s.spacing.item_spacing = Vec2::new(8.0, 6.0);
            s.spacing.button_padding = Vec2::new(8.0, 4.0);
            s.spacing.interact_size = Vec2::new(28.0, 24.0);
            s.spacing.indent = 16.0;
            s.spacing.menu_margin = egui::Margin::same(6);
            s.spacing.window_margin = egui::Margin::same(12);
            s.spacing.scroll = egui::style::ScrollStyle::thin();
            s.animation_time = 0.12;
            use egui::{FontId, TextStyle};
            s.text_styles = [
                (TextStyle::Small, FontId::proportional(11.0 * scale)),
                (TextStyle::Body, FontId::proportional(13.0 * scale)),
                (TextStyle::Button, FontId::proportional(13.0 * scale)),
                (TextStyle::Heading, FontId::proportional(18.0 * scale)),
                (TextStyle::Monospace, FontId::monospace(12.0 * scale)),
            ]
            .into();
            s.visuals.override_text_color = Some(self.fg0);
        });
    }
}

/// Load system fonts with Cyrillic coverage when present; fall back to egui's built-ins.
pub fn install_fonts(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();
    let candidates_prop = ["/usr/share/fonts/noto/NotoSans-Regular.ttf", "/usr/share/fonts/TTF/NotoSans-Regular.ttf", "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf", "/usr/share/fonts/TTF/DejaVuSans.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "C:\\Windows\\Fonts\\segoeui.ttf", "/System/Library/Fonts/Supplemental/Arial Unicode.ttf"];
    let candidates_mono = ["/usr/share/fonts/TTF/JetBrainsMono-Regular.ttf", "/usr/share/fonts/TTF/DejaVuSansMono.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", "/usr/share/fonts/noto/NotoSansMono-Regular.ttf", "C:\\Windows\\Fonts\\consola.ttf", "/System/Library/Fonts/Menlo.ttc"];
    if let Some(bytes) = candidates_prop.iter().find_map(|p| std::fs::read(p).ok()) {
        defs.font_data.insert("ui".into(), Arc::new(FontData::from_owned(bytes)));
        defs.families.entry(FontFamily::Proportional).or_default().insert(0, "ui".into());
    }
    if let Some(bytes) = candidates_mono.iter().find_map(|p| std::fs::read(p).ok()) {
        defs.font_data.insert("mono".into(), Arc::new(FontData::from_owned(bytes)));
        defs.families.entry(FontFamily::Monospace).or_default().insert(0, "mono".into());
    }
    // Symbol fallback (arrows, geometric shapes, ⚙ …) before egui's emoji fonts.
    let symbol_candidates = ["/usr/share/fonts/TTF/DejaVuSans.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/noto/NotoSansSymbols2-Regular.ttf"];
    if let Some(bytes) = symbol_candidates.iter().find_map(|p| std::fs::read(p).ok()) {
        defs.font_data.insert("symbols".into(), Arc::new(FontData::from_owned(bytes)));
        for fam in [FontFamily::Proportional, FontFamily::Monospace] {
            let list = defs.families.entry(fam).or_default();
            let pos = list.iter().position(|n| n == "egui-icons").unwrap_or(list.len());
            list.insert(pos, "symbols".into());
        }
    }
    ctx.set_fonts(defs);
}
