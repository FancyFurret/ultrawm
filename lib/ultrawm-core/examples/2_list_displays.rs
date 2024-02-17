use ultrawm_core::platform::{Platform, PlatformImpl, PlatformInit, PlatformInitImpl};

fn init() {
    unsafe {
        PlatformInit::initialize().expect("Error initializing platform");
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
