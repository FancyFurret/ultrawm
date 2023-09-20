#![allow(dead_code)]

use application_services::accessibility_ui::AXUIElement;
use application_services::{kAXErrorSuccess, AXError, AXUIElementRef};
use core_foundation::base::TCFType;

#[link(name = "AppKit", kind = "framework")]
extern "C" {
    pub fn _AXUIElementGetWindow(element: AXUIElementRef, window: *mut u32) -> AXError;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn _CGSDefaultConnection() -> u32;
    pub fn CGSSetWindowAlpha(connection: u32, window_number: u32, alpha: f64);
    pub fn CGSMoveWorkspaceWindowList(
        connection: u32,
        window_numbers: *const u32,
        count: u32,
        workspace: u32,
    );
}

pub fn get_window_id(element: &AXUIElement) -> Option<u32> {
    unsafe {
        let mut window_id: u32 = 0;
        let result = _AXUIElementGetWindow(element.as_concrete_TypeRef(), &mut window_id);
        if result == kAXErrorSuccess {
            Some(window_id)
        } else {
            None
        }
    }
}
