use crate::config::Config;
use crate::overlay::OverlayContent;
use crate::overlay::{OverlayWindowBackgroundStyle, OverlayWindowBorderStyle, OverlayWindowConfig};
use crate::platform::{Bounds, PlatformResult};
use skia_safe::{Canvas, Color};

pub struct ResizeHandleOverlay;

impl ResizeHandleOverlay {
    pub fn new() -> Self {
        Self
    }
}

impl OverlayContent for ResizeHandleOverlay {
    fn config(&self) -> OverlayWindowConfig {
        let config = Config::current();
        OverlayWindowConfig {
            fade_animation_ms: if config.tile_preview_fade_animate {
                config.tile_preview_animation_ms
            } else {
                0
            },
            move_animation_ms: 0,
            border_radius: 20.0,
            blur: true,
            background: Some(OverlayWindowBackgroundStyle {
                color: Color::from_rgb(35, 35, 35),
                opacity: 0.75,
            }),
            border: Some(OverlayWindowBorderStyle {
                width: 10,
                color: Color::from_rgb(30, 30, 30),
            }),
        }
    }

    fn draw(&mut self, _canvas: &Canvas, _bounds: &Bounds) -> PlatformResult<()> {
        // Resize handle is just a border/background overlay, no custom drawing needed
        Ok(())
    }
}

impl Default for ResizeHandleOverlay {
    fn default() -> Self {
        Self::new()
    }
}
