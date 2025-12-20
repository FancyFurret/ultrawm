use crate::config::Config;
use crate::event_handlers::command_handler::CommandHandler;
use crate::event_handlers::context_menu_handler::ContextMenuHandler;
use crate::event_handlers::focus_on_hover_handler::FocusOnHoverHandler;
use crate::event_handlers::mod_transform_handler::ModTransformHandler;
use crate::event_handlers::native_transform_handler::NativeTransformHandler;
use crate::event_handlers::resize_handle_handler::ResizeHandleHandler;
use crate::event_handlers::EventHandler;
use crate::platform::PlatformWindowImpl;
use crate::window::Window;
use crate::wm::WMError;
use crate::{
    event_loop_main::EventLoopMain,
    platform::{
        input_state::InputState, inteceptor::Interceptor, EventBridge, Platform, PlatformImpl,
        WMEvent,
    },
    wm::WindowManager,
    UltraWMResult,
};
use log::{error, info, trace, warn};
use std::rc::Rc;
use std::time::Duration;
use thiserror::Error;
use tokio::time::Interval;

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

pub struct EventLoopWM {
    wm: WindowManager,
    handlers: Vec<Box<dyn EventHandler>>,
    current_handler: Option<usize>,
    flush_interval: Interval,
}

impl EventLoopWM {
    pub async fn new() -> UltraWMResult<Self> {
        let wm = WindowManager::new()?;
        let handlers = Self::create_handlers().await;

        Ok(Self {
            wm,
            handlers,
            current_handler: None,
            flush_interval: Self::create_flush_interval(),
        })
    }

    fn create_flush_interval() -> Interval {
        let flush_interval_ms = 1000 / Config::live_window_resize_fps().max(1);
        let mut interval = tokio::time::interval(Duration::from_millis(flush_interval_ms as u64));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval
    }

    pub async fn run(mut bridge: EventBridge) -> UltraWMResult<()> {
        trace!("Handling events...");

        // Log open windows at startup for debugging
        if let Ok(windows) = Platform::list_visible_windows() {
            trace!("Open windows ({}):", windows.len());
            for w in &windows {
                trace!("  {} - {:?}", w.id(), w.title());
            }
        }

        let mut event_loop = Self::new().await?;

        loop {
            tokio::select! {
                event = bridge.next_event() => {
                    match event_loop.handle_event(event).await {
                        LoopControl::Continue => {}
                        LoopControl::Break => break,
                    }
                }
                _ = event_loop.flush_interval.tick() => {
                    event_loop.flush();
                }
            }
        }

        event_loop.wm.cleanup()?;
        EventLoopMain::shutdown();
        Ok(())
    }

    async fn handle_event(&mut self, event: Option<WMEvent>) -> LoopControl {
        let Some(event) = event else {
            return LoopControl::Break;
        };

        if matches!(event, WMEvent::Shutdown) {
            return LoopControl::Break;
        }

        if matches!(event, WMEvent::ConfigChanged) {
            self.reload_config().await;
        }

        InputState::handle_event(&event);
        Interceptor::handle_event(&event).unwrap_or_else(|e| {
            error!("Interceptor error: {e}");
        });

        self.handle_window_event(&event);
        self.dispatch_to_handlers(&event);

        if let WMEvent::ShowContextMenu(request) = &event {
            crate::trigger_context_menu(request.clone());
            return LoopControl::Continue;
        }

        LoopControl::Continue
    }

    async fn reload_config(&mut self) {
        info!("Reloading config...");
        self.handlers = Self::create_handlers().await;
        self.current_handler = None;
        self.wm.config_changed().unwrap_or_else(|e| {
            error!("Could not reload config: {e}");
        });
    }

    fn handle_window_event(&mut self, event: &WMEvent) {
        match event {
            WMEvent::WindowOpened(window) => {
                self.wm
                    .track_window(Rc::new(Window::new(window.clone())))
                    .unwrap_or_else(|e| {
                        warn!("Could not track window: {e}");
                    });
            }
            WMEvent::WindowShown(id) => {
                self.wm.unhide_window(*id).unwrap_or_else(|e| {
                    warn!("Could not unhide window: {e}");
                });
            }
            WMEvent::WindowClosed(id) | WMEvent::WindowHidden(id) => {
                self.wm.remove_window(*id).unwrap_or_else(|_| {});
            }
            _ => {}
        }
    }

    fn dispatch_to_handlers(&mut self, event: &WMEvent) {
        if let Some(index) = self.current_handler {
            let handled = self.handlers[index]
                .handle_event(event, &mut self.wm)
                .unwrap_or_else(|e| {
                    error!("Error: {e}");
                    false
                });
            if !handled {
                self.current_handler = None;
            }
            return;
        }

        for (index, handler) in self.handlers.iter_mut().enumerate() {
            let handled = handler
                .handle_event(event, &mut self.wm)
                .unwrap_or_else(|e| {
                    error!("Error: {e}");
                    false
                });
            if handled {
                self.current_handler = Some(index);
                break;
            }
        }
    }

    fn flush(&mut self) {
        self.wm.flush().unwrap_or_else(|e| {
            error!("Flush error: {e}");
        });
    }

    async fn create_handlers() -> Vec<Box<dyn EventHandler>> {
        vec![
            Box::new(ContextMenuHandler::new()),
            Box::new(NativeTransformHandler::new().await),
            Box::new(ResizeHandleHandler::new().await),
            Box::new(ModTransformHandler::new().await),
            Box::new(FocusOnHoverHandler::new()),
            Box::new(CommandHandler::new().await),
        ]
    }
}

enum LoopControl {
    Continue,
    Break,
}
