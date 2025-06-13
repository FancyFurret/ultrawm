use crate::config::Config;
use crate::overlay_window::{OverlayWindowBackgroundStyle, OverlayWindowBorderStyle};
use crate::{
    drag_handle::DragHandle,
    drag_tracker::{WindowDragEvent, WindowDragTracker, WindowDragType},
    event_loop_main::EventLoopMain,
    handle_tracker::{HandleDragEvent, HandleDragTracker},
    overlay_window::{OverlayWindow, OverlayWindowConfig},
    platform::{EventBridge, PlatformEvent},
    wm::WindowManager,
    UltraWMResult,
};
use skia_safe::Color;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct EventLoopWM {}

impl EventLoopWM {
    pub async fn run(mut bridge: EventBridge, shutdown: Arc<AtomicBool>) -> UltraWMResult<()> {
        println!("Handling events...");

        let mut wm = WindowManager::new()?;
        let mut drag_tracker = WindowDragTracker::new();
        let mut handle_tracker = HandleDragTracker::new();
        let mut handle_drag_active = false;
        let mut _active_drag_handle: Option<DragHandle> = None;
        let mut hover_drag_handle: Option<DragHandle> = None;
        let config = Config::current();

        let move_overlay = OverlayWindow::new(OverlayWindowConfig {
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
        let handle_overlay = OverlayWindow::new(OverlayWindowConfig {
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
                opacity: 0.75,
            }),
            border: Some(OverlayWindowBorderStyle {
                width: 10,
                color: Color::from_rgb(30, 30, 30),
            }),
        })
        .await?;
        let mut handle_overlay_shown = false;
        let mut last_preview_bounds = None;
        let mut move_overlay_shown = false;
        let mut valid_tile_position = false;

        while !shutdown.load(Ordering::SeqCst) {
            let event = bridge
                .next_event()
                .await
                .ok_or("Could not get next event")?;

            match &event {
                PlatformEvent::WindowOpened(window) => {
                    wm.track_window(window.clone()).unwrap_or_else(|_| {
                        println!("Could not track window");
                    });
                }
                PlatformEvent::WindowShown(_) => {
                    // TODO: If the window was hidden, then bring it back to where it was
                }
                PlatformEvent::WindowClosed(id) | PlatformEvent::WindowHidden(id) => {
                    // TODO: Check if manageable
                    wm.remove_window(*id).unwrap_or_else(|_| {
                        // println!("Could not remove window");
                    });
                }
                PlatformEvent::MouseMoved(pos) => {
                    if !handle_drag_active {
                        let handle_under_cursor = wm.drag_handle_at_position(pos);

                        if handle_under_cursor.is_some() && hover_drag_handle.is_none() {
                            // Started hovering a handle
                            hover_drag_handle = handle_under_cursor.clone();

                            let preview_bounds = handle_under_cursor
                                .as_ref()
                                .unwrap()
                                .preview_bounds(config.drag_handle_width);

                            if !handle_overlay_shown {
                                handle_overlay.show()?;
                                handle_overlay_shown = true;
                            }

                            handle_overlay.move_to(&preview_bounds)?;
                            last_preview_bounds = Some(preview_bounds);
                        } else if handle_under_cursor.is_none() && hover_drag_handle.is_some() {
                            // Exited hover
                            hover_drag_handle = None;
                            if !handle_drag_active && handle_overlay_shown {
                                handle_overlay.hide()?;
                                handle_overlay_shown = false;
                                last_preview_bounds = None;
                            }
                        }
                    }
                }
                _ => {}
            }

            match drag_tracker.handle_event(&event, &wm) {
                Some(WindowDragEvent::Start(_, _, _)) => {}
                Some(WindowDragEvent::Drag(id, position, drag_type)) => {
                    if drag_type == WindowDragType::Move {
                        let bounds = if let Some(bounds) = wm.get_tile_bounds(id, &position) {
                            valid_tile_position = true;
                            bounds
                        } else {
                            valid_tile_position = false;
                            wm.get_window(id).unwrap().bounds().clone()
                        };

                        if let Some(last_preview_bounds) = &last_preview_bounds {
                            if &bounds == last_preview_bounds {
                                continue;
                            }
                        }

                        if !move_overlay_shown {
                            move_overlay.show()?;
                            move_overlay_shown = true;
                        }

                        move_overlay.move_to(&bounds)?;
                        last_preview_bounds = Some(bounds);
                    }
                }
                Some(WindowDragEvent::End(id, position, drag_type)) => {
                    if drag_type == WindowDragType::Move {
                        if move_overlay_shown {
                            move_overlay.hide()?;
                            move_overlay_shown = false;
                            last_preview_bounds = None;
                        }

                        if valid_tile_position {
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
                            window.flush().unwrap();
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
                }
                _ => {}
            }

            // Handle drag-handle based resizing
            match handle_tracker.handle_event(&event, &wm) {
                Some(HandleDragEvent::Start(handle, _pos)) => {
                    handle_drag_active = true;
                    _active_drag_handle = Some(handle.clone());

                    // Show a thin tile preview bar for the handle
                    let mut preview_bounds = handle.preview_bounds(config.drag_handle_width);
                    // Expand preview to full span of container along perpendicular axis for clarity
                    if handle.orientation == crate::drag_handle::HandleOrientation::Vertical {
                        preview_bounds.position.y = 0; // temp full screen; todo: use container bounds
                        preview_bounds.size.height = u32::MAX; // placeholder
                    } else {
                        preview_bounds.position.x = 0;
                        preview_bounds.size.width = u32::MAX;
                    }

                    if !handle_overlay_shown {
                        handle_overlay.show()?;
                        handle_overlay_shown = true;
                    }
                    handle_overlay.move_to(&preview_bounds)?;
                    last_preview_bounds = Some(preview_bounds);
                }
                Some(HandleDragEvent::Drag(handle, pos)) => {
                    if handle_drag_active {
                        let mut preview_bounds = handle.preview_bounds(config.drag_handle_width);
                        if handle.orientation == crate::drag_handle::HandleOrientation::Vertical {
                            let clamped_x = handle.clamp_coordinate(pos.x);
                            preview_bounds.position.x =
                                clamped_x - (preview_bounds.size.width as i32 / 2);
                        } else {
                            let clamped_y = handle.clamp_coordinate(pos.y);
                            preview_bounds.position.y =
                                clamped_y - (preview_bounds.size.height as i32 / 2);
                        }

                        if Some(&preview_bounds) != last_preview_bounds.as_ref() {
                            if !handle_overlay_shown {
                                handle_overlay.show()?;
                                handle_overlay_shown = true;
                            }

                            handle_overlay.move_to(&preview_bounds)?;
                            last_preview_bounds = Some(preview_bounds);
                        }
                    }
                }
                Some(HandleDragEvent::End(_handle, _pos)) => {
                    if handle_drag_active {
                        // Hide preview
                        if handle_overlay_shown {
                            handle_overlay.hide()?;
                            handle_overlay_shown = false;
                            last_preview_bounds = None;
                        }

                        handle_drag_active = false;

                        // TODO: Apply actual resize logic to wm using handle and final position
                    }
                }
                None => {}
            }
        }

        EventLoopMain::shutdown();
        wm.cleanup()?;
        Ok(())
    }
}
