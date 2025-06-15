// TODO: Remove
#![allow(dead_code)]

use crate::event_loop_main::EventLoopMain;
use crate::event_loop_wm::EventLoopWM;
use crate::platform::{EventBridge, PlatformError, PlatformEvents, PlatformEventsImpl};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;
use std::{process, thread};

mod animation;
pub mod config;
mod drag_handle;
mod drag_tracker;
mod event_loop_main;
pub mod event_loop_wm;
mod handle_tracker;
mod layouts;
mod overlay_window;
mod partition;
pub mod platform;
mod serialization;
mod thread_lock;
mod tile_result;
mod window;
mod window_move_handler;
mod window_resize_handler;
mod wm;
mod workspace;

pub use config::Config;

pub fn version() -> &'static str {
    option_env!("VERSION").unwrap_or("v0.0.0-dev")
}

pub fn reset_layout() -> UltraWMResult<()> {
    serialization::reset_layout().map_err(|_| "Failed to reset layout".into())
}

pub fn start_with_config(shutdown: Arc<AtomicBool>, config: Config) -> UltraWMResult<()> {
    Config::set_config(config);
    start(shutdown)
}

pub fn start(shutdown: Arc<AtomicBool>) -> UltraWMResult<()> {
    let bridge = EventBridge::new();
    let dispatcher = bridge.dispatcher();

    // Create a channel to signal when main thread is ready
    let (main_ready_tx, main_ready_rx) = mpsc::channel();

    // Spawn the WM thread but wait for main thread to be ready
    let shutdown_wm = shutdown.clone();
    thread::spawn(move || {
        // Wait for signal that main thread is running
        if main_ready_rx.recv().is_err() {
            println!("Failed to receive main thread ready signal");
            process::exit(1);
        }

        thread::sleep(std::time::Duration::from_millis(1000));

        let tk = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        tk.block_on(EventLoopWM::run(bridge, shutdown_wm))
            .map_err(|e| {
                println!("Error running UltraWM: {:?}", e);
                process::exit(1);
            })
    });

    unsafe {
        PlatformEvents::initialize(dispatcher)?;
    }

    // Signal that we're about to start the main event loop
    if main_ready_tx.send(()).is_err() {
        return Err("Failed to signal main thread ready".into());
    }

    // Start main event loop
    EventLoopMain::run()?;

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
