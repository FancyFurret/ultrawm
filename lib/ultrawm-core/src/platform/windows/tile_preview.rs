use crate::config::ConfigRef;
use crate::platform::animation::{ease_in_out_cubic, Animator};
use crate::platform::{Bounds, PlatformResult, PlatformTilePreviewImpl, Position, Size};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
    DWMWA_WINDOW_CORNER_PREFERENCE, DWM_SYSTEMBACKDROP_TYPE, DWM_WINDOW_CORNER_PREFERENCE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongW, RegisterClassExW,
    SetLayeredWindowAttributes, SetWindowLongW, SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_STYLE,
    HTTRANSPARENT, LWA_ALPHA, SWP_NOZORDER, SW_HIDE, SW_SHOW, WM_DESTROY, WM_NCHITTEST,
    WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
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
    command_tx: Sender<TilePreviewCommand>,
}

struct TilePreviewConfig {
    fade_duration: f64,
    move_duration: f64,
    frame_time: Duration,
}

impl PlatformTilePreviewImpl for WindowsTilePreview {
    fn new(config: ConfigRef) -> PlatformResult<Self> {
        let (command_tx, command_rx) = mpsc::channel();
        let tile_config = TilePreviewConfig {
            fade_duration: if config.tile_preview_fade_animate {
                config.tile_preview_animation_duration as f64 / 1000.0
            } else {
                0.0
            },
            move_duration: if config.tile_preview_move_animate {
                config.tile_preview_animation_duration as f64 / 1000.0
            } else {
                0.0
            },
            frame_time: Duration::from_secs_f64(1.0 / config.tile_preview_fps as f64),
        };
        thread::spawn(move || {
            if let Ok(mut controller) = TilePreviewController::new(command_rx, tile_config) {
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
    command_rx: Receiver<TilePreviewCommand>,
    bounds_animator: Animator<Bounds, fn(f64) -> f64>,
    fade_animator: Animator<u8, fn(f64) -> f64>,
    is_visible: bool,
    config: TilePreviewConfig,
}

impl TilePreviewController {
    fn new(
        command_rx: Receiver<TilePreviewCommand>,
        config: TilePreviewConfig,
    ) -> PlatformResult<Self> {
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
            bounds_animator: Animator::new(
                initial_bounds.clone(),
                initial_bounds.clone(),
                ease_in_out_cubic,
            ),
            fade_animator: Animator::new(0, 0, ease_in_out_cubic),
            is_visible: false,
            config,
        })
    }

    fn run_loop(&mut self) {
        loop {
            let mut did_command = false;
            let animating =
                self.bounds_animator.is_animating() || self.fade_animator.is_animating();
            if animating {
                match self.command_rx.try_recv() {
                    Ok(command) => {
                        self.handle_command(command);
                        did_command = true;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        unsafe {
                            let _ = DestroyWindow(self.hwnd);
                        }
                        break;
                    }
                }
                if animating || did_command {
                    self.update_animation();
                }
                thread::sleep(self.config.frame_time);
            } else {
                match self.command_rx.recv() {
                    Ok(command) => {
                        self.handle_command(command);
                        did_command = true;
                    }
                    Err(_) => {
                        unsafe {
                            let _ = DestroyWindow(self.hwnd);
                        }
                        break;
                    }
                }
                if did_command {
                    self.update_animation();
                }
            }
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
                self.fade_animator.start(
                    *self.fade_animator.current_value(),
                    255,
                    self.config.fade_duration,
                );
            }
            TilePreviewCommand::Hide => {
                self.fade_animator.start(
                    *self.fade_animator.current_value(),
                    0,
                    self.config.fade_duration,
                );
            }
            TilePreviewCommand::MoveTo(bounds) => {
                let from = if *self.fade_animator.current_value() == 0 {
                    bounds.clone()
                } else {
                    self.bounds_animator.current_value().clone()
                };

                self.bounds_animator
                    .start(from, bounds, self.config.move_duration);
            }
        }
    }

    fn update_animation(&mut self) {
        // Animate movement
        if self.bounds_animator.is_animating() {
            if let Some(new_bounds) = self.bounds_animator.update() {
                unsafe {
                    SetWindowPos(
                        self.hwnd,
                        None,
                        new_bounds.position.x,
                        new_bounds.position.y,
                        new_bounds.size.width as i32,
                        new_bounds.size.height as i32,
                        SWP_NOZORDER,
                    )
                    .ok();
                }
            }
        }
        // Animate opacity
        if self.fade_animator.is_animating() {
            if let Some(new_alpha) = self.fade_animator.update() {
                unsafe {
                    let _ =
                        SetLayeredWindowAttributes(self.hwnd, COLORREF(0), new_alpha, LWA_ALPHA);
                }
            }
        }
        if *self.fade_animator.current_value() == 0
            && !self.fade_animator.is_animating()
            && self.is_visible
        {
            unsafe {
                ShowWindow(self.hwnd, SW_HIDE);
            }
            self.is_visible = false;
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
