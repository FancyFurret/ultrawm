use crate::platform::inteceptor::Interceptor;
use crate::platform::macos::ffi::run_loop_mode;
use crate::platform::macos::platform::MacOSPlatform;
use crate::platform::{EventDispatcher, MouseButton, PlatformResult, Position, WMEvent};
use core_foundation::runloop::{CFRunLoop, CFRunLoopSource};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, CallbackResult, EventField,
};
use std::sync::atomic::{AtomicU64, Ordering};
use winit::keyboard::KeyCode;

pub struct EventListenerCG {
    _event_tap: CGEventTap<'static>,
    _source: CFRunLoopSource,
}

// Track the current modifier flags state
static CURRENT_MODIFIER_FLAGS: AtomicU64 = AtomicU64::new(0);

impl EventListenerCG {
    // Helper function to get keyboard keycode from event
    fn get_keyboard_keycode(event: &CGEvent) -> i64 {
        event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE)
    }

    // Helper function to get mouse button from event
    fn get_mouse_button_from_event(event: &CGEvent) -> Option<MouseButton> {
        let button_number = event.get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER);
        match button_number {
            2 => Some(MouseButton::Middle),
            3 => Some(MouseButton::Button4),
            4 => Some(MouseButton::Button5),
            _ => None,
        }
    }

    pub fn run(dispatcher: EventDispatcher) -> PlatformResult<Self> {
        let mask = vec![
            CGEventType::MouseMoved,
            CGEventType::LeftMouseDown,
            CGEventType::LeftMouseUp,
            CGEventType::LeftMouseDragged,
            CGEventType::RightMouseDown,
            CGEventType::RightMouseUp,
            CGEventType::RightMouseDragged,
            CGEventType::OtherMouseUp,
            CGEventType::OtherMouseDown,
            CGEventType::OtherMouseDragged,
            CGEventType::KeyDown,
            CGEventType::KeyUp,
            CGEventType::FlagsChanged,
        ];

        let tap = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            mask,
            move |_proxy, event_type, event| {
                if Self::handle_event(&dispatcher, event_type, event) {
                    return CallbackResult::Drop;
                }
                CallbackResult::Keep
            },
        )?;

        let loop_source = tap.mach_port().create_runloop_source(0)?;
        CFRunLoop::get_current().add_source(&loop_source, run_loop_mode::common_modes());

        tap.enable();

        Ok(Self {
            _event_tap: tap,
            _source: loop_source,
        })
    }

    fn handle_event(
        dispatcher: &EventDispatcher,
        event_type: CGEventType,
        event: &CGEvent,
    ) -> bool {
        let location = event.location();
        let y_offset = MacOSPlatform::get_cgevent_y_offset();
        let position = Position {
            x: location.x as i32,
            y: location.y as i32 + y_offset,
        };

        let (e, button) = match event_type {
            CGEventType::MouseMoved => (WMEvent::MouseMoved(position), None),
            CGEventType::LeftMouseDown => (
                WMEvent::MouseDown(position, MouseButton::Left),
                Some(MouseButton::Left),
            ),
            CGEventType::LeftMouseUp => (
                WMEvent::MouseUp(position, MouseButton::Left),
                Some(MouseButton::Left),
            ),
            CGEventType::LeftMouseDragged => (WMEvent::MouseMoved(position), None),
            CGEventType::RightMouseDown => (
                WMEvent::MouseDown(position, MouseButton::Right),
                Some(MouseButton::Right),
            ),
            CGEventType::RightMouseUp => (
                WMEvent::MouseUp(position, MouseButton::Right),
                Some(MouseButton::Right),
            ),
            CGEventType::RightMouseDragged => (WMEvent::MouseMoved(position), None),
            CGEventType::OtherMouseDown => {
                if let Some(mouse_button) = Self::get_mouse_button_from_event(event) {
                    (
                        WMEvent::MouseDown(position, mouse_button.clone()),
                        Some(mouse_button),
                    )
                } else {
                    return false;
                }
            }
            CGEventType::OtherMouseUp => {
                if let Some(mouse_button) = Self::get_mouse_button_from_event(event) {
                    (
                        WMEvent::MouseUp(position, mouse_button.clone()),
                        Some(mouse_button),
                    )
                } else {
                    return false;
                }
            }
            CGEventType::OtherMouseDragged => {
                if Self::get_mouse_button_from_event(event).is_some() {
                    (WMEvent::MouseMoved(position), None)
                } else {
                    return false;
                }
            }
            CGEventType::KeyDown => {
                let keycode = Self::get_keyboard_keycode(event);
                if let Some(keycode) = map_cg_keycode_to_winit(keycode as u16) {
                    (WMEvent::KeyDown(keycode), None)
                } else {
                    return false;
                }
            }
            CGEventType::KeyUp => {
                let keycode = Self::get_keyboard_keycode(event);
                if let Some(keycode) = map_cg_keycode_to_winit(keycode as u16) {
                    (WMEvent::KeyUp(keycode), None)
                } else {
                    return false;
                }
            }
            CGEventType::FlagsChanged => {
                Self::handle_flags_changed(dispatcher, event);
                return false;
            }
            _ => return false,
        };

        if let Some(button) = button.clone() {
            if matches!(e, WMEvent::MouseDown(_, _) | WMEvent::MouseUp(_, _)) {
                if Interceptor::pop_ignore_click(
                    button,
                    matches!(e, WMEvent::MouseUp(_, _)),
                ) {
                    return false;
                }
            }
        }

        dispatcher.send(e);

        if let Some(button) = button {
            if Interceptor::should_intercept_button(&button) {
                return true;
            }
        }

        false
    }

    fn handle_flags_changed(dispatcher: &EventDispatcher, event: &CGEvent) {
        let new_flags = event.get_flags().bits();
        let old_flags = CURRENT_MODIFIER_FLAGS.load(Ordering::SeqCst);

        CURRENT_MODIFIER_FLAGS.store(new_flags, Ordering::SeqCst);

        let changed_flags = new_flags ^ old_flags;

        if changed_flags & CGEventFlags::CGEventFlagControl.bits() != 0 {
            if new_flags & CGEventFlags::CGEventFlagControl.bits() != 0 {
                dispatcher.send(WMEvent::KeyDown(KeyCode::ControlLeft));
            } else {
                dispatcher.send(WMEvent::KeyUp(KeyCode::ControlLeft));
            }
        }

        if changed_flags & CGEventFlags::CGEventFlagShift.bits() != 0 {
            if new_flags & CGEventFlags::CGEventFlagShift.bits() != 0 {
                dispatcher.send(WMEvent::KeyDown(KeyCode::ShiftLeft));
            } else {
                dispatcher.send(WMEvent::KeyUp(KeyCode::ShiftLeft));
            }
        }

        if changed_flags & CGEventFlags::CGEventFlagAlternate.bits() != 0 {
            if new_flags & CGEventFlags::CGEventFlagAlternate.bits() != 0 {
                dispatcher.send(WMEvent::KeyDown(KeyCode::AltLeft));
            } else {
                dispatcher.send(WMEvent::KeyUp(KeyCode::AltLeft));
            }
        }

        if changed_flags & CGEventFlags::CGEventFlagCommand.bits() != 0 {
            if new_flags & CGEventFlags::CGEventFlagCommand.bits() != 0 {
                dispatcher.send(WMEvent::KeyDown(KeyCode::SuperLeft));
            } else {
                dispatcher.send(WMEvent::KeyUp(KeyCode::SuperLeft));
            }
        }
    }
}

