use crate::platform::macos::ffi::run_loop_mode;
use crate::platform::{EventDispatcher, MouseButton, PlatformEvent, PlatformResult, Position};
use core_foundation::runloop::CFRunLoop;
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, EventField,
};
use std::sync::atomic::{AtomicU64, Ordering};
use winit::keyboard::KeyCode;

pub struct EventListenerCG {
    _event_tap: CGEventTap<'static>,
}

// Track the current modifier flags state
static CURRENT_MODIFIER_FLAGS: AtomicU64 = AtomicU64::new(0);

impl EventListenerCG {
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
            CGEventType::KeyDown,
            CGEventType::KeyUp,
            CGEventType::FlagsChanged,
        ];

        let tap = CGEventTap::new(
            CGEventTapLocation::Session,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            mask,
            move |_proxy, event_type, event| {
                Self::handle_event(&dispatcher, event_type, event);
                Some(event.clone())
            },
        )?;

        let loop_source = tap.mach_port.create_runloop_source(0)?;
        CFRunLoop::get_current().add_source(&loop_source, run_loop_mode::common_modes());

        tap.enable();

        Ok(Self { _event_tap: tap })
    }

    fn handle_event(dispatcher: &EventDispatcher, event_type: CGEventType, event: &CGEvent) {
        let location = event.location();
        let position = Position {
            x: location.x as i32,
            y: location.y as i32,
        };

        let e = match event_type {
            CGEventType::MouseMoved => PlatformEvent::MouseMoved(position),
            CGEventType::LeftMouseDown => PlatformEvent::MouseDown(position, MouseButton::Left),
            CGEventType::LeftMouseUp => PlatformEvent::MouseUp(position, MouseButton::Left),
            CGEventType::LeftMouseDragged => PlatformEvent::MouseMoved(position),
            CGEventType::RightMouseDown => PlatformEvent::MouseDown(position, MouseButton::Right),
            CGEventType::RightMouseUp => PlatformEvent::MouseUp(position, MouseButton::Right),
            CGEventType::OtherMouseDown => {
                let button = event.get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER);
                if button == 2 {
                    PlatformEvent::MouseDown(position, MouseButton::Middle)
                } else {
                    return;
                }
            }
            CGEventType::OtherMouseUp => {
                let button = event.get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER);
                if button == 2 {
                    PlatformEvent::MouseUp(position, MouseButton::Middle)
                } else {
                    return;
                }
            }
            CGEventType::KeyDown => {
                let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                if let Some(keycode) = map_cg_keycode_to_winit(keycode as u16) {
                    PlatformEvent::KeyDown(keycode)
                } else {
                    return;
                }
            }
            CGEventType::KeyUp => {
                let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                if let Some(keycode) = map_cg_keycode_to_winit(keycode as u16) {
                    PlatformEvent::KeyUp(keycode)
                } else {
                    return;
                }
            }
            CGEventType::FlagsChanged => {
                Self::handle_flags_changed(dispatcher, event);
                return;
            }
            _ => return,
        };

        dispatcher.send(e);
    }

    fn handle_flags_changed(dispatcher: &EventDispatcher, event: &CGEvent) {
        let new_flags = event.get_flags().bits();
        let old_flags = CURRENT_MODIFIER_FLAGS.load(Ordering::SeqCst);

        CURRENT_MODIFIER_FLAGS.store(new_flags, Ordering::SeqCst);

        let changed_flags = new_flags ^ old_flags;

        if changed_flags & CGEventFlags::CGEventFlagControl.bits() != 0 {
            if new_flags & CGEventFlags::CGEventFlagControl.bits() != 0 {
                dispatcher.send(PlatformEvent::KeyDown(KeyCode::ControlLeft));
            } else {
                dispatcher.send(PlatformEvent::KeyUp(KeyCode::ControlLeft));
            }
        }

        if changed_flags & CGEventFlags::CGEventFlagShift.bits() != 0 {
            if new_flags & CGEventFlags::CGEventFlagShift.bits() != 0 {
                dispatcher.send(PlatformEvent::KeyDown(KeyCode::ShiftLeft));
            } else {
                dispatcher.send(PlatformEvent::KeyUp(KeyCode::ShiftLeft));
            }
        }

        if changed_flags & CGEventFlags::CGEventFlagAlternate.bits() != 0 {
            if new_flags & CGEventFlags::CGEventFlagAlternate.bits() != 0 {
                dispatcher.send(PlatformEvent::KeyDown(KeyCode::AltLeft));
            } else {
                dispatcher.send(PlatformEvent::KeyUp(KeyCode::AltLeft));
            }
        }

        if changed_flags & CGEventFlags::CGEventFlagCommand.bits() != 0 {
            if new_flags & CGEventFlags::CGEventFlagCommand.bits() != 0 {
                dispatcher.send(PlatformEvent::KeyDown(KeyCode::SuperLeft));
            } else {
                dispatcher.send(PlatformEvent::KeyUp(KeyCode::SuperLeft));
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
