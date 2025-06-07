pub use manageable::*;
pub use tile_preview::*;
pub use window::*;

use crate::platform::{
    Bounds, Display, DisplayId, EventDispatcher, MouseButton, PlatformEvent, PlatformImpl,
    PlatformInitImpl, PlatformResult, PlatformWindow, PlatformWindowImpl, Position, WindowId,
};
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromPoint, HDC, HMONITOR, MONITORINFOEXW,
    MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::HiDpi::{
    GetDpiForMonitor, GetDpiForSystem, SetProcessDpiAwarenessContext,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, EnumWindows, GetCursorPos, GetMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, EVENT_MAX, EVENT_MIN, EVENT_OBJECT_DESTROY,
    EVENT_OBJECT_FOCUS, EVENT_OBJECT_SHOW, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART,
    EVENT_SYSTEM_MOVESIZESTART, HHOOK, MOUSEHOOKSTRUCT, MSG, OBJID_CLIENT, WH_MOUSE_LL,
    WINEVENT_OUTOFCONTEXT, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP,
    WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

pub mod manageable;

mod tile_preview;
mod window;

static mut EVENT_DISPATCHER: Option<EventDispatcher> = None;
static mut MOUSE_HOOK: Option<HHOOK> = None;

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
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
            .map_err(|e| format!("Failed to set DPI awareness: {:?}", e))?;
        Ok(())
    }

    unsafe fn run_event_loop(dispatcher: EventDispatcher) -> PlatformResult<()> {
        println!("Current working directory: {:?}", std::env::current_dir());

        EVENT_DISPATCHER = Some(dispatcher);

        // Set up hooks for specific events we care about
        let events = [
            EVENT_SYSTEM_MOVESIZESTART,
            EVENT_SYSTEM_MINIMIZESTART,
            EVENT_SYSTEM_MINIMIZEEND,
            EVENT_OBJECT_SHOW,
            EVENT_OBJECT_FOCUS,
            EVENT_OBJECT_DESTROY,
        ];

        let mut hooks = Vec::new();
        for event in events {
            let hook = SetWinEventHook(
                event,
                event, // Same event for min and max to only hook this specific event
                None,
                Some(win_event_hook_proc),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if hook.0 == 0 {
                return Err(format!("Could not set win event hook for event {}", event).into());
            }
            hooks.push(hook);
        }

        // Set up low-level mouse hook
        let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0)
            .map_err(|e| format!("Could not set mouse hook: {:?}", e))?;
        MOUSE_HOOK = Some(mouse_hook);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Clean up hooks
        if let Some(hook) = MOUSE_HOOK {
            UnhookWindowsHookEx(hook);
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
    let window = WindowsPlatformWindow::new(hwnd).unwrap();

    let event = match event {
        EVENT_SYSTEM_MOVESIZESTART => PlatformEvent::WindowTransformStarted(window.id()),
        EVENT_SYSTEM_MINIMIZESTART => PlatformEvent::WindowHidden(window.id()),
        EVENT_SYSTEM_MINIMIZEEND => PlatformEvent::WindowShown(window.id()),
        EVENT_OBJECT_SHOW => PlatformEvent::WindowOpened(window.clone()),
        EVENT_OBJECT_FOCUS => PlatformEvent::WindowFocused(window.id()),
        EVENT_OBJECT_DESTROY => PlatformEvent::WindowClosed(window.id()),
        _ => return,
    };

    // If it's a show event, make sure the window is manageable
    // The WM will automatically ignore unmanaged windows for other events
    if let PlatformEvent::WindowOpened(window) = &event {
        if window_is_manageable(window).is_err() {
            return;
        }
    }

    // println!("Dispatching event: {:?}", event);

    EVENT_DISPATCHER.as_ref().unwrap().send(event);
}

unsafe extern "system" fn mouse_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        // Use GetCursorPos to get the logical position of the mouse
        let mut position = POINT::default();
        let _ = GetCursorPos(&mut position);
        let position = Position::new(position.x, position.y);

        let event = match w_param.0 as u32 {
            WM_LBUTTONDOWN => PlatformEvent::MouseDown(position, MouseButton::Left),
            WM_LBUTTONUP => PlatformEvent::MouseUp(position, MouseButton::Left),
            WM_RBUTTONDOWN => PlatformEvent::MouseDown(position, MouseButton::Right),
            WM_RBUTTONUP => PlatformEvent::MouseUp(position, MouseButton::Right),
            WM_MBUTTONDOWN => PlatformEvent::MouseDown(position, MouseButton::Middle),
            WM_MBUTTONUP => PlatformEvent::MouseUp(position, MouseButton::Middle),
            WM_MOUSEMOVE => PlatformEvent::MouseMoved(position),
            _ => return unsafe { CallNextHookEx(None, n_code, w_param, l_param) },
        };

        EVENT_DISPATCHER.as_ref().unwrap().send(event);
    }

    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}
