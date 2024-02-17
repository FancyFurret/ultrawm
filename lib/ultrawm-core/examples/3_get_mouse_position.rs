use ultrawm_core::platform::{Platform, PlatformImpl, PlatformInit, PlatformInitImpl};

fn init() {
    unsafe {
        PlatformInit::initialize().expect("Error initializing platform");
    }
}

fn main() {
    init();
    let position = Platform::get_mouse_position().expect("Error getting mouse position");
    println!("Mouse position: {:?}", position);
}
