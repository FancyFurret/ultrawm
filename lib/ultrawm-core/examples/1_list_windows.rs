use ultrawm_core::platform::{
    EventBridge, Platform, PlatformEvents, PlatformEventsImpl, PlatformImpl, PlatformWindowImpl,
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
    let windows = Platform::list_visible_windows().expect("Error listing windows");

    for window in &windows {
        println!("Window: {:?}", window.title());
        println!(
            "\tID: {:?}\n\
            \tPID: {:?}\n\
            \tPosition: {:?}\n\
            \tSize: {:?}\n\
            \tVisible: {:?}\n",
            window.id(),
            window.pid(),
            window.position(),
            window.size(),
            window.visible(),
        );
    }
}
