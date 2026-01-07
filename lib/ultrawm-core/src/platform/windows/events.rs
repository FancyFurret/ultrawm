use crate::platform::inteceptor::Interceptor;
use crate::platform::windows::{window_is_manageable, WindowsPlatformWindow};
use crate::platform::{
    EventDispatcher, MouseButton, PlatformEventsImpl, PlatformResult, PlatformWindowImpl, Position,
    WMEvent, WindowId,
};
use log::warn;
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetCursorPos, SetWindowsHookExW, UnhookWindowsHookEx, EVENT_OBJECT_DESTROY,
    EVENT_OBJECT_FOCUS, EVENT_OBJECT_SHOW, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART,
    EVENT_SYSTEM_MOVESIZESTART, HHOOK, KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, WH_KEYBOARD_LL,
    WH_MOUSE_LL, WINEVENT_OUTOFCONTEXT, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN,
    WM_SYSKEYUP, WM_XBUTTONDOWN, WM_XBUTTONUP, XBUTTON1, XBUTTON2,
};
use winit::keyboard::KeyCode;

static EVENT_DISPATCHER: OnceLock<EventDispatcher> = OnceLock::new();
static WIN_EVENT_HOOKS: Mutex<Vec<isize>> = Mutex::new(Vec::new());
static LOW_LEVEL_HOOKS: Mutex<Vec<isize>> = Mutex::new(Vec::new());

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
            if hook.0.is_null() {
                return Err(format!("Could not set win event hook for event {}", event).into());
            }
            WIN_EVENT_HOOKS.lock().unwrap().push(hook.0 as isize);
        }

        // Set up low-level mouse hook
        let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0)
            .map_err(|e| format!("Could not set mouse hook: {:?}", e))?;
        LOW_LEVEL_HOOKS.lock().unwrap().push(mouse_hook.0 as isize);

        // Set up low-level keyboard hook
        let keyboard_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)
            .map_err(|e| format!("Could not set keyboard hook: {:?}", e))?;
        LOW_LEVEL_HOOKS
            .lock()
            .unwrap()
            .push(keyboard_hook.0 as isize);

        Ok(())
    }

    unsafe fn finalize() -> PlatformResult<()> {
        let mut errors = Vec::new();

        let mut win_hooks = WIN_EVENT_HOOKS.lock().unwrap();
        for &hook in win_hooks.iter() {
            if UnhookWinEvent(HWINEVENTHOOK(hook as *mut _)).0 == 0 {
                errors.push(format!("Failed to unhook WinEvent hook {:?}", hook));
            }
        }
        win_hooks.clear();

        let mut low_level_hooks = LOW_LEVEL_HOOKS.lock().unwrap();
        for &hook in low_level_hooks.iter() {
            if UnhookWindowsHookEx(HHOOK(hook as *mut _)).is_err() {
                errors.push(format!("Failed to unhook low-level hook {:?}", hook));
            }
        }
        low_level_hooks.clear();

        if !errors.is_empty() {
            return Err(format!(
                "Failed to cleanup {} hooks: {}",
                errors.len(),
                errors.join(", ")
            )
            .into());
        }

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
        EVENT_SYSTEM_MOVESIZESTART => WMEvent::WindowTransformStarted(window.id()),
        EVENT_SYSTEM_MINIMIZESTART => WMEvent::WindowClosed(window.id()),
        EVENT_SYSTEM_MINIMIZEEND => WMEvent::WindowOpened(window.clone()),
        EVENT_OBJECT_SHOW => WMEvent::WindowOpened(window.clone()),
        EVENT_OBJECT_FOCUS => WMEvent::WindowFocused(window.id()),
        EVENT_OBJECT_DESTROY => WMEvent::WindowClosed(window.id()),
        _ => return,
    };

    // If it's a show event, make sure the window is manageable
    // The WM will automatically ignore unmanaged windows for other events
    if let WMEvent::WindowOpened(window) = &event {
        if window_is_manageable(window).is_err() {
            return;
        }
    }

    EVENT_DISPATCHER.get().unwrap().send(event);
}

unsafe extern "system" fn mouse_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code < 0 {
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    // Use GetCursorPos to get the logical position of the mouse
    let mut position = POINT::default();
    let _ = GetCursorPos(&mut position);
    let position = Position::new(position.x, position.y);

    let event = match w_param.0 as u32 {
        WM_LBUTTONDOWN => WMEvent::MouseDown(position, MouseButton::Left),
        WM_LBUTTONUP => WMEvent::MouseUp(position, MouseButton::Left),
        WM_RBUTTONDOWN => WMEvent::MouseDown(position, MouseButton::Right),
        WM_RBUTTONUP => WMEvent::MouseUp(position, MouseButton::Right),
        WM_MBUTTONDOWN => WMEvent::MouseDown(position, MouseButton::Middle),
        WM_MBUTTONUP => WMEvent::MouseUp(position, MouseButton::Middle),
        WM_XBUTTONDOWN => {
            if let Some(button) = map_xbutton_to_button(l_param) {
                WMEvent::MouseDown(position, button)
            } else {
                return CallNextHookEx(None, n_code, w_param, l_param);
            }
        }
        WM_XBUTTONUP => {
            if let Some(button) = map_xbutton_to_button(l_param) {
                WMEvent::MouseUp(position, button)
            } else {
                return CallNextHookEx(None, n_code, w_param, l_param);
            }
        }
        WM_MOUSEMOVE => WMEvent::MouseMoved(position),
        _ => {
            return CallNextHookEx(None, n_code, w_param, l_param);
        }
    };

    // Check if we should ignore this event due to simulated click
    let button = match &event {
        WMEvent::MouseDown(_, button) | WMEvent::MouseUp(_, button) => Some(button.clone()),
        _ => None,
    };

    // Check if we should ignore this event due to simulated click
    if let Some(button) = button {
        if Interceptor::pop_ignore_click(
            button,
            matches!(event, WMEvent::MouseUp(_, _)),
        ) {
            return CallNextHookEx(None, n_code, w_param, l_param);
        }
    }

    EVENT_DISPATCHER.get().unwrap().send(event);

    // Check if we should intercept this specific button before calling next hook
    if let Some(button) = button.as_ref() {
        if Interceptor::should_intercept_button(&button) {
            return LRESULT(1);
        }
    }

    CallNextHookEx(None, n_code, w_param, l_param)
}

