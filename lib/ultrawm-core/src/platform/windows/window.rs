use crate::platform::{
    Bounds, PlatformResult, PlatformWindowImpl, Position, ProcessId, Size, WindowId,
};
use std::mem;
use windows::core::w;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowInfo, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsIconic, SetWindowPos,
    ShowWindow, SWP_NOSENDCHANGING, SWP_NOZORDER, SW_RESTORE, WINDOWINFO, WS_VISIBLE,
};

#[derive(Debug, Clone)]
pub struct WindowsPlatformWindow {
    hwnd: HWND,
}

impl WindowsPlatformWindow {
    pub fn new(hwnd: HWND) -> PlatformResult<Self> {
        Ok(Self { hwnd })
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Gets the visible window bounds, excluding invisible resize borders
    fn get_visible_bounds(&self) -> PlatformResult<RECT> {
        let mut rect = RECT::default();

        // Try to get the extended frame bounds (visible bounds) first
        unsafe {
            if DwmGetWindowAttribute(
                self.hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut rect as *mut _ as *mut _,
                mem::size_of::<RECT>() as u32,
            )
            .is_ok()
            {
                return Ok(rect);
            }
        }

        // Fall back to GetWindowRect if DwmGetWindowAttribute fails
        unsafe {
            GetWindowRect(self.hwnd, &mut rect).map_err(|_| "Could not get window bounds")?;
        }

        Ok(rect)
    }

    /// Calculates the border offsets between GetWindowRect and DwmGetWindowAttribute
    /// Returns (left_offset, top_offset, right_offset, bottom_offset)
    fn get_border_offsets(&self) -> (i32, i32, i32, i32) {
        let mut window_rect = RECT::default();
        let mut extended_rect = RECT::default();

        unsafe {
            // Get the full window rect (including invisible borders)
            if GetWindowRect(self.hwnd, &mut window_rect).is_err() {
                return (0, 0, 0, 0);
            }

            // Get the visible frame bounds
            if DwmGetWindowAttribute(
                self.hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut extended_rect as *mut _ as *mut _,
                mem::size_of::<RECT>() as u32,
            )
            .is_err()
            {
                return (0, 0, 0, 0);
            }
        }

        // Calculate the border differences on all sides
        let left_offset = extended_rect.left - window_rect.left;
        let top_offset = extended_rect.top - window_rect.top;
        let right_offset = window_rect.right - extended_rect.right;
        let bottom_offset = window_rect.bottom - extended_rect.bottom;

        (left_offset, top_offset, right_offset, bottom_offset)
    }
}

impl PlatformWindowImpl for WindowsPlatformWindow {
    fn id(&self) -> WindowId {
        self.hwnd.0 as WindowId
    }

    fn pid(&self) -> ProcessId {
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(self.hwnd, Some(&mut pid));
        }
        pid as ProcessId
    }

    fn title(&self) -> String {
        let mut text: [u16; 512] = [0; 512];
        let len = unsafe { GetWindowTextW(self.hwnd, &mut text) };

        String::from_utf16_lossy(&text[..len as usize])
    }

    fn position(&self) -> Position {
        let rect = self
            .get_visible_bounds()
            .expect("Could not get window position");
        Position {
            x: rect.left,
            y: rect.top,
        }
    }

    fn size(&self) -> Size {
        let rect = self
            .get_visible_bounds()
            .expect("Could not get window size");
        Size {
            width: (rect.right - rect.left) as u32,
            height: (rect.bottom - rect.top) as u32,
        }
    }

    fn visible(&self) -> bool {
        unsafe { !IsIconic(self.hwnd).as_bool() }
    }

    fn set_bounds(&self, bounds: &Bounds) -> PlatformResult<()> {
        unsafe {
            // First restore the window if it's maximized
            ShowWindow(self.hwnd, SW_RESTORE);

            // Get the border offsets to compensate for invisible borders
            let (left_offset, top_offset, right_offset, bottom_offset) = self.get_border_offsets();
            let adjusted_x = bounds.position.x - left_offset;
            let adjusted_y = bounds.position.y - top_offset;
            let adjusted_width = bounds.size.width as i32 + left_offset + right_offset;
            let adjusted_height = bounds.size.height as i32 + top_offset + bottom_offset;

            SetWindowPos(
                self.hwnd,
                None,
                adjusted_x,
                adjusted_y,
                adjusted_width,
                adjusted_height,
                SWP_NOZORDER,
            )
            .map_err(|_| "Could not set window bounds")?;
        }

        Ok(())
    }
}
