use crate::platform::inteceptor::Interceptor;
use crate::platform::macos::ffi::{window_info, AXUIElementExt, CFArrayExt, CFDictionaryExt};
use crate::platform::macos::ObserveError::NotManageable;
use crate::platform::macos::{app_is_manageable, window_is_manageable, MacOSPlatformWindow};
use crate::platform::{
    Bounds, CursorType, Display, MouseButton, PlatformError, PlatformImpl, PlatformResult,
    Position, ProcessId,
};
use application_services::accessibility_ui::AXUIElement;
use application_services::pid_t;
use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::window::{copy_window_info, kCGNullWindowID, kCGWindowListOptionAll};
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSDeviceDescriptionKey, NSEvent, NSScreen};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSNumber, NSRect};
use std::collections::HashSet;
use std::mem;
use std::sync::atomic::AtomicI32;
use std::sync::OnceLock;

pub struct MacOSPlatform;

static CURRENT_CURSOR_TYPE: AtomicI32 = AtomicI32::new(-1);
static CACHED_SCREENS: OnceLock<Vec<CachedScreen>> = OnceLock::new();

// TODO: Improve screens
#[derive(Debug, Clone)]
struct CachedScreen {
    id: u32,
    name: String,
    bounds: Bounds,
    work_area: Bounds,
    frame: NSRect,
}

impl MacOSPlatform {
    pub fn find_pids_with_windows() -> PlatformResult<HashSet<u32>> {
        let window_info = copy_window_info(kCGWindowListOptionAll, kCGNullWindowID);
        let window_info = window_info.ok_or("Could not get window info")?;
        let window_info = CFArrayExt::<CFDictionaryExt>::from(window_info);

        let mut pids = HashSet::new();
        for window in window_info {
            let pid = window
                .get_int(window_info::owner_pid())
                .ok_or("Could not get window pid")? as ProcessId;
            pids.insert(pid);
        }

        Ok(pids)
    }

    pub fn initialize_screens() -> PlatformResult<()> {
        if CACHED_SCREENS.get().is_some() {
            return Ok(());
        }

        unsafe {
            let mtm = MainThreadMarker::new().unwrap();
            let displays = NSScreen::screens(mtm);
            let mut result = Vec::new();

            for screen in displays {
                let desc = screen.deviceDescription();
                let key = NSDeviceDescriptionKey::from_str("NSScreenNumber");
                let obj = desc.objectForKey(&key).ok_or("Could not get screen id")?;
                let number = Retained::cast_unchecked::<NSNumber>(obj);

                let total_height = screen.frame().size.height as i32;

                result.push(CachedScreen {
                    id: number.unsignedIntegerValue() as u32,
                    name: screen.localizedName().to_string(),
                    bounds: Bounds::new(
                        screen.frame().origin.x as i32,
                        total_height
                            - screen.frame().origin.y as i32
                            - screen.frame().size.height as i32,
                        screen.frame().size.width as u32,
                        screen.frame().size.height as u32,
                    ),
                    work_area: Bounds::new(
                        screen.visibleFrame().origin.x as i32,
                        total_height
                            - screen.visibleFrame().origin.y as i32
                            - screen.visibleFrame().size.height as i32,
                        screen.visibleFrame().size.width as u32,
                        screen.visibleFrame().size.height as u32,
                    ),
                    frame: screen.frame(),
                });
            }

            CACHED_SCREENS
                .set(result)
                .map_err(|_| PlatformError::Error("Failed to cache screens".to_string()))?;
        }
        Ok(())
    }

    fn get_cached_screens() -> PlatformResult<&'static [CachedScreen]> {
        if let Some(screens) = CACHED_SCREENS.get() {
            Ok(screens)
        } else {
            Ok(CACHED_SCREENS.get().unwrap())
        }
    }

    fn get_screen_bounds_for_position(position: &Position) -> Option<Bounds> {
        let screens = Self::get_cached_screens().ok()?;
        for screen in screens {
            if position.x >= screen.bounds.position.x
                && position.x < screen.bounds.position.x + screen.bounds.size.width as i32
                && position.y >= screen.bounds.position.y
                && position.y < screen.bounds.position.y + screen.bounds.size.height as i32
            {
                return Some(screen.bounds.clone());
            }
        }
        None
    }
}

