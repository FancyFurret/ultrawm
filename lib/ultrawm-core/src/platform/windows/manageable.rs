use crate::platform::windows::WindowsPlatformWindow;
use crate::platform::{PlatformError, PlatformErrorType};
use std::mem::size_of;
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetWindowLongW, IsWindowVisible, GWL_EXSTYLE, GWL_STYLE, WS_CAPTION, WS_CHILD,
    WS_DISABLED, WS_EX_TOOLWINDOW, WS_OVERLAPPEDWINDOW,
};

pub fn window_is_manageable(window: &WindowsPlatformWindow) -> ObserveResult {
    let hwnd = window.hwnd();

    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            Err("Window is not visible")?
        }

        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        if style & WS_CHILD.0 != 0 {
            Err("Window is a child window")?
        }

        if style & WS_DISABLED.0 != 0 {
            Err("Window is disabled")?
        }

        if style & (WS_CAPTION.0 | WS_OVERLAPPEDWINDOW.0) == 0 {
            Err("Window has no title bar")?
        }

        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
            Err("Window is a tool window")?
        }

        let mut class_name: [u16; 256] = [0; 256];
        let len = GetClassNameW(hwnd, &mut class_name);
        let class_name = String::from_utf16_lossy(&class_name[..len as usize]);

        if class_name.trim() == "ApplicationFrameWindow" {
            let mut cloaked: u32 = 0;
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED,
                &mut cloaked as *mut _ as *mut _,
                size_of::<u32>() as u32,
            )
            .map_err(|_| "Could not get window attribute")?;

            if cloaked != 0 {
                Err("Window is cloaked")?
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
pub enum ObserveError {
    NotManageable(String),
    PlatformError(PlatformError),
}

pub type ObserveResult = Result<(), ObserveError>;

impl From<&str> for ObserveError {
    fn from(error: &str) -> Self {
        ObserveError::NotManageable(error.to_string())
    }
}

impl From<()> for ObserveError {
    fn from(_: ()) -> Self {
        ObserveError::PlatformError(PlatformErrorType::Unknown.into())
    }
}