unsafe extern "system" fn keyboard_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let kb_struct = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
        let vk_code = kb_struct.vkCode as u32;
        let keycode = map_vk_to_keycode(vk_code);
        if let Some(keycode) = keycode {
            let event = match w_param.0 as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => WMEvent::KeyDown(keycode),
                WM_KEYUP | WM_SYSKEYUP => WMEvent::KeyUp(keycode),
                _ => return unsafe { CallNextHookEx(None, n_code, w_param, l_param) },
            };
            EVENT_DISPATCHER.get().unwrap().send(event);
        }
    }
    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

fn map_xbutton_to_button(l_param: LPARAM) -> Option<MouseButton> {
    unsafe {
        let hook_struct = &*(l_param.0 as *const MSLLHOOKSTRUCT);
        let mouse_data = hook_struct.mouseData;
        let button_id = ((mouse_data >> 16) & 0xFFFF) as u16;

        match button_id {
            XBUTTON1 => Some(MouseButton::Button4),
            XBUTTON2 => Some(MouseButton::Button5),
            _ => {
                warn!("Unknown xbutton id: {}", button_id);
                None
            }
        }
    }
}

fn map_vk_to_keycode(vk: u32) -> Option<KeyCode> {
    use win_key_codes::*;
    use winit::keyboard::KeyCode::*;
    match vk as i32 {
        VK_A => Some(KeyA),
        VK_B => Some(KeyB),
        VK_C => Some(KeyC),
        VK_D => Some(KeyD),
        VK_E => Some(KeyE),
        VK_F => Some(KeyF),
        VK_G => Some(KeyG),
        VK_H => Some(KeyH),
        VK_I => Some(KeyI),
        VK_J => Some(KeyJ),
        VK_K => Some(KeyK),
        VK_L => Some(KeyL),
        VK_M => Some(KeyM),
        VK_N => Some(KeyN),
        VK_O => Some(KeyO),
        VK_P => Some(KeyP),
        VK_Q => Some(KeyQ),
        VK_R => Some(KeyR),
        VK_S => Some(KeyS),
        VK_T => Some(KeyT),
        VK_U => Some(KeyU),
        VK_V => Some(KeyV),
        VK_W => Some(KeyW),
        VK_X => Some(KeyX),
        VK_Y => Some(KeyY),
        VK_Z => Some(KeyZ),
        VK_0 => Some(Digit0),
        VK_1 => Some(Digit1),
        VK_2 => Some(Digit2),
        VK_3 => Some(Digit3),
        VK_4 => Some(Digit4),
        VK_5 => Some(Digit5),
        VK_6 => Some(Digit6),
        VK_7 => Some(Digit7),
        VK_8 => Some(Digit8),
        VK_9 => Some(Digit9),
        VK_F1 => Some(F1),
        VK_F2 => Some(F2),
        VK_F3 => Some(F3),
        VK_F4 => Some(F4),
        VK_F5 => Some(F5),
        VK_F6 => Some(F6),
        VK_F7 => Some(F7),
        VK_F8 => Some(F8),
        VK_F9 => Some(F9),
        VK_F10 => Some(F10),
        VK_F11 => Some(F11),
        VK_F12 => Some(F12),
        VK_ESCAPE => Some(Escape),
        VK_TAB => Some(Tab),
        VK_RETURN => Some(Enter),
        VK_BACK => Some(Backspace),
        VK_DELETE => Some(Delete),
        VK_INSERT => Some(Insert),
        VK_HOME => Some(Home),
        VK_END => Some(End),
        VK_UP => Some(ArrowUp),
        VK_DOWN => Some(ArrowDown),
        VK_LEFT => Some(ArrowLeft),
        VK_RIGHT => Some(ArrowRight),
        VK_SHIFT => Some(ShiftLeft),
        VK_LSHIFT => Some(ShiftLeft),
        VK_RSHIFT => Some(ShiftRight),
        VK_CONTROL => Some(ControlLeft),
        VK_LCONTROL => Some(ControlLeft),
        VK_RCONTROL => Some(ControlRight),
        VK_MENU => Some(AltLeft),
        VK_LMENU => Some(AltLeft),
        VK_RMENU => Some(AltRight),
        VK_LWIN => Some(SuperLeft),
        VK_RWIN => Some(SuperRight),
        _ => None,
    }
}
