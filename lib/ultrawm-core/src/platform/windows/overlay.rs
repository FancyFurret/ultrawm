use crate::overlay_window::OverlayWindowConfig;
use crate::platform::windows::ffi::enable_composition_blur;
use crate::platform::{Bounds, PlatformOverlayImpl, PlatformResult, WindowId};
use skia_safe::{op, Image};
use windows::Win32::Foundation::{COLORREF, HWND};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED, DWMWA_USE_IMMERSIVE_DARK_MODE,
};
use windows::Win32::Graphics::Gdi::{
    GetDC, ReleaseDC, SetDIBitsToDevice, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW, SetWindowPos, ShowWindow,
    GWL_EXSTYLE, GWL_STYLE, HWND_TOPMOST, LWA_ALPHA, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SW_SHOW,
    WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};
use winit::platform::windows::{CornerPreference, WindowExtWindows};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

pub struct WindowsPlatformOverlay;

fn get_hwnd(window: &Window) -> PlatformResult<HWND> {
    // Only support Win32 window handles
    let window_handle = window
        .window_handle()
        .map_err(|e| format!("Failed to get window handle: {}", e))?;

    if let RawWindowHandle::Win32(handle) = window_handle.as_raw() {
        Ok(HWND(handle.hwnd.get() as _))
    } else {
        Err("Expected Win32 window handle".into())
    }
}

impl PlatformOverlayImpl for WindowsPlatformOverlay {
    fn get_window_id(window: &Window) -> PlatformResult<WindowId> {
        let window_handle = window
            .window_handle()
            .map_err(|e| format!("Failed to get window handle: {}", e))?;

        if let RawWindowHandle::Win32(handle) = window_handle.as_raw() {
            Ok(handle.hwnd.get() as _)
        } else {
            Err("Expected Win32 window handle".into())
        }
    }

    fn set_window_bounds(window_id: WindowId, bounds: Bounds) -> PlatformResult<()> {
        unsafe {
            SetWindowPos(
                HWND(window_id as isize),
                HWND_TOPMOST,
                bounds.position.x,
                bounds.position.y,
                bounds.size.width as i32,
                bounds.size.height as i32,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )
            .map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    fn set_window_opacity(window_id: WindowId, opacity: f32) -> PlatformResult<()> {
        unsafe {
            SetLayeredWindowAttributes(
                HWND(window_id as isize),
                COLORREF(0),
                (opacity * 255.0) as u8,
                LWA_ALPHA,
            )
            .map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    fn render_to_window(image: &Image, window_id: WindowId) -> PlatformResult<()> {
        let hwnd = HWND(window_id as _);

        // Get the image pixels
        let Some(pixmap) = image.peek_pixels() else {
            return Err("Failed to get Skia image pixels".into());
        };
        let width = pixmap.width();
        let height = pixmap.height();
        let skia_pixels = pixmap.addr() as *const u8;

        unsafe {
            let hdc = GetDC(hwnd);
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -(height), // Negative for top-down DIB
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [Default::default(); 1],
            };
            let res = SetDIBitsToDevice(
                hdc,
                0,
                0,
                width as u32,
                height as u32,
                0,
                0,
                0,
                height as u32,
                skia_pixels as *const _,
                &bmi,
                DIB_RGB_COLORS,
            );
            if res == 0 {
                println!("Failed to render image to window");
                return Err("Failed to render image to window".into());
            }
            ReleaseDC(hwnd, hdc);
        }

        Ok(())
    }
    fn initialize_overlay_window(
        window: &Window,
        config: &OverlayWindowConfig,
    ) -> PlatformResult<()> {
        unsafe {
            let hwnd = get_hwnd(window)?;

            // Get current styles
            let current_style = GetWindowLongW(hwnd, GWL_STYLE);
            let current_ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);

            let new_style = current_style | WS_POPUP.0 as i32;
            let new_ex_style = (current_ex_style & !(WS_EX_APPWINDOW.0 as i32))
                | WS_EX_TRANSPARENT.0 as i32
                | WS_EX_TOOLWINDOW.0 as i32
                | WS_EX_LAYERED.0 as i32
                | WS_EX_NOACTIVATE.0 as i32
                | WS_EX_TOPMOST.0 as i32;

            ShowWindow(hwnd, SW_SHOW);
            SetWindowLongW(hwnd, GWL_STYLE, new_style);
            SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex_style);
            ShowWindow(hwnd, SW_SHOW);

            let dark_mode = 1;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark_mode as *const _ as *const _,
                size_of_val(&dark_mode) as u32,
            );

            let disable_transitions = 1u32;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_TRANSITIONS_FORCEDISABLED,
                &disable_transitions as *const _ as *const _,
                size_of::<u32>() as u32,
            );

            // Use Windows Composition API to create a backdrop-blurred visual
            if config.blur {
                enable_composition_blur(hwnd)?;
            }
        }

        // Note: Removed window.set_enable(false) as it can interfere with click-through behavior
        window.set_skip_taskbar(true);
        window.set_undecorated_shadow(false);
        window.set_corner_preference(CornerPreference::Round);

        Ok(())
    }
}
