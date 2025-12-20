use crate::config::Config;
use crate::event_handlers::mod_mouse_keybind_tracker::{KeybindEvent, ModMouseKeybindTracker};
use crate::event_handlers::EventHandler;
use crate::event_loop_wm::WMOperationResult;
use crate::platform::{ContextMenuRequest, Position, WMEvent, WindowId};
use crate::wm::WindowManager;
use crate::GLOBAL_EVENT_DISPATCHER;
use log::trace;

pub struct ContextMenuHandler {
    tracker: ModMouseKeybindTracker,
}

impl ContextMenuHandler {
    pub fn new() -> Self {
        let config = Config::current();
        Self {
            tracker: ModMouseKeybindTracker::new(
                config.mod_transform_bindings.context_menu.clone(),
            ),
        }
    }

    fn show_context_menu(&self, position: Position, target_window: Option<WindowId>) {
        trace!(
            "Showing context menu at {:?}, target_window: {:?}",
            position,
            target_window
        );

        if let Some(dispatcher) = GLOBAL_EVENT_DISPATCHER.get() {
            dispatcher.send(WMEvent::ShowContextMenu(ContextMenuRequest {
                position,
                target_window,
            }));
        }
    }
}

impl EventHandler for ContextMenuHandler {
    fn handle_event(&mut self, event: &WMEvent, wm: &mut WindowManager) -> WMOperationResult<bool> {
        if let Some(keybind_event) = self.tracker.handle_event(event) {
            match keybind_event {
                KeybindEvent::Activate(pos) => {
                    // Click without drag - show context menu
                    let target_window = wm.find_window_at_position(&pos).map(|w| w.id());
                    trace!(
                        "Context menu activated at {:?}, window: {:?}",
                        pos,
                        target_window
                    );
                    self.show_context_menu(pos, target_window);
                    return Ok(true);
                }
                _ => {}
            }
            return Ok(false);
        }

        if matches!(event, WMEvent::ConfigChanged) {
            let config = Config::current();
            self.tracker =
                ModMouseKeybindTracker::new(config.mod_transform_bindings.context_menu.clone());
        }

        Ok(false)
    }
}
