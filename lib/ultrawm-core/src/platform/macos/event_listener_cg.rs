use crate::platform::macos::ffi::run_loop_mode;
use crate::platform::{EventDispatcher, MouseButton, PlatformEvent, PlatformResult, Position};
use core_foundation::runloop::CFRunLoop;
use core_graphics::event::{
    CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    EventField,
};

pub struct EventListenerCG {
    _event_tap: CGEventTap<'static>,
}

impl EventListenerCG {
    pub fn run(dispatcher: EventDispatcher) -> PlatformResult<Self> {
        let mask = vec![
            CGEventType::MouseMoved,
            CGEventType::LeftMouseDown,
            CGEventType::LeftMouseUp,
            CGEventType::RightMouseDown,
            CGEventType::RightMouseUp,
            CGEventType::OtherMouseUp,
            CGEventType::OtherMouseDown,
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
            x: location.x as u32,
            y: location.y as u32,
        };

        let e = match event_type {
            CGEventType::MouseMoved => PlatformEvent::MouseMoved(position),
            CGEventType::LeftMouseDown => PlatformEvent::MouseDown(position, MouseButton::Left),
            CGEventType::LeftMouseUp => PlatformEvent::MouseUp(position, MouseButton::Left),
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
            _ => return,
        };

        dispatcher.send(e);
    }
}
