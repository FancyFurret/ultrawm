use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::animation::{ease_in_out_cubic, Animator};
use crate::event_loop_main::run_on_main_thread_blocking;
use crate::platform::{Bounds, PlatformOverlay, PlatformOverlayImpl, PlatformResult, WindowId};
use crate::{UltraWMFatalError, UltraWMResult};
use skia_safe::{surfaces, Color, Paint, PaintStyle, RRect, Rect, Surface};
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::window::{Window, WindowAttributes, WindowLevel};

#[derive(Clone)]
pub struct OverlayWindowConfig {
    pub fade_animation_ms: u32,
    pub move_animation_ms: u32,
    pub border_radius: f32,
    pub blur: bool,
    pub background: Option<OverlayWindowBackgroundStyle>,
    pub border: Option<OverlayWindowBorderStyle>,
    pub animation_fps: u32,
}

#[derive(Clone)]
pub struct OverlayWindowBackgroundStyle {
    pub opacity: f32,
    pub color: Color,
}

#[derive(Clone)]
pub struct OverlayWindowBorderStyle {
    pub(crate) color: Color,
    pub(crate) width: u32,
}

#[derive(Debug)]
pub enum OverlayWindowCommand {
    Show,
    Hide,
    MoveTo(Bounds),
    Exit,
}

pub struct OverlayWindow {
    config: OverlayWindowConfig,
    command_sender: Sender<OverlayWindowCommand>,
    animator_thread: Option<thread::JoinHandle<()>>,
    shown: bool,
}

pub struct OverlayWindowAnimator {
    window: Window,
    handle: WindowId,
    fade_animator: Animator<f32>,
    move_animator: Animator<Bounds>,
    surface: Surface,
    last_size: (u32, u32),
    config: OverlayWindowConfig,
    visible: bool,
    command_receiver: Receiver<OverlayWindowCommand>,
}

impl OverlayWindow {
    pub async fn new(config: OverlayWindowConfig) -> UltraWMResult<Self> {
        let (tx, rx) = channel();

        let config_clone = config.clone();
        let animator_thread = thread::spawn(move || {
            let surface = surfaces::raster_n32_premul(skia_safe::ISize::new(100, 100)).unwrap();
            let attributes = WindowAttributes::default()
                .with_position(PhysicalPosition::new(0, 0))
                .with_inner_size(LogicalSize::new(0, 0))
                .with_decorations(false)
                .with_transparent(true)
                .with_blur(true)
                .with_active(false)
                .with_visible(true)
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_resizable(false);

            let config_clone_2 = config_clone.clone();
            let (window, handle) = run_on_main_thread_blocking(move |event_loop| {
                let window = event_loop.create_window(attributes).unwrap();
                PlatformOverlay::initialize_overlay_window(&window, &config_clone_2).unwrap();
                let handle = PlatformOverlay::get_window_id(&window).unwrap();
                (window, handle)
            });

            let mut animator = OverlayWindowAnimator {
                window,
                handle,
                fade_animator: Animator::new(0.0, 0.0, ease_in_out_cubic),
                move_animator: Animator::new(
                    Bounds::default(),
                    Bounds::default(),
                    ease_in_out_cubic,
                ),
                surface,
                last_size: (0, 0),
                config: config_clone,
                visible: false,
                command_receiver: rx,
            };

            animator.run_loop();
        });

        Ok(Self {
            config,
            shown: false,
            command_sender: tx,
            animator_thread: Some(animator_thread),
        })
    }
    pub fn shown(&self) -> bool {
        self.shown
    }

    pub fn show(&mut self) -> UltraWMResult<()> {
        if self.shown {
            return Ok(());
        }

        self.command_sender
            .send(OverlayWindowCommand::Show)
            .map_err(|e| -> UltraWMFatalError {
                format!("Failed to send show command: {}", e).into()
            })?;

        self.shown = true;
        Ok(())
    }

    pub fn hide(&mut self) -> UltraWMResult<()> {
        if !self.shown {
            return Ok(());
        }

        self.command_sender
            .send(OverlayWindowCommand::Hide)
            .map_err(|e| -> UltraWMFatalError {
                format!("Failed to send hide command: {}", e).into()
            })?;

        self.shown = false;
        Ok(())
    }

    pub fn move_to(&mut self, bounds: &Bounds) -> UltraWMResult<()> {
        self.command_sender
            .send(OverlayWindowCommand::MoveTo(bounds.clone()))
            .map_err(|e| -> UltraWMFatalError {
                format!("Failed to send move command: {}", e).into()
            })
    }
}

impl Drop for OverlayWindow {
    fn drop(&mut self) {
        let _ = self.command_sender.send(OverlayWindowCommand::Exit);
        if let Some(thread) = self.animator_thread.take() {
            let _ = thread.join();
        }
    }
}

