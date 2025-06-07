use crate::platform::{Bounds, PlatformResult, PlatformTilePreviewImpl, Position, Size};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmEnableBlurBehindWindow, DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWM_SYSTEMBACKDROP_TYPE,
    DWM_WINDOW_CORNER_PREFERENCE,
};
use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, HRGN};
use windows::Win32::UI::WindowsAndMessaging::{
    AnimateWindow, CreateWindowExW, DefWindowProcW, GetWindowLongW, GetWindowRect,
    RegisterClassExW, SetLayeredWindowAttributes, SetWindowLongW, SetWindowPos, ShowWindow,
    AW_ACTIVATE, AW_BLEND, AW_HIDE, GWL_EXSTYLE, GWL_STYLE, HTTRANSPARENT, LWA_ALPHA,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOW, WM_DESTROY,
    WM_NCHITTEST, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
};

#[derive(PartialEq)]
enum AnimationState {
    None,
    Showing,
    Hiding,
}

enum TilePreviewCommand {
    Show,
    Hide,
    MoveTo(Bounds),
}

pub struct WindowsTilePreview {
    command_tx: UnboundedSender<TilePreviewCommand>,
}

const ANIMATION_DURATION: f64 = 0.15;

fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - 4.0 * (1.0 - t) * (1.0 - t) * (1.0 - t)
    }
}

impl PlatformTilePreviewImpl for WindowsTilePreview {
    fn new() -> PlatformResult<Self> {
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        thread::spawn(move || {
            if let Ok(mut controller) = TilePreviewController::new(command_rx) {
                controller.run_loop();
            }
        });

        Ok(Self { command_tx })
    }

    fn show(&mut self) -> PlatformResult<()> {
        self.command_tx.send(TilePreviewCommand::Show).ok();
        Ok(())
    }

    fn hide(&mut self) -> PlatformResult<()> {
        self.command_tx.send(TilePreviewCommand::Hide).ok();
        Ok(())
    }

    fn move_to(&mut self, bounds: &Bounds) -> PlatformResult<()> {
        self.command_tx
            .send(TilePreviewCommand::MoveTo(bounds.clone()))
            .ok();
        Ok(())
    }
}

struct TilePreviewController {
    hwnd: HWND,
    command_rx: UnboundedReceiver<TilePreviewCommand>,
    current_bounds: Bounds,
    target_bounds: Bounds,
    current_alpha: u8,
    target_alpha: u8,
    animation_start: Option<Instant>,
    animating_opacity: bool,
    animating_move: bool,
    is_visible: bool,
}

impl TilePreviewController {
    fn new(command_rx: UnboundedReceiver<TilePreviewCommand>) -> PlatformResult<Self> {
        let hwnd = unsafe {
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
            let backdrop_type = DWM_SYSTEMBACKDROP_TYPE(3);
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

            // Make window transparent to mouse events
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            SetWindowLongW(
                hwnd,
                GWL_EXSTYLE,
                (ex_style | WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0) as i32,
            );
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            SetWindowLongW(hwnd, GWL_STYLE, (style | WS_POPUP.0) as i32);

            hwnd
        };

        let initial_bounds = Bounds {
            position: Position { x: 0, y: 0 },
            size: Size {
                width: 0,
                height: 0,
            },
        };

        Ok(Self {
            hwnd,
            command_rx,
            current_bounds: initial_bounds.clone(),
            target_bounds: initial_bounds,
            current_alpha: 0,
            target_alpha: 0,
            animation_start: None,
            animating_opacity: false,
            animating_move: false,
            is_visible: false,
        })
    }

    fn run_loop(&mut self) {
        loop {
            while let Ok(command) = self.command_rx.try_recv() {
                self.handle_command(command);
            }

            self.update_animation();

            if self.command_rx.is_closed() {
                unsafe {
                    windows::Win32::UI::WindowsAndMessaging::DestroyWindow(self.hwnd);
                }
                break;
            }

            thread::sleep(Duration::from_millis(4)); // ~60fps
        }
    }

