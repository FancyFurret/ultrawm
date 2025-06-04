use crate::platform::{
    Bounds, PlatformResult, PlatformWindowImpl, Position, ProcessId, Size, WindowId,
};
use windows::core::w;
use windows::Win32::Foundation::{HWND, RECT};
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
        let mut rect = RECT::default();
        unsafe { GetWindowRect(self.hwnd, &mut rect).expect("Could not get window position") }
        Position {
            x: rect.left,
            y: rect.top,
        }
    }

    fn size(&self) -> Size {
        let mut rect = RECT::default();
        unsafe { GetWindowRect(self.hwnd, &mut rect).expect("Could not get window size") }
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

            SetWindowPos(
                self.hwnd,
                None,
                bounds.position.x,
                bounds.position.y,
                bounds.size.width as i32,
                bounds.size.height as i32,
                SWP_NOZORDER,
            )
            .map_err(|_| "Could not set window bounds")?;
        }

        Ok(())
    }
}