impl OverlayWindowAnimator {
    fn run_loop(&mut self) {
        let mut running = true;
        let frame_duration = Duration::from_secs_f64(1.0 / self.config.animation_fps as f64);
        let mut last_frame_time = Instant::now();

        while running {
            while let Ok(cmd) = self.command_receiver.try_recv() {
                self.handle_command(cmd, &mut running);
            }

            if self.is_animating() {
                let now = Instant::now();
                let elapsed = now.duration_since(last_frame_time);

                if elapsed >= frame_duration {
                    self.animate_frame();
                    last_frame_time = now;
                } else {
                    thread::sleep(frame_duration - elapsed);
                }
            } else {
                match self.command_receiver.recv() {
                    Ok(cmd) => self.handle_command(cmd, &mut running),
                    Err(_) => break,
                }
            }

            if !running {
                break;
            }
        }
    }

    fn handle_command(&mut self, cmd: OverlayWindowCommand, running: &mut bool) {
        match cmd {
            OverlayWindowCommand::Show => {
                if self.config.fade_animation_ms == 0 {
                    self.set_visible(true);
                    self.set_opacity(1.0);
                    let _ = self.render();
                } else {
                    self.set_visible(true);
                    self.start_fade(1.0, self.config.fade_animation_ms);
                    let _ = self.render();
                }
            }
            OverlayWindowCommand::Hide => {
                if self.config.fade_animation_ms == 0 {
                    self.set_visible(false);
                } else {
                    self.start_fade(0.0, self.config.fade_animation_ms);
                }
            }
            OverlayWindowCommand::MoveTo(bounds) => {
                if self.config.move_animation_ms == 0 {
                    self.set_bounds(bounds);
                    let _ = self.render();
                } else {
                    self.start_move(bounds, self.config.move_animation_ms);
                }
            }
            OverlayWindowCommand::Exit => {
                *running = false;
            }
        }
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn set_opacity(&mut self, opacity: f32) {
        let _ = PlatformOverlay::set_window_opacity(self.handle, opacity);
    }

    fn set_bounds(&mut self, bounds: Bounds) {
        let _ = PlatformOverlay::set_window_bounds(self.handle, bounds);
    }

    fn start_fade(&mut self, target_opacity: f32, duration_ms: u32) {
        self.fade_animator.start(target_opacity, duration_ms);
    }

    fn start_move(&mut self, bounds: Bounds, duration_ms: u32) {
        if *self.fade_animator.current_value() < f32::EPSILON {
            self.move_animator
                .start_from(bounds.clone(), bounds, duration_ms);
        } else {
            self.move_animator.start(bounds, duration_ms);
        }
    }

    fn is_animating(&self) -> bool {
        self.fade_animator.is_animating() || self.move_animator.is_animating()
    }

    fn animate_frame(&mut self) -> bool {
        if !self.is_animating() {
            return false;
        }

        if self.fade_animator.is_animating() {
            if let Some(new_opacity) = self.fade_animator.update() {
                self.set_opacity(new_opacity);

                if !self.fade_animator.is_animating() && new_opacity == 0.0 {
                    self.set_visible(false);
                }
            }
        }

        if self.move_animator.is_animating() {
            if let Some(bounds) = self.move_animator.update() {
                self.set_bounds(bounds);
            }
        }

        let _ = self.render();
        true
    }
    fn check_resize(&mut self) {
        let size = self.window.inner_size();
        let (w, h) = (size.width, size.height);
        if (w, h) != self.last_size && w > 0 && h > 0 {
            self.surface =
                surfaces::raster_n32_premul(skia_safe::ISize::new(w as i32, h as i32)).unwrap();
            self.last_size = (w, h);
        }
    }

    fn draw(&mut self) -> PlatformResult<()> {
        self.check_resize();

        let canvas = self.surface.canvas();
        canvas.clear(Color::from_argb(0, 0, 0, 0));
        if !self.visible {
            return Ok(());
        }

        let size = self.window.inner_size();
        let width = size.width as f32;
        let height = size.height as f32;
        let rect = Rect::from_xywh(0.0, 0.0, width, height);
        let rounded_rect =
            RRect::new_rect_xy(rect, self.config.border_radius, self.config.border_radius);

        if let Some(background) = &self.config.background {
            let mut paint = Paint::default();
            paint.set_color(background.color.with_a((background.opacity * 255.0) as u8));
            paint.set_style(PaintStyle::Fill);
            canvas.draw_rrect(&rounded_rect, &paint);
        }
        if let Some(border) = &self.config.border {
            if border.width > 0 {
                let mut border_paint = Paint::default();
                border_paint.set_color(border.color);
                border_paint.set_style(PaintStyle::Stroke);
                border_paint.set_stroke_width(border.width as f32);
                canvas.draw_rrect(&rounded_rect, &border_paint);
            }
        }
        Ok(())
    }

    fn render(&mut self) -> PlatformResult<()> {
        self.draw()?;
        PlatformOverlay::render_to_window(&self.surface.image_snapshot(), self.handle)?;
        Ok(())
    }
}
