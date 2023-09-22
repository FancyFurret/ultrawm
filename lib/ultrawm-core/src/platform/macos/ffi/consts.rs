macro_rules! cf_str {
    ($name:ident, $value:expr) => {
        pub fn $name() -> CFString {
            CFString::from_static_string($value)
        }
    };
}

pub mod accessibility_attribute {
    use core_foundation::string::CFString;

    cf_str!(title, "AXTitle");
    cf_str!(role, "AXRole");
    cf_str!(subrole, "AXSubrole");
    cf_str!(position, "AXPosition");
    cf_str!(size, "AXSize");
    cf_str!(windows, "AXWindows");
    cf_str!(focused_window, "AXFocusedWindow");
    cf_str!(minimized, "AXMinimized");
}

pub mod window_info {
    use core_foundation::string::CFString;

    cf_str!(owner_pid, "kCGWindowOwnerPID");
}

pub mod notification {
    use core_foundation::string::CFString;

    cf_str!(application_activated, "AXApplicationActivated");
    cf_str!(application_shown, "AXApplicationShown");
    cf_str!(application_hidden, "AXApplicationHidden");
    cf_str!(focused_window_changed, "AXFocusedWindowChanged");
    cf_str!(window_created, "AXWindowCreated");
    cf_str!(window_miniaturized, "AXWindowMiniaturized");
    cf_str!(window_deminiaturized, "AXWindowDeminiaturized");
    cf_str!(window_moved, "AXWindowMoved");
    cf_str!(window_resized, "AXWindowResized");
    cf_str!(element_destroyed, "AXUIElementDestroyed");
}

pub mod run_loop_mode {
    use core_foundation::runloop::{kCFRunLoopCommonModes, kCFRunLoopDefaultMode, CFRunLoopMode};

    pub fn default() -> CFRunLoopMode {
        unsafe { kCFRunLoopDefaultMode }
    }

    pub fn common_modes() -> CFRunLoopMode {
        unsafe { kCFRunLoopCommonModes }
    }
}