impl PlatformImpl for MacOSPlatform {
    fn list_visible_windows() -> PlatformResult<Vec<MacOSPlatformWindow>> {
        let mut windows = Vec::new();
        for pid in MacOSPlatform::find_pids_with_windows()? {
            let app = AXUIElementExt::from(
                AXUIElement::create_application(pid as pid_t)
                    .map_err(|_| format!("Could not create AXUIElement for pid {}", pid))?,
            );

            match app_is_manageable(&app) {
                Ok(_) => {}
                Err(NotManageable(_)) => continue,
                Err(e) => return Err(e.into()),
            }

            if let Ok(app_windows) = app.windows() {
                for window in app_windows {
                    match window_is_manageable(&window) {
                        Ok(_) => {}
                        Err(NotManageable(_)) => continue,
                        Err(e) => return Err(e.into()),
                    }

                    let window = MacOSPlatformWindow::new(window);
                    if let Ok(window) = window {
                        windows.push(window);
                    }
                }
            }
        }

        Ok(windows)
    }

    fn list_all_displays() -> PlatformResult<Vec<Display>> {
        let screens = Self::get_cached_screens()?;
        Ok(screens
            .iter()
            .map(|screen| Display {
                id: screen.id,
                name: screen.name.clone(),
                bounds: screen.bounds.clone(),
                work_area: screen.work_area.clone(),
            })
            .collect())
    }

    fn get_mouse_position() -> PlatformResult<Position> {
        // TODO: Slow?
        unsafe {
            let pos = NSEvent::mouseLocation();
            let position = Position::new(pos.x as i32, pos.y as i32);
            let screen = Self::get_screen_bounds_for_position(&position).unwrap();
            let total_height = screen.size.height as f64;
            Ok(Position::new(
                position.x,
                total_height as i32 - position.y - 1,
            ))
        }
    }

    fn set_cursor(_cursor_type: CursorType) -> PlatformResult<()> {
        // TODO
        Ok(())
    }

    fn reset_cursor() -> PlatformResult<()> {
        // TODO
        Ok(())
    }

    fn start_window_bounds_batch(_window_count: u32) -> PlatformResult<()> {
        // Not supported on macOS for now
        Ok(())
    }

    fn end_window_bounds_batch() -> PlatformResult<()> {
        // Not supported on macOS for now
        Ok(())
    }

    fn simulate_mouse_click(position: Position, button: MouseButton) -> PlatformResult<()> {
        Interceptor::ignore_next_click(button.clone());

        println!("Simulatint mouse click {:?}", button);

        unsafe {
            let screen_pos =
                core_graphics::geometry::CGPoint::new(position.x as f64, position.y as f64);

            let (event_type_down, event_type_up, cg_button) = match button {
                MouseButton::Left => (
                    CGEventType::LeftMouseDown,
                    CGEventType::LeftMouseUp,
                    CGMouseButton::Left as u32,
                ),
                MouseButton::Right => (
                    CGEventType::RightMouseDown,
                    CGEventType::RightMouseUp,
                    CGMouseButton::Right as u32,
                ),
                MouseButton::Middle => (
                    CGEventType::OtherMouseDown,
                    CGEventType::OtherMouseUp,
                    CGMouseButton::Center as u32,
                ),
                MouseButton::Button4 => (CGEventType::OtherMouseDown, CGEventType::OtherMouseUp, 3),
                MouseButton::Button5 => (CGEventType::OtherMouseDown, CGEventType::OtherMouseUp, 4),
            };

            // Create event source
            let event_source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)?;

            // Create mouse down event
            let down_event = CGEvent::new_mouse_event(
                event_source.clone(),
                event_type_down,
                screen_pos,
                mem::transmute(cg_button),
            )?;

            // Create mouse up event
            let up_event = CGEvent::new_mouse_event(
                event_source,
                event_type_up,
                screen_pos,
                mem::transmute(cg_button),
            )?;

            // Post the events
            down_event.post(CGEventTapLocation::HID);
            up_event.post(CGEventTapLocation::HID);
        }

        Ok(())
    }
}

impl From<Bounds> for CGRect {
    fn from(value: Bounds) -> Self {
        let screen = MacOSPlatform::get_screen_bounds_for_position(&value.position).unwrap();
        let total_height = screen.size.height as f64;
        CGRect::new(
            CGPoint::new(
                value.position.x as f64,
                total_height - value.position.y as f64 - value.size.height as f64,
            ),
            CGSize::new(value.size.width as f64, value.size.height as f64),
        )
    }
}

impl From<CGRect> for Bounds {
    fn from(value: NSRect) -> Self {
        let screen = MacOSPlatform::get_screen_bounds_for_position(&Position::new(
            value.origin.x as i32,
            value.origin.y as i32,
        ))
        .unwrap();
        let total_height = screen.size.height as i32;
        Bounds::new(
            value.origin.x as i32,
            total_height - value.origin.y as i32 - value.size.height as i32,
            value.size.width as u32,
            value.size.height as u32,
        )
    }
}
