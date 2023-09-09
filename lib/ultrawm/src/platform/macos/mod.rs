use self::api::{CFArrayExt, CFDictionaryExt};
use super::{PlatformError, PlatformResult, PlatformTrait};
use crate::platform::common::{Position, Size};
use crate::platform::macos::api::{
    accessibility_attribute, window_info, AXUIElementExt, AXValueExt,
};
use crate::platform::{PlatformWindow, PlatformWindowTrait};
use application_services::accessibility_ui::create_application_element;
use core_foundation::boolean::CFBoolean;
use core_foundation::{base::ToVoid, string::CFString};
use core_graphics::geometry::{CGPoint, CGSize};
use core_graphics::window::{self};
use std::collections::HashSet;

mod api;

pub struct MacOSPlatform {}

impl MacOSPlatform {
    fn find_pids_with_windows(&self) -> PlatformResult<HashSet<u32>> {
        let window_info = window::copy_window_info(
            window::kCGWindowListOptionExcludeDesktopElements,
            window::kCGNullWindowID,
        );
        let window_info = window_info.ok_or(PlatformError::Unknown)?;
        let window_info = CFArrayExt::<CFDictionaryExt>::new(window_info);

        let mut pids = HashSet::new();
        for window in window_info {
            let pid = window
                .get_int(window_info::owner_pid())
                .ok_or(PlatformError::Error("Could not get window pid"))?
                as u32;
            pids.insert(pid);
        }

        Ok(pids)
    }
}

impl PlatformTrait for MacOSPlatform {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    fn list_all_windows(&self) -> PlatformResult<Vec<PlatformWindow>> {
        let mut windows = Vec::new();
        for pid in self.find_pids_with_windows()? {
            let app = AXUIElementExt::new(create_application_element(pid as i32));
            let elements = app.copy_attribute_value::<CFArrayExt<AXUIElementExt>>(
                accessibility_attribute::windows(),
            );

            if let Some(elements) = elements {
                for window in elements {
                    windows.push(MacOSPlatformWindow::new(pid, window));
                }
            }
        }

        Ok(windows)
    }
}

#[derive(Debug, Clone)]
pub struct MacOSPlatformWindow {
    pid: u32,
    element: AXUIElementExt,
}

impl MacOSPlatformWindow {
    pub fn new(pid: u32, element: AXUIElementExt) -> Self {
        Self { pid, element }
    }
}

impl PlatformWindowTrait for MacOSPlatformWindow {
    fn id(&self) -> PlatformResult<u32> {
        Ok(self.element.element.to_void() as u32)
    }

    fn pid(&self) -> PlatformResult<u32> {
        Ok(self.pid)
    }

    fn title(&self) -> PlatformResult<String> {
        Ok(self
            .element
            .copy_attribute_value::<CFString>(accessibility_attribute::title())
            .unwrap_or_else(|| CFString::new("Unknown"))
            .to_string())
    }

    fn position(&self) -> PlatformResult<Position> {
        let position = self
            .element
            .copy_attribute_value::<AXValueExt>(accessibility_attribute::position())
            .ok_or(PlatformError::Error("Could not get window position"))?
            .into_point()
            .ok_or(PlatformError::Error(
                "Could not get convert position to point",
            ))?;

        Ok(Position {
            x: position.x as u32,
            y: position.y as u32,
        })
    }

    fn size(&self) -> PlatformResult<Size> {
        let size = self
            .element
            .copy_attribute_value::<AXValueExt>(accessibility_attribute::size())
            .ok_or(PlatformError::Error("Could not get window size"))?
            .into_size()
            .ok_or(PlatformError::Error("Could not get convert size to size"))?;

        Ok(Size {
            width: size.width as u32,
            height: size.height as u32,
        })
    }

    fn visible(&self) -> PlatformResult<bool> {
        Ok(self
            .element
            .copy_attribute_value::<CFBoolean>(accessibility_attribute::enabled())
            .ok_or(PlatformError::Error("Could not get window visibility"))?
            .into())
    }

    fn move_to(&self, x: u32, y: u32) -> PlatformResult<()> {
        if self.element.set_attribute_value(
            accessibility_attribute::position(),
            AXValueExt::from_point(CGPoint::new(x as f64, y as f64)),
        ) {
            Ok(())
        } else {
            Err(PlatformError::Error("Could not move window"))
        }
    }

    fn resize(&self, width: u32, height: u32) -> PlatformResult<()> {
        if self.element.set_attribute_value(
            accessibility_attribute::size(),
            AXValueExt::from_size(CGSize::new(width as f64, height as f64)),
        ) {
            Ok(())
        } else {
            Err(PlatformError::Error("Could not resize window"))
        }
    }
}
