use crate::platform::macos::ffi::{get_window_id, AXUIElementExt};
use crate::platform::traits::PlatformWindowImpl;
use crate::platform::{Bounds, PlatformResult, Position, ProcessId, Size, WindowId};
use core_graphics::geometry::{CGPoint, CGSize};

#[derive(Debug, Clone)]
pub struct MacOSPlatformWindow {
    id: u32,
    pid: u32,
    pub element: AXUIElementExt,
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
unsafe impl Sync for MacOSPlatformWindow {}

impl PlatformWindowImpl for MacOSPlatformWindow {
    fn id(&self) -> WindowId {
        self.id as WindowId
    }

    fn pid(&self) -> ProcessId {
        self.pid
    }

    fn title(&self) -> String {
        self.element
            .title()
            .unwrap_or("Unknown".to_string())
            .to_string()
    }

    fn position(&self) -> Position {
        let position = self
            .element
            .position()
            .expect("Could not get window position");
        Position {
            x: position.x as i32,
            y: position.y as i32,
        }
    }

    fn size(&self) -> Size {
        let size = self.element.size().expect("Could not get window size");
        Size {
            width: size.width as u32,
            height: size.height as u32,
        }
    }

    fn visible(&self) -> bool {
        self.element.minimized().unwrap_or(false)
    }

    fn set_bounds(&self, bounds: &Bounds) -> PlatformResult<()> {
        self.element.set_position(CGPoint::new(
            bounds.position.x as f64,
            bounds.position.y as f64,
        ))?;
        self.element.set_size(CGSize::new(
            bounds.size.width as f64,
            bounds.size.height as f64,
        ))?;
        Ok(())
    }
}
