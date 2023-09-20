use crate::platform::macos::event_listener_ax::EventListenerAX;
use crate::platform::macos::event_listener_ns::EventListenerNS;
use crate::platform::macos::ffi::{
    get_window_id, window_info, AXUIElementExt, CFArrayExt, CFDictionaryExt,
};
use crate::platform::{
    EventDispatcher, PlatformInterface, PlatformResult, PlatformWindowInterface, Position,
    ProcessId, Size, WindowId,
};
use application_services::accessibility_ui::AXUIElement;
use application_services::pid_t;
use core_graphics::geometry::{CGPoint, CGSize};
use core_graphics::window::{self};
use icrate::objc2::rc::autoreleasepool;
use icrate::AppKit::NSApplicationLoad;
use icrate::Foundation::NSRunLoop;
use std::collections::HashSet;

mod event_listener_ax;
mod event_listener_ns;
mod ffi;

pub struct MacOSPlatform;

impl PlatformInterface for MacOSPlatform {
    fn list_all_windows() -> PlatformResult<Vec<MacOSPlatformWindow>> {
        let mut windows = Vec::new();
        for pid in MacOSPlatform::find_pids_with_windows()? {
            let app = AXUIElementExt::from(
                AXUIElement::create_application(pid as pid_t)
                    .map_err(|_| format!("Could not create AXUIElement for pid {}", pid))?,
            );

            if let Ok(app_windows) = app.windows() {
                for window in app_windows {
                    let window = MacOSPlatformWindow::new(window);
                    if let Ok(window) = window {
                        windows.push(window);
                    }
                }
            }
        }

        Ok(windows)
    }

    fn run_event_loop(dispatcher: EventDispatcher) -> PlatformResult<()> {
        autoreleasepool(|_| -> PlatformResult<()> {
            unsafe {
                NSApplicationLoad();

                let listener_ax = EventListenerAX::run(dispatcher.clone())?;
                let _listener_ns = EventListenerNS::run(listener_ax.clone())?;

                NSRunLoop::currentRunLoop().run();
            }

            Ok(())
        })
    }
}

impl MacOSPlatform {
    pub fn find_pids_with_windows() -> PlatformResult<HashSet<u32>> {
        let window_info =
            window::copy_window_info(window::kCGWindowListOptionAll, window::kCGNullWindowID);
        let window_info = window_info.ok_or("Could not get window info")?;
        let window_info = CFArrayExt::<CFDictionaryExt>::from(window_info);

        let mut pids = HashSet::new();
        for window in window_info {
            let pid = window
                .get_int(window_info::owner_pid())
                .ok_or("Could not get window pid")? as ProcessId;
            pids.insert(pid);
        }

        Ok(pids)
    }
}

#[derive(Debug, Clone)]
pub struct MacOSPlatformWindow {
    id: u32,
    pid: u32,
    element: AXUIElementExt,
}

impl MacOSPlatformWindow {
    pub fn new(element: AXUIElementExt) -> PlatformResult<Self> {
        let id = get_window_id(&element.element).ok_or("Could not get window id")?;
        let pid = element
            .element
            .get_pid()
            .map_err(|_| "Could not get window pid")?;

        Ok(Self {
            id,
            pid: pid as u32,
            element,
        })
    }
}

unsafe impl Send for MacOSPlatformWindow {}

impl PlatformWindowInterface for MacOSPlatformWindow {
    fn id(&self) -> WindowId {
        self.id
    }

    fn pid(&self) -> ProcessId {
        self.pid
    }

    fn title(&self) -> PlatformResult<String> {
        Ok(self
            .element
            .title()
            .unwrap_or("Unknown".to_string())
            .to_string())
    }

    fn position(&self) -> PlatformResult<Position> {
        let position = self.element.position()?;
        Ok(Position {
            x: position.x as u32,
            y: position.y as u32,
        })
    }

    fn size(&self) -> PlatformResult<Size> {
        let size = self.element.size()?;
        Ok(Size {
            width: size.width as u32,
            height: size.height as u32,
        })
    }

    fn visible(&self) -> PlatformResult<bool> {
        Ok(self.element.minimized()?)
    }

    fn move_to(&self, x: u32, y: u32) -> PlatformResult<()> {
        Ok(self
            .element
            .set_position(CGPoint::new(x as f64, y as f64))?)
    }

    fn resize(&self, width: u32, height: u32) -> PlatformResult<()> {
        Ok(self
            .element
            .set_size(CGSize::new(width as f64, height as f64))?)
    }
}
