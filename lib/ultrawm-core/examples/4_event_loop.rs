use tokio::sync::mpsc;
use ultrawm_core::platform::{EventDispatcher, PlatformInit, PlatformInitImpl};

fn init() {
    unsafe {
        PlatformInit::initialize().expect("Error initializing platform");
    }
}

fn main() {
    init();

    let (tx, rx) = mpsc::unbounded_channel();
    let dispatch = EventDispatcher::new(tx);
    unsafe {
        PlatformInit::run_event_loop(dispatch).expect("Error running event loop");
    }
}
