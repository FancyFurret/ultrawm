use crate::platform::PlatformResult;
use std::ffi::c_void;
use windows::core::PCSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

// Manual definitions for blur APIs not available in windows-rs
#[repr(C)]
pub struct ACCENT_POLICY {
    pub accent_state: u32,
    pub accent_flags: u32,
    pub gradient_color: u32,
    pub animation_id: u32,
}

#[repr(C)]
pub struct WINDOWCOMPOSITIONATTRIBDATA {
    pub attribute: u32,
    pub data: *const c_void,
    pub size_of_data: usize,
}

// Accent states for different visual effects
pub const ACCENT_DISABLED: u32 = 0;
pub const ACCENT_ENABLE_GRADIENT: u32 = 1;
pub const ACCENT_ENABLE_TRANSPARENTGRADIENT: u32 = 2;
pub const ACCENT_ENABLE_BLURBEHIND: u32 = 3;
pub const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4; // Fluent Design acrylic effect
pub const ACCENT_INVALID_STATE: u32 = 5;

// Window composition attributes
pub const WCA_ACCENT_POLICY: u32 = 19;
pub const WCA_USEDARKMODECOLORS: u32 = 26;
pub const WCA_BORDER_COLOR: u32 = 34;
pub const WCA_CAPTION_COLOR: u32 = 35;
pub const WCA_TEXT_COLOR: u32 = 36;

type SetWindowCompositionAttributeFn =
    unsafe extern "system" fn(HWND, *const WINDOWCOMPOSITIONATTRIBDATA) -> i32;

/// Enable blur using ACCENT_ENABLE_BLURBEHIND and SetWindowCompositionAttribute
/// This uses the undocumented Windows API for composition effects
pub fn enable_composition_blur(hwnd: HWND) -> PlatformResult<()> {
    unsafe {
        // Load user32.dll and get SetWindowCompositionAttribute function
        let user32 = LoadLibraryA(PCSTR(b"user32.dll\0".as_ptr()))
            .map_err(|e| format!("Failed to load user32.dll: {:?}", e))?;

        let set_window_composition_attribute =
            GetProcAddress(user32, PCSTR(b"SetWindowCompositionAttribute\0".as_ptr()))
                .ok_or("SetWindowCompositionAttribute not found")?;

        let set_window_composition_attribute: SetWindowCompositionAttributeFn =
            std::mem::transmute(set_window_composition_attribute);

        // Set up accent policy for blur behind
        let accent_policy = ACCENT_POLICY {
            accent_state: ACCENT_ENABLE_BLURBEHIND,
            accent_flags: 0,
            gradient_color: 0,
            animation_id: 0,
        };

        let data = WINDOWCOMPOSITIONATTRIBDATA {
            attribute: WCA_ACCENT_POLICY,
            data: &accent_policy as *const _ as *const c_void,
            size_of_data: std::mem::size_of::<ACCENT_POLICY>(),
        };

        let result = set_window_composition_attribute(hwnd, &data);

        if result == 0 {
            return Err("SetWindowCompositionAttribute failed".into());
        }
    }
    Ok(())
}
