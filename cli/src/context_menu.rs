use crate::menu_system::ContextMenuBuilder;
use log::{trace, warn};
use ultrawm_core::{
    run_on_main_thread_blocking, ContextMenuRequest, Interceptor, Position,
    AI_ORGANIZE_ALL_WINDOWS, AI_ORGANIZE_CURRENT_WINDOW, CLOSE_WINDOW, FLOAT_WINDOW,
    MINIMIZE_WINDOW,
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

        let position = request.position.clone();
        run_on_main_thread_blocking(move || {
            show_context_menu(request, position).unwrap_or_else(|e| {
                warn!("Failed to show context menu: {:?}", e);
            });
        });
    });
}

// Store the current context menu request so we can pass it when commands are triggered
static CURRENT_CONTEXT_MENU: std::sync::Mutex<Option<ContextMenuRequest>> =
    std::sync::Mutex::new(None);

pub(crate) fn get_current_context_menu() -> Option<ContextMenuRequest> {
    CURRENT_CONTEXT_MENU.lock().unwrap().clone()
}

fn show_context_menu(
    request: ContextMenuRequest,
    position: Position,
) -> Result<(), Box<dyn std::error::Error>> {
    // Store the context menu request
    if let Ok(mut current) = CURRENT_CONTEXT_MENU.lock() {
        *current = Some(request.clone());
    }

    let has_window = request.target_window.is_some();
    let mut menu_builder = ContextMenuBuilder::new().with_window(has_window);

    menu_builder.add_label(&format!("UltraWM {}", ultrawm_core::version()))?;
    menu_builder.add_separator()?;

    menu_builder.add_command(&AI_ORGANIZE_CURRENT_WINDOW)?;
    menu_builder.add_command(&AI_ORGANIZE_ALL_WINDOWS)?;

    if has_window {
        menu_builder.add_separator()?;
        menu_builder.add_command(&FLOAT_WINDOW)?;
        menu_builder.add_command(&CLOSE_WINDOW)?;
        menu_builder.add_command(&MINIMIZE_WINDOW)?;
    }

    let menu = menu_builder.build();

    Interceptor::pause();
    show_menu_at_position(&menu, &position);
    Interceptor::resume();

    // Clear the context after menu is dismissed
    if let Ok(mut current) = CURRENT_CONTEXT_MENU.lock() {
        *current = None;
    }

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
