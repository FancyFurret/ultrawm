use crate::drag_handler::WindowMoveHandler;
use crate::resize_handler::WindowResizeHandler;
use crate::window_area_handler::WindowAreaHandler;
use crate::wm::WMError;
use crate::{
    event_loop_main::EventLoopMain,
    platform::{inteceptor::Interceptor, EventBridge, PlatformEvent},
    wm::WindowManager,
    UltraWMResult,
};
use log::{error, trace, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WMOperationError {
    #[error("Could not move window: {0}")]
    Error(#[from] WMError),
    #[error("Could not move window: {0}")]
    Move(WMError),
    #[error("Could not resize window: {0}")]
    Resize(WMError),
}

pub type WMOperationResult<T> = Result<T, WMOperationError>;

pub struct EventLoopWM {}

impl EventLoopWM {
    pub async fn run(mut bridge: EventBridge, shutdown: Arc<AtomicBool>) -> UltraWMResult<()> {
        trace!("Handling events...");

        let mut wm = WindowManager::new()?;

        let mut move_handler = WindowMoveHandler::new().await;
        let mut resize_handler = WindowResizeHandler::new().await;
        let mut window_area_handler = WindowAreaHandler::new().await;

        while !shutdown.load(Ordering::SeqCst) {
            let event = bridge
                .next_event()
                .await
                .ok_or("Could not get next event")?;

            Interceptor::handle_event(&event).unwrap_or_else(|e| {
                error!("Interceptor error: {e}");
            });

            match &event {
                PlatformEvent::WindowOpened(window) => {
                    wm.track_window(window.clone()).unwrap_or_else(|_| {
                        warn!("Could not track window");
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

            let mut handled = move_handler
                .handle_event(&event, &mut wm)
                .unwrap_or_else(|e| Self::handle_error(e));

            if !handled {
                handled = window_area_handler
                    .handle_event(&event, &mut wm)
                    .unwrap_or_else(|e| Self::handle_error(e));
            }

            if !handled {
                resize_handler
                    .handle_event(&event, &mut wm)
                    .unwrap_or_else(|e| Self::handle_error(e));
            }
        }

        wm.cleanup()?;
        EventLoopMain::shutdown();
        Ok(())
    }

    fn handle_error(error: WMOperationError) -> bool {
        error!("Error: {error}");
        false
    }
}
