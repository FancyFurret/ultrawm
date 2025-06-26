use crate::platform::traits::PlatformImpl;
use crate::platform::{
    Platform, PlatformEvent, PlatformEvents, PlatformEventsImpl, PlatformResult,
};
use log::error;
use std::collections::{HashMap, HashSet};
use std::sync::{atomic::AtomicU64, LazyLock, Mutex};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static ACTIVE_REQUESTS: LazyLock<Mutex<HashSet<u64>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

static BUTTON_STATES: LazyLock<Mutex<HashMap<crate::platform::MouseButton, bool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug)]
pub struct InterceptionRequest {
    id: u64,
}

impl InterceptionRequest {
    fn new() -> Result<Self, String> {
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut requests = ACTIVE_REQUESTS
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        let was_empty = requests.is_empty();
        requests.insert(id);

        // If this is the first request, start intercepting
        if was_empty {
            PlatformEvents::set_intercept_clicks(true).map_err(|e| {
                error!("Failed to set intercept clicks: {e:?}");
                format!("Failed to set intercept clicks: {e:?}")
            })?;
        }

        Ok(Self { id })
    }

    fn release(&self) -> Result<(), String> {
        let mut requests = ACTIVE_REQUESTS
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        requests.remove(&self.id);

        // If no more requests, stop intercepting
        if requests.is_empty() {
            PlatformEvents::set_intercept_clicks(false).map_err(|e| {
                error!("Failed to unset intercept clicks: {e:?}");
                format!("Failed to unset intercept clicks: {e:?}")
            })?;
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
    pub fn request_interception() -> Result<InterceptionRequest, String> {
        InterceptionRequest::new()
    }

    pub fn release_request(request: InterceptionRequest) {
        // The Drop implementation will handle the release automatically
        drop(request);
    }

    pub fn has_active_requests() -> bool {
        ACTIVE_REQUESTS
            .lock()
            .map(|requests| !requests.is_empty())
            .unwrap_or(false)
    }

    pub fn handle_event(event: &PlatformEvent) -> PlatformResult<()> {
        if !Self::has_active_requests() {
            return Ok(());
        }

        match event {
            PlatformEvent::MouseDown(_pos, button) => {
                if let Ok(mut states) = BUTTON_STATES.lock() {
                    states.insert(button.clone(), true);
                }
            }
            PlatformEvent::MouseUp(pos, button) => {
                let should_replay = {
                    let mut states = BUTTON_STATES.lock().unwrap();
                    let was_down = states.get(button).copied().unwrap_or(false);
                    states.insert(button.clone(), false);
                    was_down
                };

                if should_replay {
                    Platform::simulate_mouse_click(pos.clone(), button.clone())?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub fn set_handled() {
        if let Ok(mut states) = BUTTON_STATES.lock() {
            states.clear();
        }
    }
}
