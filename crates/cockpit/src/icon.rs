//! The application icon, embedded so the window carries it on X11/Xwayland, Windows and macOS.
//! On Wayland the compositor looks the icon up by app id (`flightdeck`) in the installed
//! desktop entry instead; see `packaging/`.

use std::sync::Arc;

/// 256 px raster of `assets/icon.svg` (regenerate with `scripts/render-icon.sh`).
const PNG: &[u8] = include_bytes!("../../../assets/icon-256.png");

pub fn window_icon() -> Option<Arc<egui::IconData>> {
    let img = image::load_from_memory_with_format(PNG, image::ImageFormat::Png).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(Arc::new(egui::IconData { rgba: img.into_raw(), width, height }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icon_decodes_to_a_square_rgba_bitmap() {
        let icon = window_icon().expect("embedded PNG must decode");
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
        // The tile is opaque; the rounded corners are transparent.
        let px = |x: u32, y: u32| &icon.rgba[((y * 256 + x) * 4) as usize..][..4];
        assert_eq!(px(128, 128)[3], 255, "centre is opaque");
        assert_eq!(px(0, 0)[3], 0, "corner is transparent (rounded tile)");
    }
}
