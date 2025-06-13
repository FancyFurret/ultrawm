use ultrawm_core::platform::{
    EventBridge, Platform, PlatformEvents, PlatformEventsImpl, PlatformImpl,
};

fn init() {
    unsafe {
        let bridge = EventBridge::new();
        let dispatcher = bridge.dispatcher();
        PlatformEvents::initialize(dispatcher).expect("Error initializing platform");
    }
}

fn main() {
    init();
    let position = Platform::get_mouse_position().expect("Error getting mouse position");
    println!("Mouse position: {:?}", position);
}
