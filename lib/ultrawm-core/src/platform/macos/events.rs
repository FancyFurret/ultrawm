use crate::platform::macos::event_listener_ax::EventListenerAX;
use crate::platform::macos::event_listener_cg::EventListenerCG;
use crate::platform::macos::event_listener_ns::EventListenerNS;
use crate::platform::{EventDispatcher, PlatformEventsImpl, PlatformResult};
use icrate::AppKit::NSApplicationLoad;
pub struct MacOSPlatformEvents;

unsafe impl PlatformEventsImpl for MacOSPlatformEvents {
    unsafe fn initialize(dispatcher: EventDispatcher) -> PlatformResult<()> {
        unsafe {
            NSApplicationLoad();
            let listener_ax = EventListenerAX::run(dispatcher.clone())?;
            let listener_ns = EventListenerNS::run(listener_ax.clone())?;
            let listener_cg = EventListenerCG::run(dispatcher.clone())?;

            // Intentionally leak the listeners so they live for the program duration
            // This prevents them from being dropped when this method returns
            std::mem::forget(listener_ax);
            std::mem::forget(listener_ns);
            std::mem::forget(listener_cg);
        }

        Ok(())
    }

    fn set_intercept_clicks(_intercept: bool) -> PlatformResult<()> {
        // TODO
        Ok(())
    }
}
