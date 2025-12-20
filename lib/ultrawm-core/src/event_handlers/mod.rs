use crate::event_loop_wm::WMOperationResult;
use crate::platform::WMEvent;
use crate::wm::WindowManager;

pub mod native_transform_handler;
mod native_transform_tracker;

pub mod resize_handle_handler;
mod resize_handle_tracker;

pub mod mod_transform_handler;
mod mod_transform_tracker;

pub mod focus_on_hover_handler;
mod mod_mouse_keybind_tracker;

pub mod command_handler;
pub mod command_registry;
pub mod commands;
mod keyboard_keybind_tracker;

pub mod context_menu_handler;

pub trait EventHandler {
    /// Returns true if events currently being handled
    fn handle_event(&mut self, event: &WMEvent, wm: &mut WindowManager) -> WMOperationResult<bool>;
}
