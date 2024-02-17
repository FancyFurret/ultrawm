pub use manageable::*;
pub use tile_preview::*;
pub use window::*;

use crate::platform::{
    Bounds, Display, DisplayId, EventDispatcher, PlatformEvent, PlatformImpl, PlatformInitImpl,
    PlatformResult, PlatformWindow, PlatformWindowImpl, Position, WindowId,
};
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EnumWindows, GetCursorPos, GetMessageW, TranslateMessage, EVENT_MAX,
    EVENT_MIN, EVENT_OBJECT_DESTROY, EVENT_OBJECT_FOCUS, EVENT_OBJECT_SHOW,
    EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MOVESIZESTART, MSG,
    OBJID_CLIENT, WINEVENT_OUTOFCONTEXT,
};

pub mod manageable;

mod tile_preview;
mod window;

static mut EVENT_DISPATCHER: Option<EventDispatcher> = None;

pub struct WindowsPlatformInit;

#[derive(Debug, Serialize, Deserialize)]
pub enum WindowsHookEvent {
    WindowCreated(WindowId),
    WindowDestroyed(WindowId),
    WindowFocused(WindowId),
    WindowMoved(WindowId),
    WindowResized(WindowId),
    WindowShown(WindowId),
    WindowHidden(WindowId),
}

unsafe impl PlatformInitImpl for WindowsPlatformInit {
    unsafe fn initialize() -> PlatformResult<()> {
        Ok(())
    }

    unsafe fn run_event_loop(dispatcher: EventDispatcher) -> PlatformResult<()> {
        println!("Current working directory: {:?}", std::env::current_dir());

        EVENT_DISPATCHER = Some(dispatcher);

        let hook = SetWinEventHook(
            EVENT_MIN,
            EVENT_MAX,
            None,
            Some(win_event_hook_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );
        if hook.0 == 0 {
            return Err("Could not set win event hook".into());
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        Ok(())
    }
}

impl WindowsPlatformInit {}

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

unsafe extern "system" fn win_event_hook_proc(
    _h_win_event_hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    if _id_object != OBJID_CLIENT.0 {
        return;
    }

    let window = WindowsPlatformWindow::new(hwnd).unwrap();

    let event = match event {
        EVENT_SYSTEM_MOVESIZESTART => PlatformEvent::WindowTransformStarted(window.clone()),
        EVENT_SYSTEM_MINIMIZESTART => PlatformEvent::WindowHidden(window.clone()),
        EVENT_SYSTEM_MINIMIZEEND => PlatformEvent::WindowShown(window.clone()),
        EVENT_OBJECT_SHOW => PlatformEvent::WindowShown(window.clone()),
        EVENT_OBJECT_FOCUS => PlatformEvent::WindowFocused(window.clone()),
        EVENT_OBJECT_DESTROY => PlatformEvent::WindowClosed(window.id()),
        _ => return,
    };

    // If it's a show event, make sure the window is manageable
    // The WM will automatically ignore unmanaged windows for other events
    if let PlatformEvent::WindowShown(window) = &event {
        if window_is_manageable(window).is_err() {
            return;
        }
    }

    EVENT_DISPATCHER.as_ref().unwrap().send(event);
}
