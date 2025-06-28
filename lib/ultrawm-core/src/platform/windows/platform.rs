use crate::platform::inteceptor::Interceptor;
use crate::platform::windows::{window_is_manageable, WindowsPlatformWindow};
use crate::platform::{
    Bounds, CursorType, Display, DisplayId, MouseButton, PlatformImpl, PlatformResult,
    PlatformWindow, Position,
};
use std::sync::atomic::{AtomicI32, AtomicIsize, Ordering};
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BeginDeferWindowPos, CopyIcon, EndDeferWindowPos, EnumWindows, GetCursorPos, LoadCursorW,
    SetSystemCursor, SystemParametersInfoW, HCURSOR, HDWP, HICON, IDC_ARROW, IDC_IBEAM, IDC_NO,
    IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, IDC_WAIT, OCR_IBEAM, OCR_NO,
    OCR_NORMAL, OCR_SIZEALL, OCR_SIZENESW, OCR_SIZENS, OCR_SIZENWSE, OCR_SIZEWE, OCR_WAIT,
    SPIF_SENDCHANGE, SPI_SETCURSORS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
};

pub struct WindowsPlatform;

static CURRENT_CURSOR_TYPE: AtomicI32 = AtomicI32::new(-1);
pub static WINDOW_BATCH: AtomicIsize = AtomicIsize::new(0);

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

    fn set_cursor(cursor_type: CursorType) -> PlatformResult<()> {
        unsafe {
            let cursor_type_value = cursor_type as i32;
            let current_type = CURRENT_CURSOR_TYPE.load(Ordering::Relaxed);

            // Only update if cursor type has changed
            if cursor_type_value != current_type {
                // Get the cursor we want to use
                let cursor_id = match cursor_type {
                    CursorType::Normal => IDC_ARROW,
                    CursorType::ResizeNorth | CursorType::ResizeSouth => IDC_SIZENS,
                    CursorType::ResizeEast | CursorType::ResizeWest => IDC_SIZEWE,
                    CursorType::ResizeNorthEast | CursorType::ResizeSouthWest => IDC_SIZENESW,
                    CursorType::ResizeNorthWest | CursorType::ResizeSouthEast => IDC_SIZENWSE,
                    CursorType::Move => IDC_SIZEALL,
                    CursorType::IBeam => IDC_IBEAM,
                    CursorType::Wait => IDC_WAIT,
                    CursorType::NotAllowed => IDC_NO,
                };

                // Set ALL system cursors to our desired cursor
                let system_cursors = [
                    OCR_NORMAL,
                    OCR_IBEAM,
                    OCR_WAIT,
                    OCR_SIZENWSE,
                    OCR_SIZENESW,
                    OCR_SIZEWE,
                    OCR_SIZENS,
                    OCR_SIZEALL,
                    OCR_NO,
                ];

                let cursor = LoadCursorW(None, cursor_id)
                    .map_err(|e| format!("Failed to load cursor: {:?}", e))?;
                CURRENT_CURSOR_TYPE.store(cursor_type_value, Ordering::Relaxed);
                for system_cursor in system_cursors {
                    SetSystemCursor(copy_cursor(cursor).unwrap(), system_cursor)
                        .map_err(|e| format!("Failed to set system cursor: {:?}", e))?;
                }
            }
        }
        Ok(())
    }

    fn reset_cursor() -> PlatformResult<()> {
        unsafe {
            let cursor_type = CURRENT_CURSOR_TYPE.load(Ordering::Relaxed);
            if cursor_type == -1 {
                return Ok(());
            }

            SystemParametersInfoW(SPI_SETCURSORS, 0, None, SPIF_SENDCHANGE).unwrap();
            CURRENT_CURSOR_TYPE.store(-1, Ordering::Relaxed);
            Ok(())
        }
    }

    fn start_window_bounds_batch(window_count: u32) -> PlatformResult<()> {
        let hdswp = unsafe { BeginDeferWindowPos(window_count as i32) }.unwrap();
        WINDOW_BATCH.store(hdswp.0 as isize, Ordering::Relaxed);
        Ok(())
    }
    fn end_window_bounds_batch() -> PlatformResult<()> {
        let hdswp_val = WINDOW_BATCH.load(Ordering::Relaxed);
        if hdswp_val == 0 {
            return Ok(()); // No batch in progress
        }

        unsafe { EndDeferWindowPos(HDWP(hdswp_val as *mut _)) }.unwrap();
        WINDOW_BATCH.store(0, Ordering::Relaxed);
        Ok(())
    }

    fn simulate_mouse_click(position: Position, button: MouseButton) -> PlatformResult<()> {
        Interceptor::ignore_next_click(button.clone());

        unsafe {
            // Get screen dimensions for absolute positioning
            let screen_width = GetSystemMetrics(SM_CXSCREEN);
            let screen_height = GetSystemMetrics(SM_CYSCREEN);

            // Convert to absolute coordinates (0-65535 range)
            let abs_x = (position.x * 65536) / screen_width;
            let abs_y = (position.y * 65536) / screen_height;

            let (down_flag, up_flag, mouse_data) = match button {
                MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, 0),
                MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, 0),
                MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, 0),
                MouseButton::Button4 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, 0x0001),
                MouseButton::Button5 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, 0x0002),
            };

            // First move to position
            let move_input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: abs_x,
                        dy: abs_y,
                        mouseData: 0,
                        dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };

            // Mouse down
            let down_input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: mouse_data,
                        dwFlags: down_flag,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };

            // Mouse up
            let up_input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: mouse_data,
                        dwFlags: up_flag,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };

            let inputs = [move_input, down_input, up_input];
            let result = SendInput(&inputs, size_of::<INPUT>() as i32);

            if result != inputs.len() as u32 {
                return Err(format!(
                    "Failed to simulate mouse click. Expected {}, got {}",
                    inputs.len(),
                    result
                )
                .into());
            }
        }

        Ok(())
    }
}

unsafe fn get_foreground_window() -> Option<HWND> {
    let hwnd = GetForegroundWindow();
    if !hwnd.0.is_null() {
        Some(hwnd)
    } else {
        None
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

fn copy_cursor(cursor: HCURSOR) -> PlatformResult<HCURSOR> {
    unsafe { Ok(HCURSOR(CopyIcon(HICON(cursor.0)).unwrap().0)) }
}
