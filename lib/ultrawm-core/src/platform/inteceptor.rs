use crate::platform::traits::PlatformImpl;
use crate::platform::{
    MouseButton, Platform, PlatformEvents, PlatformEventsImpl, PlatformResult, WMEvent,
};
use log::error;
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

// Track how many requests exist for each button
static BUTTON_REQUEST_COUNTS: LazyLock<Mutex<HashMap<MouseButton, u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static BUTTON_STATES: LazyLock<Mutex<HashMap<crate::platform::MouseButton, bool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static HANDLED_BUTTONS: LazyLock<Mutex<HashSet<MouseButton>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

#[derive(Debug)]
pub struct InterceptionRequest {
    buttons: HashSet<MouseButton>,
}

impl InterceptionRequest {
    fn new(buttons: HashSet<MouseButton>) -> Result<Self, String> {
        let mut button_counts = BUTTON_REQUEST_COUNTS
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        // Update button request counts and start intercepting buttons that weren't intercepted before
        for button in &buttons {
            let count = button_counts.entry(button.clone()).or_insert(0);
            if *count == 0 {
                // First request for this button - start intercepting
                if let Err(e) = PlatformEvents::intercept_button(button.clone(), true) {
                    error!("Failed to start intercepting button: {e:?}");
                }
            }
            *count += 1;
        }

        Ok(Self { buttons })
    }

    fn release(&self) -> Result<(), String> {
        let mut button_counts = BUTTON_REQUEST_COUNTS
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        // Update button request counts and stop intercepting buttons that have no more requests
        for button in &self.buttons {
            if let Some(count) = button_counts.get_mut(button) {
                *count -= 1;
                if *count == 0 {
                    // No more requests for this button - stop intercepting
                    button_counts.remove(button);
                    if let Err(e) = PlatformEvents::intercept_button(button.clone(), false) {
                        error!("Failed to stop intercepting button: {e:?}");
                    }
                }
            }
        }

        Ok(())
    }
}

impl Drop for InterceptionRequest {
    fn drop(&mut self) {
        if let Err(e) = self.release() {
            error!("Failed to release interception request on drop: {}", e);
        }
    }
}

pub struct Interceptor;

impl Interceptor {
    pub fn initialize() -> PlatformResult<()> {
        // Always intercept mouse modifiers
        PlatformEvents::intercept_button(MouseButton::Button4, true)?;
        PlatformEvents::intercept_button(MouseButton::Button5, true)?;
        BUTTON_REQUEST_COUNTS
            .lock()
            .unwrap()
            .insert(MouseButton::Button4, 1);
        BUTTON_REQUEST_COUNTS
            .lock()
            .unwrap()
            .insert(MouseButton::Button5, 1);
        Ok(())
    }

    pub fn request_interception(
        buttons: HashSet<MouseButton>,
    ) -> Result<InterceptionRequest, String> {
        InterceptionRequest::new(buttons)
    }

    pub fn release_request(request: InterceptionRequest) {
        // The Drop implementation will handle the release automatically
        drop(request);
    }

    pub fn has_active_requests_for_button(button: &MouseButton) -> bool {
        BUTTON_REQUEST_COUNTS
            .lock()
            .map(|counts| counts.get(button).unwrap_or(&0) > &0)
            .unwrap_or(false)
    }

    pub fn handle_event(event: &WMEvent) -> PlatformResult<()> {
        match event {
            WMEvent::MouseDown(_pos, button) => {
                // Track button down state for all intercepted buttons
                if Self::has_active_requests_for_button(button) {
                    if let Ok(mut states) = BUTTON_STATES.lock() {
                        states.insert(button.clone(), true);
                    }
                }
            }
            WMEvent::MouseUp(pos, button) => {
                let should_replay = if Self::has_active_requests_for_button(button) {
                    let mut handled = HANDLED_BUTTONS.lock().unwrap();
                    let was_handled = handled.contains(button);
                    handled.remove(button);
                    !was_handled
                } else {
                    false
                };

                if should_replay {
                    Platform::simulate_mouse_click(pos.clone(), button.clone())?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub fn set_handled(buttons: &HashSet<MouseButton>) {
        if let Ok(mut handled) = HANDLED_BUTTONS.lock() {
            for button in buttons {
                handled.insert(button.clone());
            }
        }
    }
}
