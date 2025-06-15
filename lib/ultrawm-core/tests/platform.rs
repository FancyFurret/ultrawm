#[cfg(feature = "platform-tests")]
mod platform_tests {
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

    #[test]
    fn test_initialize() {
        init();
    }

    #[test]
    fn test_list_windows() {
        init();
        let windows = Platform::list_visible_windows().expect("Error listing windows");
        assert!(windows.len() > 0);
    }

    #[test]
    fn test_list_displays() {
        init();
        let displays = Platform::list_all_displays().expect("Error listing displays");
        assert!(displays.len() > 0);
    }

    #[test]
    fn test_get_mouse_position() {
        init();
        let position = Platform::get_mouse_position().expect("Error getting mouse position");
        assert!(position.x > 0);
        assert!(position.y > 0);
    }
}
