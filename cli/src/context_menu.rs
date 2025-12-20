use crate::menu_system::ContextMenuBuilder;
use log::{trace, warn};
use ultrawm_core::{
    run_on_main_thread_blocking, ContextMenuRequest, Interceptor, Position,
    AI_ORGANIZE_ALL_WINDOWS, AI_ORGANIZE_CURRENT_WINDOW,
};

/// Initialize the context menu system
/// Note: The unified menu event handler must be initialized first via menu_system::init_unified_handler()
pub fn init_context_menu() {
    // Register the context menu handler with ultrawm_core
    ultrawm_core::set_context_menu_handler(move |request: ContextMenuRequest| {
        trace!(
            "Context menu requested at {:?}, target_window: {:?}",
            request.position,
            request.target_window
        );
        // Must run on main thread - muda requires it
        let has_target_window = request.target_window.is_some();

        let position = request.position.clone();
        run_on_main_thread_blocking(move || {
            show_context_menu(has_target_window, position).unwrap_or_else(|e| {
                warn!("Failed to show context menu: {:?}", e);
            });
        });
    });
}

fn show_context_menu(
    has_target_window: bool,
    position: Position,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut menu_builder = ContextMenuBuilder::new();

    if has_target_window {
        menu_builder.add_command(&AI_ORGANIZE_CURRENT_WINDOW)?;
    }

    menu_builder.add_command(&AI_ORGANIZE_ALL_WINDOWS)?;

    let menu = menu_builder.build();

    Interceptor::pause();
    show_menu_at_position(&menu, &position);
    Interceptor::resume();

    Ok(())
}

#[cfg(target_os = "macos")]
fn show_menu_at_position(menu: &muda::Menu, position: &Position) {
    unsafe {
        use objc2_app_kit::NSMenu;
        use objc2_foundation::NSPoint;
        use ultrawm_core::Platform;

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
fn show_menu_at_position(menu: &muda::Menu, position: &Position) {
    use muda::ContextMenu as _;

    // On Windows, show at screen coordinates
    let hmenu_raw = menu.hpopupmenu();
    unsafe {
        use std::ffi::c_void;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, TrackPopupMenu, HMENU, TPM_LEFTALIGN, TPM_TOPALIGN,
        };

        let hmenu = HMENU(hmenu_raw as *mut c_void);
        let hwnd = GetForegroundWindow();
        let _ = TrackPopupMenu(
            hmenu,
            TPM_LEFTALIGN | TPM_TOPALIGN,
            position.x,
            position.y,
            None,
            hwnd,
            None,
        );
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn show_menu_at_position(_menu: &muda::Menu, _position: &Position) {
    warn!("Context menu not supported on this platform");
}
