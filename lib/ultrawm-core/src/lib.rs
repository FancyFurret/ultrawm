use crate::platform::{
    EventBridge, Platform, PlatformError, PlatformImpl, PlatformInit, PlatformInitImpl,
    PlatformTilePreview, PlatformTilePreviewImpl, PlatformWindowImpl,
};
use std::{process, thread};
use tokio::task;

mod platform;

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

    let mut tile_preview = PlatformTilePreview::new()?;
    tile_preview.show()?;

    let test_preview_frames = vec![
        (200, 500, 1000, 1000),
        (500, 200, 750, 500),
        (500, 500, 750, 750),
        (500, 750, 750, 1000),
        (750, 500, 1000, 750),
        (750, 750, 1000, 1000),
    ];

    task::spawn(async move {
        let mut i = 0;
        loop {
            let (x, y, width, height) = test_preview_frames[i % test_preview_frames.len()];
            tile_preview.move_to(x, y, width, height).unwrap();
            i += 1;

            if i % 5 == 0 {
                tile_preview.hide().unwrap();
            } else if i % 5 == 1 {
                tile_preview.show().unwrap();
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });

    loop {
        let event = bridge
            .next_event()
            .await
            .ok_or("Could not get next event")?;

        let window = event.window();
        let title = window.map_or("NO WINDOW".to_owned(), |w| {
            w.title().unwrap_or("NO NAME".to_owned())
        });

        println!("Event Received: {} | {:?}", title, event);
    }
}
