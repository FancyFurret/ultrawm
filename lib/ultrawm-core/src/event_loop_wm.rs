use crate::event_handlers::focus_on_hover_handler::FocusOnHoverHandler;
use crate::event_handlers::mod_transform_handler::ModTransformHandler;
use crate::event_handlers::native_transform_handler::NativeTransformHandler;
use crate::event_handlers::resize_handle_handler::ResizeHandleHandler;
use crate::event_handlers::EventHandler;
use crate::window::Window;
use crate::wm::WMError;
use crate::{
    event_loop_main::EventLoopMain,
    platform::{input_state::InputState, inteceptor::Interceptor, EventBridge, WMEvent},
    wm::WindowManager,
    UltraWMResult,
};
use log::{error, info, trace, warn};
use std::rc::Rc;
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
    pub async fn run(mut bridge: EventBridge) -> UltraWMResult<()> {
        trace!("Handling events...");

        let mut wm = WindowManager::new()?;

        let mut handlers = Self::create_handlers().await;
        let mut current_handler: Option<usize> = None;

        while let Some(event) = bridge.next_event().await {
            if matches!(event, WMEvent::Shutdown) {
                break;
            }

            if matches!(event, WMEvent::ConfigChanged) {
                info!("Reloading config...");
                handlers = Self::create_handlers().await;
                current_handler = None;
                wm.config_changed().unwrap_or_else(|e| {
                    error!("Could not reload config: {e}");
                });
            }

            InputState::handle_event(&event);
            Interceptor::handle_event(&event).unwrap_or_else(|e| {
                error!("Interceptor error: {e}");
            });

            match &event {
                WMEvent::WindowOpened(window) => {
                    wm.track_window(Rc::new(Window::new(window.clone())))
                        .unwrap_or_else(|_| {
                            warn!("Could not track window");
                        });
                }
                WMEvent::WindowShown(_) => {
                    // TODO: If the window was hidden, then bring it back to where it was
                }
                WMEvent::WindowClosed(id) | WMEvent::WindowHidden(id) => {
                    // TODO: Check if manageable
                    wm.remove_window(*id).unwrap_or_else(|_| {
                        // println!("Could not remove window");
                    });
                }
                _ => {}
            }

            if let Some(index) = current_handler.clone() {
                let handled = handlers[index]
                    .handle_event(&event, &mut wm)
                    .unwrap_or_else(|e| Self::handle_error(e));
                if !handled {
                    current_handler = None;
                }
            } else {
                for (index, handler) in handlers.iter_mut().enumerate() {
                    let handled = handler
                        .handle_event(&event, &mut wm)
                        .unwrap_or_else(|e| Self::handle_error(e));
                    if handled {
                        current_handler = Some(index);
                        break;
                    }
                }
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

    async fn create_handlers() -> Vec<Box<dyn EventHandler>> {
        vec![
            Box::new(NativeTransformHandler::new().await),
            Box::new(ResizeHandleHandler::new().await),
            Box::new(ModTransformHandler::new().await),
            Box::new(FocusOnHoverHandler::new()),
        ]
    }
}
