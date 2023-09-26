use crate::platform::{
    EventBridge, Platform, PlatformError, PlatformImpl, PlatformInit, PlatformInitImpl,
    PlatformWindowImpl,
};
use crate::wm::WindowManager;
use std::{process, thread};

mod config;
mod layouts;
mod partition;
mod platform;
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

    let _wm = WindowManager::new()?;

    loop {
        let _event = bridge
            .next_event()
            .await
            .ok_or("Could not get next event")?;
    }
}
