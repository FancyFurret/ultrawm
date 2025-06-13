use crate::platform::windows::{window_is_manageable, WindowsPlatformWindow};
use crate::platform::{EventDispatcher, MouseButton, PlatformEvent, PlatformEventsImpl, PlatformResult, PlatformWindowImpl, Position, WindowId};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows::Win32::UI::WindowsAndMessaging::{CallNextHookEx, GetCursorPos, SetWindowsHookExW, EVENT_OBJECT_DESTROY, EVENT_OBJECT_FOCUS, EVENT_OBJECT_SHOW, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MOVESIZESTART, HHOOK, WH_MOUSE_LL, WINEVENT_OUTOFCONTEXT, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP};

static EVENT_DISPATCHER: OnceLock<EventDispatcher> = OnceLock::new();
static mut MOUSE_HOOK: Option<HHOOK> = None;

pub struct WindowsPlatformEvents;

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

unsafe impl PlatformEventsImpl for WindowsPlatformEvents {
    unsafe fn initialize(dispatcher: EventDispatcher) -> PlatformResult<()> {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
            .map_err(|e| format!("Failed to set DPI awareness: {:?}", e))?;

        let _ = EVENT_DISPATCHER.set(dispatcher);

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

        Ok(())
    }
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

    EVENT_DISPATCHER.get().unwrap().send(event);
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

        EVENT_DISPATCHER.get().unwrap().send(event);
    }

    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}
