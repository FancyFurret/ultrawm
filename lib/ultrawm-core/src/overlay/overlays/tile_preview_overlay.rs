use crate::config::Config;
use crate::overlay::OverlayContent;
use crate::overlay::{OverlayWindowBackgroundStyle, OverlayWindowConfig};
use crate::platform::{Bounds, PlatformResult};
use skia_safe::{Canvas, Color};

pub struct TilePreviewOverlay;

impl TilePreviewOverlay {
    pub fn new() -> Self {
        Self
    }
}

impl OverlayContent for TilePreviewOverlay {
    fn config(&self) -> OverlayWindowConfig {
        let config = Config::current();
        OverlayWindowConfig {
            fade_animation_ms: if config.tile_preview_fade_animate {
                config.tile_preview_animation_ms
            } else {
                0
            },
            move_animation_ms: if config.tile_preview_move_animate {
                config.tile_preview_animation_ms
            } else {
                0
            },
            border_radius: 20.0,
            blur: true,
            background: Some(OverlayWindowBackgroundStyle {
                color: Color::from_rgb(35, 35, 35),
                opacity: 0.5,
            }),
            border: None,
        }
    }

    fn draw(&mut self, _canvas: &Canvas, _bounds: &Bounds) -> PlatformResult<()> {
        // Tile preview is just a background overlay, no custom drawing needed
        Ok(())
    }
}

impl Default for TilePreviewOverlay {
    fn default() -> Self {
        Self::new()
    }
}
