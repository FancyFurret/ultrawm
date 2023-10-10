use crate::drag_tracker::{DragTracker, WindowDragEvent};
use crate::platform::{
    EventBridge, Platform, PlatformError, PlatformEvent, PlatformImpl, PlatformInit,
    PlatformInitImpl, PlatformTilePreview, PlatformTilePreviewImpl, PlatformWindowImpl,
};
use crate::wm::WindowManager;
use std::{process, thread};

mod config;
mod drag_tracker;
mod layouts;
mod partition;
mod platform;
mod serialize;
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

    let windows =
        Platform::list_all_windows().map_err(|e| format!("Could not list windows: {:?}", e))?;

    println!("Found {} windows", windows.len());

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

    let mut _wm = WindowManager::new()?;
    let mut drag_tracker = DragTracker::new();
    let mut tile_preview = PlatformTilePreview::new()?;
    let mut last_preview_bounds = None;
    let mut tile_preview_shown = false;

    loop {
        let event = bridge
            .next_event()
            .await
            .ok_or("Could not get next event")?;

        match &event {
            PlatformEvent::WindowDestroyed(window) => {
                _wm.remove_window(*window).unwrap_or_else(|_| {
                    println!("Could not remove window");
                });
                _wm.flush_windows().unwrap_or_else(|_| {
                    println!("Could not flush windows");
                });
            }
            PlatformEvent::WindowHidden(window) => {
                _wm.remove_window(window.id()).unwrap_or_else(|_| {
                    println!("Could not remove window");
                });
                _wm.flush_windows().unwrap_or_else(|_| {
                    println!("Could not flush windows");
                });
            }
            _ => {}
        }

        match drag_tracker.handle_event(&event) {
            Some(WindowDragEvent::Start(_, _)) => {}
            Some(WindowDragEvent::Move(window, position)) => {
                let pos = _wm.get_tile_preview_for_position(&window, &position);
                if let Some(pos) = pos {
                    if let Some(last_preview_bounds) = &last_preview_bounds {
                        if &pos == last_preview_bounds {
                            continue;
                        }
                    }

                    if !tile_preview_shown {
                        tile_preview.show()?;
                        tile_preview_shown = true;
                    }
                    tile_preview.move_to(&pos)?;
                    last_preview_bounds = Some(pos);
                } else {
                    if tile_preview_shown {
                        tile_preview.hide()?;
                        tile_preview_shown = false;
                        last_preview_bounds = None;
                    }
                }
            }
            Some(WindowDragEvent::End(mut window, position)) => {
                if tile_preview_shown {
                    tile_preview.hide()?;
                    tile_preview_shown = false;
                    last_preview_bounds = None;

                    _wm.insert_window_at_position(&window, &position)
                        .unwrap_or_else(|_| {
                            println!("Could not insert window at position");
                        });
                    _wm.flush_windows().unwrap_or_else(|_| {
                        println!("Could not flush windows");
                    });

                    let new_layout = _wm.serialize();
                    std::fs::write(
                        "current_layout.yaml",
                        serde_yaml::to_string(&new_layout).unwrap(),
                    )
                    .unwrap();
                } else {
                    // Move the window back to its original position
                    let original_position = _wm.get_window_bounds(&window).unwrap();
                    window.set_bounds(&original_position).unwrap();
                }
            }
            _ => {}
        }
    }
}
