// TODO: Remove
#![allow(dead_code)]

use crate::event_loop_main::EventLoopMain;
use crate::event_loop_wm::EventLoopWM;
use crate::platform::{
    EventBridge, EventDispatcher, PlatformError, PlatformEvents, PlatformEventsImpl, WMEvent,
};
use crate::tray::UltraWMTray;
use crate::workspace::WorkspaceId;
use log::error;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::{process, thread};

pub mod ai;
mod animation;
mod coalescing_channel;
mod commands;
pub mod config;
pub(crate) mod event_handlers;
mod event_loop_main;
pub mod event_loop_wm;
mod layouts;
pub mod menu;
pub mod overlay;
mod partition;
pub mod paths;
pub mod platform;
mod resize_handle;
mod serialization;
mod thread_lock;
pub mod tile_preview_handler;
mod tile_result;
pub mod tray;
mod window;
mod wm;
mod workspace;
mod workspace_animator;

use crate::menu::MenuSystem;
use crate::platform::input_state::InputState;
use crate::wm::WMError;
pub use commands::{
    register_commands, CommandContext, CommandDef, CommandId, AI_ORGANIZE_ALL_WINDOWS,
    AI_ORGANIZE_CURRENT_WINDOW, CLOSE_WINDOW, FLOAT_WINDOW, MINIMIZE_WINDOW,
};
pub use config::Config;
pub use event_loop_main::run_on_main_thread_blocking;
pub use platform::inteceptor::Interceptor;
pub use platform::{ContextMenuRequest, Platform, Position, WindowId};

static GLOBAL_EVENT_DISPATCHER: OnceLock<EventDispatcher> = OnceLock::new();

// Panic handling: store panic message to be retrieved by main thread
static PANIC_MESSAGE: OnceLock<Arc<Mutex<Option<String>>>> = OnceLock::new();

pub fn version() -> &'static str {
    option_env!("VERSION").unwrap_or("v0.0.0-dev")
}

/// Set up panic hook to catch panics from background threads
pub fn setup_panic_handler() {
    let panic_msg = Arc::new(Mutex::new(None::<String>));
    PANIC_MESSAGE.set(panic_msg.clone()).ok();

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Call the default hook first to get proper backtrace/logging
        default_hook(panic_info);

        // Format the panic message
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            format!("Panic: {}", s)
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            format!("Panic: {}", s)
        } else {
            "Panic: unknown error".to_string()
        };

        // Try to send message to main thread if it's initialized
        if let Some(sender) = crate::event_loop_main::MAIN_THREAD_TASK_SENDER.get() {
            let _ = sender.send(crate::event_loop_main::MainThreadMessage::PanicError {
                message: message.clone(),
            });
        }

        // Also store it in case main thread isn't ready yet
        if let Some(panic_msg_storage) = PANIC_MESSAGE.get() {
            if let Ok(mut msg) = panic_msg_storage.lock() {
                *msg = Some(message);
            }
        }
    }));
}

/// Check if a panic occurred and retrieve the message
pub fn check_panic() -> Option<String> {
    if let Some(panic_msg) = PANIC_MESSAGE.get() {
        if let Ok(mut msg) = panic_msg.lock() {
            return msg.take();
        }
    }
    None
}

pub fn reset_layout() -> UltraWMResult<()> {
    serialization::reset_layout().map_err(|_| "Failed to reset layout".into())
}

pub fn start_with_config(config: Config) -> UltraWMResult<()> {
    Config::set_config(config);
    start()
}

pub fn load_config(config: Config) -> UltraWMResult<()> {
    Config::set_config(config);
    if let Some(dispatcher) = GLOBAL_EVENT_DISPATCHER.get().cloned() {
        dispatcher.send(WMEvent::ConfigChanged);
    }

    Ok(())
}

pub fn shutdown() {
    if let Some(dispatcher) = GLOBAL_EVENT_DISPATCHER.get().cloned() {
        dispatcher.send(WMEvent::Shutdown);
    }
}

pub fn restart() {
    use log::info;
    use std::process::Command;

    info!("Restarting UltraWM...");

    // Get the current executable path
    if let Ok(exe_path) = std::env::current_exe() {
        // Spawn a new instance of the application
        let _ = Command::new(&exe_path)
            .args(std::env::args().skip(1))
            .spawn()
            .map_err(|e| error!("Failed to restart: {:?}", e));
    } else {
        error!("Failed to get current executable path for restart");
    }

    // Shutdown the current instance
    shutdown();
}

pub fn trigger_command(command_name: &str) {
    trigger_command_with_context(command_name, None);
}

pub fn trigger_command_with_context(
    command_name: &str,
    context: Option<crate::commands::CommandContext>,
) {
    if let Some(dispatcher) = GLOBAL_EVENT_DISPATCHER.get().cloned() {
        dispatcher.send(WMEvent::CommandTriggered(command_name.to_string(), context));
    }
}

pub fn load_layout_to_workspace(
    workspace_id: crate::workspace::WorkspaceId,
    layout: serde_yaml::Value,
) {
    if let Some(dispatcher) = GLOBAL_EVENT_DISPATCHER.get().cloned() {
        dispatcher.send(WMEvent::LoadLayoutToWorkspace(workspace_id, layout));
    }
}

pub fn place_window_relative(
    window_id: WindowId,
    target: crate::layouts::PlacementTarget,
    workspace_id: WorkspaceId,
) {
    if let Some(dispatcher) = GLOBAL_EVENT_DISPATCHER.get().cloned() {
        dispatcher.send(WMEvent::PlaceWindowRelative(
            window_id,
            target,
            workspace_id,
        ));
    }
}

pub fn float_window(window_id: WindowId) {
    if let Some(dispatcher) = GLOBAL_EVENT_DISPATCHER.get().cloned() {
        dispatcher.send(WMEvent::FloatWindow(window_id));
    }
}

pub fn start() -> UltraWMResult<()> {
    let bridge = EventBridge::new();
    let dispatcher = bridge.dispatcher();

    // Store the dispatcher globally for later use
    GLOBAL_EVENT_DISPATCHER.set(dispatcher.clone()).unwrap();

    unsafe {
        PlatformEvents::initialize(dispatcher)?;
    }

    Interceptor::initialize()?;
    InputState::initialize().map_err(|e| UltraWMFatalError::Error(e))?;
    MenuSystem::initialize().map_err(|e| UltraWMFatalError::Error(e))?;

    let _tray = UltraWMTray::initialize().map_err(|e| UltraWMFatalError::Error(e.to_string()))?;

    // Create a channel to signal when main thread is ready
    let (main_ready_tx, main_ready_rx) = mpsc::channel();

    // Spawn the WM thread but wait for main thread to be ready
    thread::spawn(move || {
        // Wait for signal that main thread is running
        if main_ready_rx.recv().is_err() {
            error!("Failed to receive main thread ready signal");
            process::exit(1);
        }

        let tk = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        tk.block_on(EventLoopWM::run(bridge)).map_err(|e| {
            error!("Error running UltraWM: {:?}", e);
            process::exit(1);
        })
    });

    // Signal that we're about to start the main event loop
    if main_ready_tx.send(()).is_err() {
        return Err("Failed to signal main thread ready".into());
    }

    // Start main event loop
    EventLoopMain::run()?;

    unsafe {
        PlatformEvents::finalize()?;
    }

    Ok(())
}

#[derive(Debug)]
pub enum UltraWMFatalError {
    Error(String),
    PlatformError(PlatformError),
    WMError(WMError),
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
