use crate::config::Config;
use crate::event_handlers::EventHandler;
use crate::event_loop_wm::WMOperationResult;
use crate::platform::{Position, WMEvent, WindowId};
use crate::wm::WindowManager;
use std::time::{Duration, Instant};

pub struct FocusOnHoverHandler {
    enabled: bool,
    last_focused_window: Option<WindowId>,
    last_check_time: Instant,
    last_check_position: Option<Position>,
    check_interval: Duration,
}

impl FocusOnHoverHandler {
    pub fn new() -> Self {
        let config = Config::current();

        Self {
            enabled: config.focus_on_hover,
            last_focused_window: None,
            last_check_time: Instant::now(),
            last_check_position: None,
            check_interval: Duration::from_millis(100), // Check at most 10 times per second
        }
    }

    fn mouse_moved(&mut self, pos: &Position, wm: &mut WindowManager) -> WMOperationResult<()> {
        if !self.enabled {
            return Ok(());
        }

        let now = Instant::now();

        // Throttle checks: only check if enough time has passed or position changed significantly
        let should_check = if let Some(last_pos) = &self.last_check_position {
            // Check if position changed significantly (more than 50 pixels)
            let dx = (pos.x - last_pos.x).abs();
            let dy = (pos.y - last_pos.y).abs();
            let moved_significantly = dx > 50 || dy > 50;

            // Or if enough time has passed
            let time_elapsed = now.duration_since(self.last_check_time) >= self.check_interval;

            moved_significantly || time_elapsed
        } else {
            // First check, always do it
            true
        };

        if !should_check {
            return Ok(());
        }

        self.last_check_time = now;
        self.last_check_position = Some(pos.clone());

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