fn map_cg_keycode_to_winit(cg_keycode: u16) -> Option<KeyCode> {
    use winit::keyboard::KeyCode::*;

    match cg_keycode {
        0 => Some(KeyA),
        1 => Some(KeyS),
        2 => Some(KeyD),
        3 => Some(KeyF),
        4 => Some(KeyH),
        5 => Some(KeyG),
        6 => Some(KeyZ),
        7 => Some(KeyX),
        8 => Some(KeyC),
        9 => Some(KeyV),
        11 => Some(KeyB),
        12 => Some(KeyQ),
        13 => Some(KeyW),
        14 => Some(KeyE),
        15 => Some(KeyR),
        16 => Some(KeyY),
        17 => Some(KeyT),
        18 => Some(Digit1),
        19 => Some(Digit2),
        20 => Some(Digit3),
        21 => Some(Digit4),
        22 => Some(Digit6),
        23 => Some(Digit5),
        24 => Some(Equal),
        25 => Some(Digit9),
        26 => Some(Digit7),
        27 => Some(Minus),
        28 => Some(Digit8),
        29 => Some(Digit0),
        30 => Some(BracketRight),
        31 => Some(KeyO),
        32 => Some(KeyU),
        33 => Some(BracketLeft),
        34 => Some(KeyI),
        35 => Some(KeyP),
        37 => Some(KeyL),
        38 => Some(KeyJ),
        39 => Some(Quote),
        40 => Some(KeyK),
        41 => Some(Semicolon),
        42 => Some(Backslash),
        43 => Some(Comma),
        44 => Some(Slash),
        45 => Some(KeyN),
        46 => Some(KeyM),
        47 => Some(Period),
        50 => Some(Backquote),
        65 => Some(Period),
        67 => Some(KeyM),
        69 => Some(NumpadAdd),
        71 => Some(NumLock),
        75 => Some(Slash),
        76 => Some(Enter),
        78 => Some(Minus),
        81 => Some(Equal),
        82 => Some(Digit0),
        83 => Some(Digit1),
        84 => Some(Digit2),
        85 => Some(Digit3),
        86 => Some(Digit4),
        87 => Some(Digit5),
        88 => Some(Digit6),
        89 => Some(Digit7),
        91 => Some(Digit8),
        92 => Some(Digit9),
        36 => Some(Enter),
        48 => Some(Tab),
        49 => Some(Space),
        51 => Some(Backspace),
        53 => Some(Escape),
        96 => Some(F5),
        97 => Some(F6),
        98 => Some(F7),
        99 => Some(F3),
        100 => Some(F8),
        101 => Some(F9),
        103 => Some(F11),
        105 => Some(F13),
        107 => Some(F14),
        109 => Some(F10),
        111 => Some(F12),
        114 => Some(Insert),
        115 => Some(Home),
        116 => Some(PageUp),
        117 => Some(Delete),
        118 => Some(F4),
        119 => Some(End),
        120 => Some(F2),
        121 => Some(PageDown),
        122 => Some(F1),
        123 => Some(ArrowLeft),
        124 => Some(ArrowRight),
        125 => Some(ArrowDown),
        126 => Some(ArrowUp),
        _ => None,
    }
}
