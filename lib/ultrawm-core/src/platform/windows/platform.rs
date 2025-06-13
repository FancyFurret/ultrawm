use crate::platform::windows::{window_is_manageable, WindowsPlatformWindow};
use crate::platform::{
    Bounds, Display, DisplayId, PlatformImpl, PlatformResult, PlatformWindow, Position,
};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetCursorPos};

pub struct WindowsPlatform;

impl PlatformImpl for WindowsPlatform {
    fn list_visible_windows() -> PlatformResult<Vec<PlatformWindow>> {
        let mut windows = Vec::new();

        unsafe {
            EnumWindows(Some(enum_window), LPARAM(&mut windows as *mut _ as isize)).unwrap();
        }

        Ok(windows)
    }

    fn list_all_displays() -> PlatformResult<Vec<Display>> {
        let mut displays = Vec::new();

        unsafe {
            EnumDisplayMonitors(
                None,
                None,
                Some(enum_display),
                LPARAM(&mut displays as *mut _ as isize),
            )
            .unwrap()
        }

        Ok(displays)
    }

    fn get_mouse_position() -> PlatformResult<Position> {
        let mut point = POINT::default();

        unsafe {
            GetCursorPos(&mut point).map_err(|err| err.to_string())?;
        }

        Ok(Position::new(point.x, point.y))
    }
}

extern "system" fn enum_window(window: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let windows = &mut *(lparam.0 as *mut Vec<PlatformWindow>);
        if let Ok(window) = WindowsPlatformWindow::new(window) {
            // This will also ensure the window is visible
            if !window_is_manageable(&window).is_ok() {
                return true.into();
            }

            windows.push(window);
        }

        true.into()
    }
}

extern "system" fn enum_display(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    unsafe {
        let displays = &mut *(lparam.0 as *mut Vec<Display>);

        let mut exinfo = MONITORINFOEXW::default();
        exinfo.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;

        if !GetMonitorInfoW(monitor, &mut exinfo as *mut _ as *mut _).as_bool() {
            return true.into();
        }

        let device_name_utf16 = exinfo
            .szDevice
            .iter()
            .take_while(|&c| *c != 0)
            .cloned()
            .collect::<Vec<u16>>();
        let device_name = String::from_utf16_lossy(&device_name_utf16);

        let info = exinfo.monitorInfo;
        let display = Display {
            id: monitor.0 as DisplayId,
            name: device_name,
            bounds: Bounds::new(
                info.rcMonitor.left,
                info.rcMonitor.top,
                (info.rcMonitor.right - info.rcMonitor.left) as u32,
                (info.rcMonitor.bottom - info.rcMonitor.top) as u32,
            ),
            work_area: Bounds::new(
                info.rcWork.left,
                info.rcWork.top,
                (info.rcWork.right - info.rcWork.left) as u32,
                (info.rcWork.bottom - info.rcWork.top) as u32,
            ),
        };

        displays.push(display);
    }

    true.into()
}