    fn handle_command(&mut self, command: TilePreviewCommand) {
        match command {
            TilePreviewCommand::Show => {
                if !self.is_visible {
                    self.is_visible = true;
                    unsafe {
                        ShowWindow(self.hwnd, SW_SHOW);
                    }
                }
                self.target_alpha = 255;
                self.animating_opacity = true;
                self.animation_start = Some(Instant::now());
                self.current_bounds = Bounds {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 0,
                        height: 0,
                    },
                };
            }
            TilePreviewCommand::Hide => {
                self.target_alpha = 0;
                self.animating_opacity = true;
                self.animation_start = Some(Instant::now());
            }
            TilePreviewCommand::MoveTo(bounds) => {
                if self.current_bounds.position.x == 0
                    && self.current_bounds.position.y == 0
                    && self.current_bounds.size.width == 0
                    && self.current_bounds.size.height == 0
                {
                    self.current_bounds = bounds.clone();
                }

                if self.target_bounds != bounds {
                    self.target_bounds = bounds;
                    self.animating_move = true;
                    self.animation_start = Some(Instant::now());
                }
            }
        }
    }

    fn update_current_bounds_from_window(&mut self) {
        unsafe {
            let mut rect = RECT::default();
            if GetWindowRect(self.hwnd, &mut rect).is_ok() {
                self.current_bounds = rect.into();
            }
        }
    }

    fn update_animation(&mut self) {
        let mut any_animating = false;
        // Animate movement
        if self.animating_move {
            if let Some(start_time) = self.animation_start {
                let elapsed = start_time.elapsed().as_secs_f64();
                let mut t = elapsed / ANIMATION_DURATION;
                if t >= 1.0 {
                    t = 1.0;
                    self.animating_move = false;
                } else {
                    any_animating = true;
                }
                let eased_t = ease_in_out_cubic(t);
                self.current_bounds = self
                    .current_bounds
                    .interpolate(&self.target_bounds, eased_t);
                if t >= 1.0 {
                    self.current_bounds = self.target_bounds.clone();
                }
                unsafe {
                    SetWindowPos(
                        self.hwnd,
                        None,
                        self.current_bounds.position.x,
                        self.current_bounds.position.y,
                        self.current_bounds.size.width as i32,
                        self.current_bounds.size.height as i32,
                        SWP_NOZORDER,
                    )
                    .ok();
                }
            }
        }
        // Animate opacity
        if self.animating_opacity {
            if let Some(start_time) = self.animation_start {
                let elapsed = start_time.elapsed().as_secs_f64();
                let mut t = elapsed / ANIMATION_DURATION;
                if t >= 1.0 {
                    t = 1.0;
                    self.animating_opacity = false;
                } else {
                    any_animating = true;
                }
                let eased_t = ease_in_out_cubic(t);
                let delta = self.target_alpha as f64 - self.current_alpha as f64;
                let new_alpha = (self.current_alpha as f64 + delta * eased_t)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                unsafe {
                    SetLayeredWindowAttributes(self.hwnd, COLORREF(0), new_alpha, LWA_ALPHA);
                }
                if t >= 1.0 {
                    self.current_alpha = self.target_alpha;
                }
            }
        }
        // If fade-out finished, hide window
        if self.current_alpha == 0 && !self.animating_opacity && self.is_visible {
            unsafe {
                ShowWindow(self.hwnd, SW_HIDE);
            }
            self.is_visible = false;
        }
        // If any animation is running, keep the animation_start as is, else clear it
        if !any_animating {
            self.animation_start = None;
        }
    }
}

impl Bounds {
    fn interpolate(&self, target: &Bounds, t: f64) -> Bounds {
        let x = self.position.x + ((target.position.x - self.position.x) as f64 * t).round() as i32;
        let y = self.position.y + ((target.position.y - self.position.y) as f64 * t).round() as i32;
        let w = self.size.width as f64
            + ((target.size.width as i32 - self.size.width as i32) as f64 * t).round() as f64;
        let h = self.size.height as f64
            + ((target.size.height as i32 - self.size.height as i32) as f64 * t).round() as f64;

        Bounds {
            position: Position { x, y },
            size: Size {
                width: w as u32,
                height: h as u32,
            },
        }
    }
}

impl From<RECT> for Bounds {
    fn from(rect: RECT) -> Self {
        Bounds {
            position: Position {
                x: rect.left,
                y: rect.top,
            },
            size: Size {
                width: (rect.right - rect.left).abs() as u32,
                height: (rect.bottom - rect.top).abs() as u32,
            },
        }
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
