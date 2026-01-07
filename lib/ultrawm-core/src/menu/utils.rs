use crate::{Interceptor, Position};
use muda::Menu;

pub fn show_menu_at_position(menu: &Menu, position: &Position) {
    Interceptor::pause();

    #[cfg(target_os = "macos")]
    {
        show_menu_at_position_macos(menu, position);
    }

    #[cfg(target_os = "windows")]
    {
        show_menu_at_position_windows(menu, position);
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        log::warn!("Context menu not supported on this platform");
    }

    Interceptor::resume();
}

#[cfg(target_os = "macos")]
fn show_menu_at_position_macos(menu: &Menu, position: &Position) {
    unsafe {
        use crate::platform::Platform;
        use objc2_app_kit::NSMenu;
        use objc2_foundation::NSPoint;

        let max_screen_top = Platform::get_max_screen_top() as f64;
        let ns_position = NSPoint::new(position.x as f64, max_screen_top - position.y as f64);

        let ns_menu_ptr: *mut std::ffi::c_void = {
            use muda::ContextMenu;
            menu.ns_menu()
        };

        if !ns_menu_ptr.is_null() {
            let ns_menu: &NSMenu = &*(ns_menu_ptr as *const NSMenu);
            let _ = ns_menu.popUpMenuPositioningItem_atLocation_inView(None, ns_position, None);
        }
    }
}

#[cfg(target_os = "windows")]
fn show_menu_at_position_windows(menu: &Menu, position: &Position) {
    use crate::menu::system::MenuSystem;
    use muda::ContextMenu as _;

    // On Windows, show at screen coordinates
    let hmenu_raw = menu.hpopupmenu();

    unsafe {
        use std::ffi::c_void;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, TrackPopupMenu, HMENU, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_TOPALIGN,
        };

        let hmenu = HMENU(hmenu_raw as *mut c_void);
        let hwnd = GetForegroundWindow();

        let result = TrackPopupMenu(
            hmenu,
            TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RETURNCMD,
            position.x,
            position.y,
            None,
            hwnd,
            None,
        );

        let result_value = result.0;
        if result_value != 0 {
            let command_id = result_value as u32;
            MenuSystem::trigger_callback(&command_id.to_string());
        }
    }
}
