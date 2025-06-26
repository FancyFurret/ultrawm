// TODO: Remove
#![allow(dead_code)]

use crate::event_loop_main::EventLoopMain;
use crate::event_loop_wm::EventLoopWM;
use crate::platform::inteceptor::Interceptor;
use crate::platform::{
    EventBridge, EventDispatcher, PlatformError, PlatformEvent, PlatformEvents, PlatformEventsImpl,
};
use log::error;
use std::sync::mpsc;
use std::sync::OnceLock;
use std::time::Duration;
use std::{process, thread};

mod animation;
pub mod config;
mod drag_handler;
mod drag_tracker;
mod event_loop_main;
pub mod event_loop_wm;
mod layouts;
mod modified_mouse_keybind_tracker;
mod overlay_window;
mod partition;
pub mod platform;
mod resize_handle;
mod resize_handle_tracker;
mod resize_handler;
mod serialization;
mod thread_lock;
pub mod tile_preview_handler;
mod tile_result;
mod window;
mod window_area_handler;
mod window_area_tracker;
mod wm;
mod workspace;

pub use config::Config;

static GLOBAL_EVENT_DISPATCHER: OnceLock<EventDispatcher> = OnceLock::new();

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
        dispatcher.send(PlatformEvent::ConfigChanged);
    }

    Ok(())
}

pub fn shutdown() {
    if let Some(dispatcher) = GLOBAL_EVENT_DISPATCHER.get().cloned() {
        dispatcher.send(PlatformEvent::Shutdown);
    }
}

pub fn start() -> UltraWMResult<()> {
    let bridge = EventBridge::new();
    let dispatcher = bridge.dispatcher();

    // Store the dispatcher globally for later use
    GLOBAL_EVENT_DISPATCHER.set(dispatcher.clone()).unwrap();

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

    unsafe {
        PlatformEvents::initialize(dispatcher)?;
    }

    Interceptor::initialize()?;

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
