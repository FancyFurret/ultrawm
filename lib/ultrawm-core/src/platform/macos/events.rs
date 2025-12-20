use crate::platform::macos::event_listener_ax::EventListenerAX;
use crate::platform::macos::event_listener_cg::EventListenerCG;
use crate::platform::macos::event_listener_ns::EventListenerNS;
use crate::platform::macos::platform::MacOSPlatform;
use crate::platform::{EventDispatcher, PlatformEventsImpl, PlatformResult};

pub struct MacOSPlatformEvents;

/// Check if the process has accessibility permissions
fn verify_accessibility_permissions() -> PlatformResult<()> {
    let trusted = unsafe { application_services::AXIsProcessTrusted() };
    if trusted == 0 {
        return Err("Accessibility permissions not granted. Please enable accessibility access for this app in System Preferences > Security & Privacy > Privacy > Accessibility".into());
    }
    Ok(())
}

unsafe impl PlatformEventsImpl for MacOSPlatformEvents {
    unsafe fn initialize(dispatcher: EventDispatcher) -> PlatformResult<()> {
        // Check accessibility permissions first
        verify_accessibility_permissions()?;

        // Initialize screen cache first
        MacOSPlatform::initialize_screens()?;

        let listener_ax = EventListenerAX::run(dispatcher.clone())?;
        let listener_ns = EventListenerNS::run(listener_ax.clone())?;
        let listener_cg = EventListenerCG::run(dispatcher.clone())?;

        // Intentionally leak the listeners so they live for the program duration
        // This prevents them from being dropped when this method returns
        std::mem::forget(listener_ax);
        std::mem::forget(listener_ns);
        std::mem::forget(listener_cg);

        Ok(())
    }

    unsafe fn finalize() -> PlatformResult<()> {
        // TODO: Clean up listeners
        // Note: Since we're intentionally leaking the listeners,
        // there's not much cleanup to do here
        Ok(())
    }
}
