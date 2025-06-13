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
    let displays = Platform::list_all_displays().expect("Error listing displays");

    for display in &displays {
        println!("Display: {:?}", display.name);
        println!(
            "\tID: {:?}\n\
            \tBounds: {:?}\n\
            \tWork Area: {:?}\n",
            display.id, display.bounds, display.work_area,
        );
    }
}
