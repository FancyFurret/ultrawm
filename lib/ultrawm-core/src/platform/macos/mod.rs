pub use manageable::*;
pub use tile_preview::*;
pub use window::*;

use crate::platform::macos::event_listener_ax::EventListenerAX;
use crate::platform::macos::event_listener_cg::EventListenerCG;
use crate::platform::macos::event_listener_ns::EventListenerNS;
use crate::platform::macos::ffi::{window_info, AXUIElementExt, CFArrayExt, CFDictionaryExt};
use crate::platform::macos::ObserveError::NotManageable;
use crate::platform::traits::{PlatformImpl, PlatformInitImpl};
use crate::platform::{Bounds, Display, EventDispatcher, PlatformResult, Position, ProcessId};
use application_services::accessibility_ui::AXUIElement;
use application_services::pid_t;
use core_graphics::window::{copy_window_info, kCGNullWindowID, kCGWindowListOptionAll};
use icrate::block2::{ConcreteBlock, RcBlock};
use icrate::objc2::rc::autoreleasepool;
use icrate::AppKit::{NSApplication, NSApplicationLoad, NSDeviceDescriptionKey, NSEvent, NSScreen};
use icrate::Foundation::{
    is_main_thread, CGPoint, CGRect, CGSize, NSBlockOperation, NSNumber, NSOperationQueue, NSRect,
    NSThread,
};
use objc2::rc::Id;
use std::collections::HashSet;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};

mod event_listener_ax;
mod event_listener_cg;
mod event_listener_ns;
mod ffi;
mod manageable;
mod tile_preview;
mod window;

pub struct MacOSPlatformInit;

unsafe impl PlatformInitImpl for MacOSPlatformInit {
    unsafe fn initialize() -> PlatformResult<()> {
        NSApplicationLoad();
        Ok(())
    }

    unsafe fn run_event_loop(dispatcher: EventDispatcher) -> PlatformResult<()> {
        autoreleasepool(|_| -> PlatformResult<()> {
            unsafe {
                NSApplicationLoad();

                let listener_ax = EventListenerAX::run(dispatcher.clone())?;
                let _listener_ns = EventListenerNS::run(listener_ax.clone())?;
                let _listener_cg = EventListenerCG::run(dispatcher.clone())?;

                NSApplication::sharedApplication().run();
            }

            Ok(())
        })
    }
}

pub struct MacOSPlatform;

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
}

impl PlatformImpl for MacOSPlatform {
    fn is_main_thread() -> bool {
        NSThread::currentThread().isMainThread()
    }

    fn run_on_main_thread<F, R>(f: F) -> PlatformResult<R>
    where
        F: FnOnce() -> R + Send,
        R: Send + 'static,
    {
        if is_main_thread() {
            return Ok(f());
        }

        let func = Arc::new(Mutex::new(Some(f)));
        let result = Arc::new(Mutex::new(None));

        let block = {
            let result = result.clone();
            ConcreteBlock::new(move || {
                if let Some(func) = func.lock().unwrap().take() {
                    result.lock().unwrap().replace(Some(func()));
                }
            })
        };

        // This is how block.copy() works and produces an RcBlock
        // The issue is that block.copy() requires the block to be static, but
        // our block is not. We can safely create an RcBlock from this block
        // because we are waiting for the operation to finish before leaving this function.
        let mut ptr = ManuallyDrop::new(block);
        let ptr: *mut c_void = &mut *ptr as *mut _ as *mut c_void;
        let block: RcBlock<(), ()> = unsafe { RcBlock::copy(ptr.cast()) };

        unsafe {
            let op = NSBlockOperation::blockOperationWithBlock(&block);
            NSOperationQueue::mainQueue().addOperation(&op);
            op.waitUntilFinished();
        }

        let result = result.lock().unwrap().take().unwrap().unwrap();
        Ok(result)
    }

    fn list_all_windows() -> PlatformResult<Vec<MacOSPlatformWindow>> {
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
        unsafe {
            let mut result = Vec::new();
            let displays = NSScreen::screens();

            for screen in displays {
                let desc = screen.deviceDescription();
                let key = NSDeviceDescriptionKey::from_str("NSScreenNumber");
                let obj = desc.objectForKey(&key).ok_or("Could not get screen id")?;
                let number = Id::cast::<NSNumber>(obj);

                result.push(Display {
                    id: number.unsignedIntegerValue() as u32,
                    name: screen.localizedName().to_string(),
                    bounds: screen.frame().into(),
                    work_area: screen.visibleFrame().into(),
                });
            }

            Ok(result)
        }
    }

    fn get_mouse_position() -> PlatformResult<Position> {
        // TODO: Slow?
        unsafe {
            let pos = NSEvent::mouseLocation();
            let position = Position::new(pos.x as i32, pos.y as i32);
            let screen = get_screen_for_position(&position).unwrap();
            let total_height = screen.frame().size.height as f64;
            Ok(Position::new(
                position.x,
                total_height as i32 - position.y - 1,
            ))
        }
    }
}

fn get_screen_for_position(position: &Position) -> Option<Id<NSScreen>> {
    unsafe {
        let screens = NSScreen::screens();
        for screen in screens {
            let frame = screen.frame();
            if position.x >= frame.origin.x as i32
                && position.x < frame.origin.x as i32 + frame.size.width as i32
                && position.y >= frame.origin.y as i32
                && position.y < frame.origin.y as i32 + frame.size.height as i32
            {
                return Some(screen);
            }
        }

        None
    }
}

impl From<Bounds> for CGRect {
    fn from(value: Bounds) -> Self {
        unsafe {
            let screen = get_screen_for_position(&value.position).unwrap();
            let total_height = screen.frame().size.height as f64;
            CGRect::new(
                CGPoint::new(
                    value.position.x as f64,
                    total_height - value.position.y as f64 - value.size.height as f64,
                ),
                CGSize::new(value.size.width as f64, value.size.height as f64),
            )
        }
    }
}

impl From<CGRect> for Bounds {
    fn from(value: NSRect) -> Self {
        unsafe {
            let screen = get_screen_for_position(&Position::new(
                value.origin.x as i32,
                value.origin.y as i32,
            ))
            .unwrap();
            let total_height = screen.frame().size.height as i32;
            Bounds::new(
                value.origin.x as i32,
                total_height - value.origin.y as i32 - value.size.height as i32,
                value.size.width as u32,
                value.size.height as u32,
            )
        }
    }
}
