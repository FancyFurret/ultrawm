use crate::platform::macos::ffi::{get_window_id, AXUIElementExt};
use crate::platform::traits::PlatformWindowImpl;
use crate::platform::{PlatformResult, Position, ProcessId, Size, WindowId};
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
