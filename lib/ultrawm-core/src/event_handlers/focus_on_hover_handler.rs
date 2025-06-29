use crate::config::Config;
use crate::event_handlers::EventHandler;
use crate::event_loop_wm::WMOperationResult;
use crate::platform::{Position, WMEvent, WindowId};
use crate::wm::WindowManager;

pub struct FocusOnHoverHandler {
    enabled: bool,
    last_focused_window: Option<WindowId>,
}

impl FocusOnHoverHandler {
    pub fn new() -> Self {
        let config = Config::current();

        Self {
            enabled: config.focus_on_hover,
            last_focused_window: None,
        }
    }

    fn mouse_moved(&mut self, pos: &Position, wm: &mut WindowManager) -> WMOperationResult<()> {
        if !self.enabled {
            return Ok(());
        }

        // Find the window at the current mouse position
        let window_at_position = wm.find_window_at_position(pos);

        if let Some(window) = window_at_position {
            let window_id = window.id();

            // Only focus if it's a different window than the last focused one
            if self.last_focused_window != Some(window_id) {
                wm.focus_window(window_id)?;
                self.last_focused_window = Some(window_id);
            }
        }

        Ok(())
    }

    pub fn update_config(&mut self) {
        let config = Config::current();
        self.enabled = config.focus_on_hover;
    }
}

impl EventHandler for FocusOnHoverHandler {
    fn handle_event(&mut self, event: &WMEvent, wm: &mut WindowManager) -> WMOperationResult<bool> {
        if !self.enabled {
            return Ok(false);
        }

        match event {
            WMEvent::MouseMoved(pos) => {
                self.mouse_moved(pos, wm)?;
                // Don't consume the event, let other handlers process it too
                Ok(false)
            }
            WMEvent::ConfigChanged => {
                self.update_config();
                Ok(false)
            }
            _ => Ok(false),
        }
    }
}
