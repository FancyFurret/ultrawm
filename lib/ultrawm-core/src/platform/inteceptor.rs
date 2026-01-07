use crate::platform::traits::PlatformImpl;
use crate::platform::{MouseButton, Platform, PlatformResult, WMEvent};
use log::{debug, error};
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

// Track how many requests exist for each button
static BUTTON_REQUEST_COUNTS: LazyLock<Mutex<HashMap<MouseButton, u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static BUTTON_STATES: LazyLock<Mutex<HashMap<crate::platform::MouseButton, bool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static HANDLED_BUTTONS: LazyLock<Mutex<HashSet<MouseButton>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

static INTERCEPT_BUTTONS: LazyLock<Mutex<HashSet<MouseButton>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

static IGNORE_NEXT_CLICK: LazyLock<Mutex<Option<MouseButton>>> = LazyLock::new(|| Mutex::new(None));

static IS_PAUSED: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

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
                if let Err(e) = Interceptor::intercept_button(button.clone(), true) {
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
                    if let Err(e) = Interceptor::intercept_button(button.clone(), false) {
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
        Interceptor::intercept_button(MouseButton::Button4, true)?;
        Interceptor::intercept_button(MouseButton::Button5, true)?;
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

    pub fn ignore_next_click(button: MouseButton) {
        if let Ok(mut ignore_next_click) = IGNORE_NEXT_CLICK.lock() {
            *ignore_next_click = Some(button);
        }
    }

    fn intercept_button(button: MouseButton, intercept: bool) -> PlatformResult<()> {
        if let Ok(mut intercept_buttons) = INTERCEPT_BUTTONS.lock() {
            if intercept {
                intercept_buttons.insert(button);
            } else {
                intercept_buttons.remove(&button);
            }
        }
        Ok(())
    }

    pub fn pop_ignore_click(button: MouseButton, up: bool) -> bool {
        if let Ok(mut ignore_next_click) = IGNORE_NEXT_CLICK.lock() {
            if let Some(ignore_button) = ignore_next_click.as_ref() {
                if ignore_button != &button {
                    return false;
                }

                if up {
                    *ignore_next_click = None;
                }
                return true;
            }
        }
        false
    }

    pub fn should_intercept_button(button: &MouseButton) -> bool {
        // If paused, don't intercept
        if let Ok(is_paused) = IS_PAUSED.lock() {
            if *is_paused {
                return false;
            }
        }

        if let Ok(intercept_buttons) = INTERCEPT_BUTTONS.lock() {
            if intercept_buttons.contains(button) {
                return true;
            }
        }

        false
    }

    /// Pause the interceptor - prevents all button interception
    pub fn pause() {
        if let Ok(mut is_paused) = IS_PAUSED.lock() {
            *is_paused = true;
            debug!("Interceptor paused");
        }
    }

    /// Resume the interceptor - allows button interception to resume
    pub fn resume() {
        if let Ok(mut is_paused) = IS_PAUSED.lock() {
            *is_paused = false;
            debug!("Interceptor resumed");
        }
    }
}
