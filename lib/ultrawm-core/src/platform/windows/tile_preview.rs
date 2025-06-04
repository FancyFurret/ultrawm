use crate::platform::{Bounds, PlatformResult, PlatformTilePreviewImpl};
use std::sync::{Arc, Mutex};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmEnableBlurBehindWindow, DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWM_SYSTEMBACKDROP_TYPE,
    DWM_WINDOW_CORNER_PREFERENCE,
};
use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, HRGN};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetWindowLongW, RegisterClassExW, SetLayeredWindowAttributes,
    SetWindowLongW, SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_STYLE, HTTRANSPARENT, LWA_ALPHA,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOW, WM_DESTROY, WM_NCHITTEST, WNDCLASSEXW,
    WS_EX_LAYERED, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    WS_POPUP, WS_VISIBLE,
};

#[derive(PartialEq)]
enum AnimationState {
    None,
    Showing,
    Hiding,
}

pub struct WindowsTilePreview {
    hwnd: HWND,
    state: Arc<Mutex<AnimationState>>,
}

const ANIMATION_DURATION: f64 = 0.15;

impl PlatformTilePreviewImpl for WindowsTilePreview {
    fn new() -> PlatformResult<Self> {
        unsafe {
            let class_name = w!("UltraWMTilePreview");
            let mut wc = WNDCLASSEXW::default();
            wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
            wc.lpfnWndProc = Some(window_proc);
            wc.lpszClassName = class_name;
            RegisterClassExW(&wc);

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TOOLWINDOW
                    | WS_EX_TOPMOST
                    | WS_EX_TRANSPARENT
                    | WS_EX_NOREDIRECTIONBITMAP,
                class_name,
                w!(""),
                WS_POPUP | WS_VISIBLE,
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            );

            // Enable dark mode
            let dark_mode = 1;
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark_mode as *const _ as *const _,
                std::mem::size_of_val(&dark_mode) as u32,
            )
            .ok();

            // Enable blur effect
            let backdrop_type = DWM_SYSTEMBACKDROP_TYPE(2); // DWMSBT_TABBEDWINDOW
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &backdrop_type as *const _ as *const _,
                std::mem::size_of_val(&backdrop_type) as u32,
            )
            .ok();

            // Set rounded corners
            let corner_preference = DWM_WINDOW_CORNER_PREFERENCE(2); // DWMWCP_ROUND
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner_preference as *const _ as *const _,
                std::mem::size_of_val(&corner_preference) as u32,
            )
            .ok();

            // Set window opacity
            let opacity = 0.95; // Increased opacity to make blur more visible
            SetLayeredWindowAttributes(hwnd, COLORREF(0), (opacity * 255.0) as u8, LWA_ALPHA).ok();

            // Make window transparent to mouse events
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            SetWindowLongW(
                hwnd,
                GWL_EXSTYLE,
                (ex_style | WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0) as i32,
            );
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            SetWindowLongW(hwnd, GWL_STYLE, (style | WS_POPUP.0) as i32);

            Ok(Self {
                hwnd,
                state: Arc::new(Mutex::new(AnimationState::None)),
            })
        }
    }

    fn show(&mut self) -> PlatformResult<()> {
        let state = self.state.clone();
        *state.lock().unwrap() = AnimationState::Showing;

        unsafe {
            ShowWindow(self.hwnd, SW_SHOW);
            // TODO: Implement fade-in animation using SetLayeredWindowAttributes
        }

        Ok(())
    }

    fn hide(&mut self) -> PlatformResult<()> {
        let state = self.state.clone();
        *state.lock().unwrap() = AnimationState::Hiding;

        unsafe {
            // TODO: Implement fade-out animation using SetLayeredWindowAttributes
            ShowWindow(self.hwnd, SW_HIDE);
            *state.lock().unwrap() = AnimationState::None;
        }

        Ok(())
    }

    fn move_to(&mut self, bounds: &Bounds) -> PlatformResult<()> {
        unsafe {
            // TODO: Implement smooth movement animation
            SetWindowPos(
                self.hwnd,
                None,
                bounds.position.x,
                bounds.position.y,
                bounds.size.width as i32,
                bounds.size.height as i32,
                SWP_NOZORDER,
            )
            .ok();
        }

        Ok(())
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            // Clean up
            LRESULT(0)
        }
        WM_NCHITTEST => {
            // Make window transparent to mouse events
            LRESULT(HTTRANSPARENT as isize)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
