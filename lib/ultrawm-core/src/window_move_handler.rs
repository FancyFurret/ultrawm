use crate::config::Config;
use crate::drag_tracker::{WindowDragEvent, WindowDragTracker, WindowDragType};
use crate::overlay_window::{OverlayWindow, OverlayWindowBackgroundStyle, OverlayWindowConfig};
use crate::platform::{Bounds, PlatformEvent, Position, WindowId};
use crate::wm::WindowManager;
use crate::UltraWMResult;
use skia_safe::Color;

pub struct WindowMoveHandler {
    overlay: OverlayWindow,
    drag_tracker: WindowDragTracker,
    last_preview_bounds: Option<Bounds>,
    valid_tile_position: bool,
}

impl WindowMoveHandler {
    pub async fn new() -> UltraWMResult<Self> {
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
        .await?;
        Ok(Self {
            overlay,
            drag_tracker: WindowDragTracker::new(),
            last_preview_bounds: None,
            valid_tile_position: false,
        })
    }

    pub fn overlay_shown(&self) -> bool {
        self.overlay.shown()
    }

    pub fn handle(&mut self, event: &PlatformEvent, wm: &mut WindowManager) -> UltraWMResult<()> {
        match self.drag_tracker.handle_event(&event, &wm) {
            Some(WindowDragEvent::Drag(id, position, drag_type)) => {
                self.drag(id, position, drag_type, wm)
            }
            Some(WindowDragEvent::End(id, position, drag_type)) => {
                self.drop(id, position, drag_type, wm)
            }
            _ => Ok(()),
        }
    }

    fn drag(
        &mut self,
        id: WindowId,
        position: Position,
        drag_type: WindowDragType,
        wm: &mut WindowManager,
    ) -> UltraWMResult<()> {
        if drag_type == WindowDragType::Move {
            let bounds = if let Some(bounds) = wm.get_tile_bounds(id, &position) {
                self.valid_tile_position = true;
                bounds
            } else {
                self.valid_tile_position = false;
                wm.get_window(id).unwrap().bounds().clone()
            };

            if let Some(last_preview_bounds) = &self.last_preview_bounds {
                if &bounds == last_preview_bounds {
                    return Ok(());
                }
            }

            self.overlay.show()?;
            self.overlay.move_to(&bounds)?;
            self.last_preview_bounds = Some(bounds);
        }

        Ok(())
    }

    fn drop(
        &mut self,
        id: WindowId,
        position: Position,
        drag_type: WindowDragType,
        wm: &mut WindowManager,
    ) -> UltraWMResult<()> {
        if drag_type == WindowDragType::Move {
            self.overlay.hide()?;
            self.last_preview_bounds = None;

            if self.valid_tile_position {
                wm.tile_window(id, &position).unwrap_or_else(|_| {
                    println!("Could not tile window at position");
                });

                let new_layout = wm.serialize();
                std::fs::write(
                    "current_layout.yaml",
                    serde_yaml::to_string(&new_layout).unwrap(),
                )
                .unwrap();
            } else {
                // Move the window back to its original position
                let window = wm.get_window(id).unwrap();
                let tiled_bounds = window.bounds().clone();
                window.set_bounds(tiled_bounds);
                window.flush()?;
            }
        } else if let WindowDragType::Resize(direction) = drag_type {
            if let Some(window) = wm.get_window(id) {
                let new_bounds = window.platform_bounds();
                wm.resize_window(&window, &new_bounds, direction)
                    .unwrap_or_else(|_| {
                        println!("Could not resize window");
                    });
            }
        }

        Ok(())
    }
}
