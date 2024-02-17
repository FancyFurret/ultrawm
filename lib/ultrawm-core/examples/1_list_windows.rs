use ultrawm_core::platform::{
    Platform, PlatformImpl, PlatformInit, PlatformInitImpl, PlatformWindowImpl,
};

fn init() {
    unsafe {
        PlatformInit::initialize().expect("Error initializing platform");
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
