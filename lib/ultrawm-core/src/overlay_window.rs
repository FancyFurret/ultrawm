use crate::animation::{ease_in_out_cubic, Animator};
use crate::coalescing_channel::CoalescingAsyncChannel;
use crate::event_loop_main::get_event_loop_blocking;
use crate::platform::{Bounds, PlatformOverlay, PlatformOverlayImpl, PlatformResult, WindowId};
use log::{error, warn};
use skia_safe::{surfaces, Color, Paint, PaintStyle, RRect, Rect, Surface};
use std::thread;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::window::{Window, WindowAttributes, WindowLevel};

#[derive(Debug, Clone)]
pub struct OverlayWindowConfig {
    pub fade_animation_ms: u32,
    pub move_animation_ms: u32,
    pub border_radius: f32,
    pub blur: bool,
    pub background: Option<OverlayWindowBackgroundStyle>,
    pub border: Option<OverlayWindowBorderStyle>,
    pub animation_fps: u32,
}

#[derive(Debug, Clone)]
pub struct OverlayWindowBackgroundStyle {
    pub opacity: f32,
    pub color: Color,
}

#[derive(Debug, Clone)]
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
    command_sender: mpsc::UnboundedSender<OverlayWindowCommand>,
    animator_thread: Option<thread::JoinHandle<()>>,
    shown: bool,
}

pub struct OverlayWindowAnimator {
    window: Window,
    handle: WindowId,
    fade_animator: Animator<f32>,
    native_fade_animation: bool,
    move_animator: Animator<Bounds>,
    native_move_animation: bool,
    surface: Surface,
    last_size: (u32, u32),
    config: OverlayWindowConfig,
    visible: bool,
    command_channel: CoalescingAsyncChannel<OverlayWindowCommand>,
}

impl OverlayWindow {
    pub async fn new(config: OverlayWindowConfig) -> Self {
        let command_channel = CoalescingAsyncChannel::new();
        let command_sender = command_channel.sender();

        let config_clone = config.clone();
        let animator_thread = thread::spawn(move || {
            let surface = match surfaces::raster_n32_premul(skia_safe::ISize::new(100, 100)) {
                Some(s) => s,
                None => {
                    error!("Could not create surface");
                    return;
                }
            };

            let attributes = WindowAttributes::default()
                .with_position(PhysicalPosition::new(0, 0))
                .with_inner_size(LogicalSize::new(0, 0))
                .with_decorations(false)
                .with_transparent(true)
                .with_blur(true)
                .with_visible(true)
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_resizable(false);

            let config_clone_2 = config_clone.clone();
            let window = get_event_loop_blocking(move |event_loop| {
                let window = match event_loop.create_window(attributes) {
                    Ok(w) => w,
                    Err(e) => {
                        error!("Failed to create overlay window: {e}");
                        return None;
                    }
                };

                if let Err(e) = PlatformOverlay::initialize_overlay_window(&window, &config_clone_2)
                {
                    error!("Failed to initialize overlay window: {e}");
                    return None;
                }

                let handle = match PlatformOverlay::get_window_id(&window) {
                    Ok(h) => h,
                    Err(e) => {
                        error!("Failed to get overlay window handle: {e}");
                        return None;
                    }
                };

                Some((window, handle))
            });

            if let Some((window, handle)) = window {
                let mut animator = OverlayWindowAnimator {
                    window,
                    handle,
                    fade_animator: Animator::new(0.0, 0.0, ease_in_out_cubic),
                    native_fade_animation: false,
                    move_animator: Animator::new(
                        Bounds::default(),
                        Bounds::default(),
                        ease_in_out_cubic,
                    ),
                    native_move_animation: false,
                    surface,
                    last_size: (0, 0),
                    config: config_clone,
                    visible: false,
                    command_channel,
                };

                // Create a tokio runtime for the animator thread
                let rt = Runtime::new().unwrap();
                rt.block_on(animator.run_loop());
            }
        });

        Self {
            config,
            shown: false,
            command_sender,
            animator_thread: Some(animator_thread),
        }
    }
    pub fn shown(&self) -> bool {
        self.shown
    }

    pub fn show(&mut self) {
        if self.shown {
            return;
        }

        self.command_sender
            .send(OverlayWindowCommand::Show)
            .unwrap_or_else(|e| error!("Failed to send show command to the overlay window: {e}"));

        self.shown = true;
    }

    pub fn hide(&mut self) {
        if !self.shown {
            return;
        }

        self.command_sender
            .send(OverlayWindowCommand::Hide)
            .unwrap_or_else(|e| warn!("Failed to send hide command: {e}"));

        self.shown = false;
    }

    pub fn move_to(&mut self, bounds: &Bounds) {
        self.command_sender
            .send(OverlayWindowCommand::MoveTo(bounds.clone()))
            .unwrap_or_else(|e| warn!("Failed to send move command: {e}"));
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
    async fn run_loop(&mut self) {
        let mut running = true;
        let frame_duration = Duration::from_secs_f64(1.0 / self.config.animation_fps as f64);
        let mut last_frame_time = Instant::now();

        while running {
            while let Some(cmd) = self
                .command_channel
                .try_coalesce(|cmd| matches!(cmd, OverlayWindowCommand::MoveTo(_)))
            {
                self.handle_command(cmd, &mut running);
            }

            if !running {
                break;
            }

            if self.is_animating() {
                let now = Instant::now();
                let elapsed = now.duration_since(last_frame_time);

                if elapsed >= frame_duration {
                    self.animate_frame();
                    last_frame_time = now;
                } else {
                    tokio::time::sleep(frame_duration - elapsed).await;
                }
            } else {
                if let Some(cmd) = self.command_channel.recv().await {
                    self.handle_command(cmd, &mut running);
                }
            }
        }
    }

    fn handle_command(&mut self, cmd: OverlayWindowCommand, running: &mut bool) {
        match cmd {
            OverlayWindowCommand::Show => {
                self.set_visible(true);
                if self.config.fade_animation_ms == 0 {
                    self.set_opacity(1.0);
                } else {
                    self.start_fade(1.0, self.config.fade_animation_ms);
                }
                let _ = self.render();
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

    fn start_fade(&mut self, opacity: f32, duration_ms: u32) {
        let result = PlatformOverlay::animate_window_opacity(
            self.handle,
            Duration::from_millis(duration_ms as u64),
            opacity,
        );

        if matches!(result, Err(_) | Ok(true)) {
            self.native_fade_animation = true;
        }

        self.fade_animator.start(opacity, duration_ms);
    }

    fn start_move(&mut self, bounds: Bounds, duration_ms: u32) {
        if !self.visible {
            // Set bounds immediately when not visible to avoid showing at old position
            self.set_bounds(bounds.clone());
            self.move_animator.start_from(bounds.clone(), bounds, 0);
            return;
        }

        let result = PlatformOverlay::animate_window_bounds(
            self.handle,
            Duration::from_millis(duration_ms as u64),
            bounds.clone(),
        );

        if matches!(result, Err(_) | Ok(true)) {
            self.native_move_animation = true;
        }

        if *self.fade_animator.current_value() < f32::EPSILON {
            self.move_animator.start_from(bounds.clone(), bounds, 0);
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
                if !self.native_fade_animation {
                    self.set_opacity(new_opacity);
                }

                if !self.fade_animator.is_animating() && new_opacity == 0.0 {
                    self.set_visible(false);
                }
            }
        }

        if self.move_animator.is_animating() {
            if let Some(bounds) = self.move_animator.update() {
                if !self.native_move_animation {
                    self.set_bounds(bounds);
                }
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
