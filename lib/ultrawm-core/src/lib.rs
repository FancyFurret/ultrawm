// TODO: Remove
#![allow(dead_code)]

use crate::drag_tracker::{WindowDragEvent, WindowDragTracker, WindowDragType};
use crate::platform::{
    EventBridge, PlatformError, PlatformEvent, PlatformInit, PlatformInitImpl, PlatformTilePreview,
    PlatformTilePreviewImpl,
};
use crate::wm::WindowManager;
use std::{process, thread};

mod config;
mod drag_tracker;
mod layouts;
mod partition;
pub mod platform;
mod serialize;
mod tile_result;
mod window;
mod wm;
mod workspace;

#[derive(Debug)]
pub enum UltraWMFatalError {
    Error(String),
    PlatformError(PlatformError),
}

pub type UltraWMResult<T> = Result<T, UltraWMFatalError>;

impl From<PlatformError> for UltraWMFatalError {
    fn from(error: PlatformError) -> Self {
        UltraWMFatalError::PlatformError(error)
    }
}

impl From<&str> for UltraWMFatalError {
    fn from(value: &str) -> Self {
        UltraWMFatalError::Error(value.to_owned())
    }
}
impl From<String> for UltraWMFatalError {
    fn from(error: String) -> Self {
        UltraWMFatalError::Error(error)
    }
}

pub fn start() -> UltraWMResult<()> {
    unsafe {
        PlatformInit::initialize()?;
    }

    let bridge = EventBridge::new();
    let dispatcher = bridge.dispatcher();

    thread::spawn(move || {
        let tk = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        tk.block_on(start_async(bridge)).map_err(|e| {
            println!("Error running UltraWM: {:?}", e);
            process::exit(1);
        })
    });

    unsafe {
        PlatformInit::run_event_loop(dispatcher)?;
    }

    Ok(())
}

async fn start_async(mut bridge: EventBridge) -> UltraWMResult<()> {
    println!("Handling events...");

    let mut wm = WindowManager::new()?;
    let mut drag_tracker = WindowDragTracker::new();
    let mut tile_preview = PlatformTilePreview::new(wm.config())?;
    let mut last_preview_bounds = None;
    let mut tile_preview_shown = false;
    let mut valid_tile_position = false;

    loop {
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

                    if !tile_preview_shown {
                        tile_preview.show()?;
                        tile_preview_shown = true;
                    }

                    tile_preview.move_to(&bounds)?;
                    last_preview_bounds = Some(bounds);
                }
            }
            Some(WindowDragEvent::End(id, position, drag_type)) => {
                if drag_type == WindowDragType::Move {
                    if tile_preview_shown {
                        tile_preview.hide()?;
                        tile_preview_shown = false;
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
                }
            }
            _ => {}
        }
    }
}
