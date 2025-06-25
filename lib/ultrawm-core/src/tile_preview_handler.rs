use crate::config::Config;
use crate::event_loop_wm::{WMOperationError, WMOperationResult};
use crate::overlay_window::{OverlayWindow, OverlayWindowBackgroundStyle, OverlayWindowConfig};
use crate::platform::{Bounds, Position, WindowId};
use crate::wm::WindowManager;
use skia_safe::Color;

/// Handles tile preview overlay and tile position validation logic for drag/tile operations.
pub struct TilePreviewHandler {
    overlay: OverlayWindow,
    last_preview_bounds: Option<Bounds>,
    valid_tile_position: bool,
}

impl TilePreviewHandler {
    pub async fn new() -> Self {
        let config = Config::current();
        let overlay = OverlayWindow::new(OverlayWindowConfig {
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
            animation_fps: config.tile_preview_fps,
            border_radius: 20.0,
            blur: true,
            background: Some(OverlayWindowBackgroundStyle {
                color: Color::from_rgb(35, 35, 35),
                opacity: 0.5,
            }),
            border: None,
        })
        .await;
        Self {
            overlay,
            last_preview_bounds: None,
            valid_tile_position: false,
        }
    }

    /// Updates the preview overlay and tile position validity for the given window and position.
    /// Returns (Some(bounds), true) if a valid tile position, (Some(current bounds), false) otherwise.
    pub fn update_preview(
        &mut self,
        id: WindowId,
        pos: &Position,
        wm: &WindowManager,
    ) -> (Option<Bounds>, bool) {
        if let Some(bounds) = wm.get_tile_bounds(id, pos) {
            self.valid_tile_position = true;
            self.show_if_changed(&bounds);
            (Some(bounds), true)
        } else if let Ok(window) = wm.get_window(id) {
            let current = window.bounds().clone();
            self.valid_tile_position = false;
            self.show_if_changed(&current);
            (Some(current), false)
        } else {
            self.valid_tile_position = false;
            (None, false)
        }
    }

    pub fn show_if_changed(&mut self, bounds: &Bounds) {
        if self.last_preview_bounds.as_ref() != Some(bounds) {
            self.overlay.show();
            self.overlay.move_to(bounds);
            self.last_preview_bounds = Some(bounds.clone());
        }
    }

    pub fn hide(&mut self) {
        self.overlay.hide();
        self.last_preview_bounds = None;
    }

    pub fn is_shown(&self) -> bool {
        self.overlay.shown()
    }

    pub fn valid_tile_position(&self) -> bool {
        self.valid_tile_position
    }

    /// Call this on drop to perform the tile or restore action based on the last previewed position.
    pub fn tile_on_drop(
        &mut self,
        id: WindowId,
        pos: &Position,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        self.hide();
        if self.valid_tile_position {
            wm.tile_window(id, pos)
                .map_err(|e| WMOperationError::Move(e))?;
        } else {
            // Move the window back to its original position
            let window = wm.get_window(id)?;
            let tiled_bounds = window.bounds().clone();
            window.set_bounds(tiled_bounds);
            window
                .flush()
                .map_err(|e| WMOperationError::Move(e.into()))?;
        }
        Ok(())
    }
}
