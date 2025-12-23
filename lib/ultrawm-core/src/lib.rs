// TODO: Remove
#![allow(dead_code)]

use crate::event_loop_main::EventLoopMain;
use crate::event_loop_wm::EventLoopWM;
use crate::platform::{
    EventBridge, EventDispatcher, PlatformError, PlatformEvents, PlatformEventsImpl, WMEvent,
};
use crate::workspace::WorkspaceId;
use log::error;
use std::sync::mpsc;
use std::sync::OnceLock;
use std::time::Duration;
use std::{process, thread};

pub mod ai;
mod animation;
mod coalescing_channel;
mod commands;
pub mod config;
mod event_handlers;
mod event_loop_main;
pub mod event_loop_wm;
mod layouts;
mod overlay_window;
mod partition;
pub mod platform;
mod resize_handle;
mod serialization;
mod thread_lock;
pub mod tile_preview_handler;
mod tile_result;
mod window;
mod wm;
mod workspace;
mod workspace_animator;

use crate::platform::input_state::InputState;
use crate::wm::WMError;
pub use commands::{
    register_commands, CommandContext, CommandDef, CommandId, AI_ORGANIZE_ALL_WINDOWS,
    AI_ORGANIZE_CURRENT_WINDOW,
};
pub use config::Config;
pub use event_loop_main::run_on_main_thread_blocking;
pub use platform::inteceptor::Interceptor;
pub use platform::{ContextMenuRequest, Platform, Position, WindowId};

static GLOBAL_EVENT_DISPATCHER: OnceLock<EventDispatcher> = OnceLock::new();

// Context menu callback registration
type ContextMenuCallback = Box<dyn Fn(ContextMenuRequest) + Send + Sync>;
static CONTEXT_MENU_CALLBACK: OnceLock<ContextMenuCallback> = OnceLock::new();

/// Register a callback to be called when a context menu should be shown
pub fn set_context_menu_handler<F>(handler: F)
where
    F: Fn(ContextMenuRequest) + Send + Sync + 'static,
{
    let _ = CONTEXT_MENU_CALLBACK.set(Box::new(handler));
}

/// Called internally to trigger the context menu callback
pub(crate) fn trigger_context_menu(request: ContextMenuRequest) {
    if let Some(callback) = CONTEXT_MENU_CALLBACK.get() {
        callback(request);
    }
}

pub fn version() -> &'static str {
    option_env!("VERSION").unwrap_or("v0.0.0-dev")
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
    InputState::initialize();

    // Create a channel to signal when main thread is ready
    let (main_ready_tx, main_ready_rx) = mpsc::channel();

    // Spawn the WM thread but wait for main thread to be ready
    thread::spawn(move || {
        // Wait for signal that main thread is running
        if main_ready_rx.recv().is_err() {
            error!("Failed to receive main thread ready signal");
            process::exit(1);
        }

        thread::sleep(Duration::from_millis(1000));

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
