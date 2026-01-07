use crate::animation::{ease_in_out_cubic, Animator};
use crate::event_loop_main::get_event_loop_blocking;
use crate::overlay::content::OverlayContent;
use crate::overlay::handle::Overlay;
use crate::overlay::OverlayId;
use crate::overlay::{OverlayWindowCommand, OverlayWindowConfig};
use crate::platform::{Bounds, PlatformOverlay, PlatformOverlayImpl, PlatformResult, WindowId};
use log::{debug, error};
use skia_safe::{surfaces, Color, Surface};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::window::{Window, WindowAttributes, WindowLevel};

pub enum OverlayManagerCommand {
    CreateOverlay {
        id: OverlayId,
        content: Box<dyn OverlayContent>,
        reply: mpsc::UnboundedSender<OverlayId>,
    },
    Command {
        id: OverlayId,
        command: OverlayWindowCommand,
    },
    UpdateContent {
        id: OverlayId,
        content: Box<dyn FnOnce(&mut dyn OverlayContent) + Send>,
    },
    RemoveOverlay {
        id: OverlayId,
    },
    Shutdown,
}

pub struct OverlayManager {
    command_sender: mpsc::UnboundedSender<OverlayManagerCommand>,
    manager_thread: Option<thread::JoinHandle<()>>,
}

impl OverlayManager {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        let manager_thread = thread::spawn(move || {
            // Single tokio runtime for ALL overlays
            let rt = Runtime::new().unwrap();
            rt.block_on(Self::run_manager(command_rx));
        });

        Self {
            command_sender: command_tx,
            manager_thread: Some(manager_thread),
        }
    }

    async fn run_manager(mut command_rx: mpsc::UnboundedReceiver<OverlayManagerCommand>) {
        let mut overlays: HashMap<OverlayId, OverlayState> = HashMap::new();
        let mut next_id: OverlayId = 1;
        let mut running = true;

        let mut frame_timer = Instant::now();
        // Use global overlay animation FPS from config
        let overlay_fps = crate::config::Config::overlay_animation_fps().max(1);
        let target_frame_duration = Duration::from_secs_f64(1.0 / overlay_fps as f64);

        while running {
            // Process all pending commands
            loop {
                let cmd = match command_rx.try_recv() {
                    Ok(cmd) => cmd,
                    Err(_) => break,
                };

                match cmd {
                    OverlayManagerCommand::CreateOverlay { id, content, reply } => {
                        let overlay_id = if id == 0 { next_id } else { id };
                        next_id = next_id.max(overlay_id + 1);

                        match Self::create_overlay_state(overlay_id, content).await {
                            Ok(state) => {
                                debug!(
                                    "Created overlay {} (handle: {:?})",
                                    overlay_id, state.handle
                                );
                                overlays.insert(overlay_id, state);
                                let _ = reply.send(overlay_id);
                            }
                            Err(e) => {
                                error!("Failed to create overlay {}: {}", overlay_id, e);
                                let _ = reply.send(0); // Error signal
                            }
                        }
                    }
                    OverlayManagerCommand::UpdateContent { id, content } => {
                        if let Some(state) = overlays.get_mut(&id) {
                            content(state.content.as_mut());
                            state.needs_render = true;
                        }
                    }
                    OverlayManagerCommand::Command { id, command } => {
                        if let Some(state) = overlays.get_mut(&id) {
                            state.handle_command(command);
                        }
                    }
                    OverlayManagerCommand::RemoveOverlay { id } => {
                        if overlays.remove(&id).is_some() {
                            debug!("Removed overlay {}", id);
                        }
                    }
                    OverlayManagerCommand::Shutdown => {
                        running = false;
                        break;
                    }
                }
            }

            if !running {
                break;
            }

            // Update and render all overlays
            let now = Instant::now();
            let elapsed = now.duration_since(frame_timer);

            if elapsed >= target_frame_duration {
                let mut to_remove = Vec::new();

                for (id, state) in overlays.iter_mut() {
                    if state.is_animating() {
                        state.animate_frame();
                    }

                    if state.needs_render() {
                        if let Err(e) = state.render() {
                            error!("Failed to render overlay {}: {}", id, e);
                            to_remove.push(*id);
                        }
                    }
                }

                // Remove failed overlays
                for id in to_remove {
                    overlays.remove(&id);
                }

                frame_timer = now;
            } else {
                // Sleep until next frame
                tokio::time::sleep(target_frame_duration - elapsed).await;
            }
        }
    }

    async fn create_overlay_state(
        id: OverlayId,
        content: Box<dyn OverlayContent>,
    ) -> PlatformResult<OverlayState> {
        let config = content.config();
        let surface = surfaces::raster_n32_premul(skia_safe::ISize::new(100, 100))
            .ok_or("Could not create surface")?;

        let attributes = WindowAttributes::default()
            .with_position(PhysicalPosition::new(0, 0))
            .with_inner_size(LogicalSize::new(0, 0))
            .with_decorations(false)
            .with_transparent(true)
            .with_blur(true)
            .with_visible(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false);

        let config_clone = config.clone();
        let window = get_event_loop_blocking(move |event_loop| {
            let window = match event_loop.create_window(attributes) {
                Ok(w) => w,
                Err(e) => {
                    error!("Failed to create overlay window: {e}");
                    return None;
                }
            };

            if let Err(e) = PlatformOverlay::initialize_overlay_window(&window, &config_clone) {
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

        let (window, handle) = window.ok_or("Failed to create window on main thread")?;

        Ok(OverlayState {
            id,
            window,
            handle,
            surface,
            last_size: (0, 0),
            config,
            content,
            fade_animator: Animator::new(0.0, 0.0, ease_in_out_cubic),
            move_animator: Animator::new(Bounds::default(), Bounds::default(), ease_in_out_cubic),
            native_fade_animation: false,
            native_move_animation: false,
            visible: false,
            needs_render: false,
            current_bounds: Bounds::default(),
        })
    }

    /// Add a new overlay and return a handle to it
    pub async fn add(
        self: &Arc<Self>,
        content: Box<dyn OverlayContent>,
    ) -> Result<Overlay, String> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let _ = self
            .command_sender
            .send(OverlayManagerCommand::CreateOverlay {
                id: 0, // Auto-assign
                content,
                reply: tx,
            });

        // Wait for overlay to be created
        let id = rx.recv().await.ok_or("Failed to receive overlay ID")?;
        if id == 0 {
            return Err("Failed to create overlay".to_string());
        }

        Ok(Overlay::new(id, self.clone()))
    }

    pub fn send_command(&self, id: OverlayId, command: OverlayWindowCommand) {
        let _ = self
            .command_sender
            .send(OverlayManagerCommand::Command { id, command });
    }

    pub fn update_content<F>(&self, id: OverlayId, f: F)
    where
        F: FnOnce(&mut dyn OverlayContent) + Send + 'static,
    {
        let _ = self
            .command_sender
            .send(OverlayManagerCommand::UpdateContent {
                id,
                content: Box::new(f),
            });
    }

    pub fn remove_overlay(&self, id: OverlayId) {
        let _ = self
            .command_sender
            .send(OverlayManagerCommand::RemoveOverlay { id });
    }
}

impl Drop for OverlayManager {
    fn drop(&mut self) {
        let _ = self.command_sender.send(OverlayManagerCommand::Shutdown);
        if let Some(thread) = self.manager_thread.take() {
            let _ = thread.join();
        }
    }
}

struct OverlayState {
    id: OverlayId,
    window: Window,
    handle: WindowId,
    surface: Surface,
    last_size: (u32, u32),
    config: OverlayWindowConfig,
    content: Box<dyn OverlayContent>,
    fade_animator: Animator<f32>,
    move_animator: Animator<Bounds>,
    native_fade_animation: bool,
    native_move_animation: bool,
    visible: bool,
    needs_render: bool,
    current_bounds: Bounds,
}

impl OverlayState {
    fn handle_command(&mut self, cmd: OverlayWindowCommand) {
        match cmd {
            OverlayWindowCommand::Show => {
                self.visible = true;
                self.content.on_show();
                if self.config.fade_animation_ms == 0 {
                    self.fade_animator.start(1.0, 0);
                } else {
                    self.start_fade(1.0, self.config.fade_animation_ms);
                }
                self.needs_render = true;
            }
            OverlayWindowCommand::Hide => {
                self.content.on_hide();
                if self.config.fade_animation_ms == 0 {
                    self.visible = false;
                } else {
                    self.start_fade(0.0, self.config.fade_animation_ms);
                }
                self.needs_render = true;
            }
            OverlayWindowCommand::MoveTo(bounds) => {
                if bounds != self.current_bounds {
                    self.content.on_bounds_changed(&bounds);
                    self.current_bounds = bounds.clone();
                }
                if self.config.move_animation_ms == 0 {
                    self.set_bounds(bounds.clone());
                    let _ = self.render();
                } else {
                    self.start_move(bounds, self.config.move_animation_ms);
                }
                self.needs_render = true;
            }
            OverlayWindowCommand::Exit => {}
        }
    }

    fn is_animating(&self) -> bool {
        self.fade_animator.is_animating() || self.move_animator.is_animating()
    }

    fn needs_render(&self) -> bool {
        self.needs_render || self.is_animating()
    }

    fn animate_frame(&mut self) {
        if self.fade_animator.is_animating() {
            if let Some(new_opacity) = self.fade_animator.update() {
                if !self.native_fade_animation {
                    let _ = PlatformOverlay::set_window_opacity(self.handle, new_opacity);
                }
                if !self.fade_animator.is_animating() && new_opacity == 0.0 {
                    self.visible = false;
                }
            }
        }

        if self.move_animator.is_animating() {
            if let Some(bounds) = self.move_animator.update() {
                if bounds != self.current_bounds {
                    self.content.on_bounds_changed(&bounds);
                    self.current_bounds = bounds.clone();
                }
                if !self.native_move_animation {
                    self.set_bounds(bounds);
                }
            }
        }

        if !self.move_animator.is_animating()
            && !self.native_move_animation
            && self.move_animator.start_time.is_some()
        {
            let target_bounds = self.move_animator.to.clone();
            if target_bounds != self.current_bounds {
                self.content.on_bounds_changed(&target_bounds);
                self.current_bounds = target_bounds.clone();
            }
            self.set_bounds(target_bounds);
        }

        self.needs_render = true;
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
            self.set_bounds(bounds.clone());
            self.move_animator.start_from(bounds.clone(), bounds, 0);
            let _ = self.render();
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

    fn set_bounds(&mut self, bounds: Bounds) {
        // Update current_bounds immediately so surface size is correct
        if bounds != self.current_bounds {
            self.current_bounds = bounds.clone();
        }

        let _ = PlatformOverlay::set_window_bounds(self.handle, bounds);
    }

    fn ensure_surface_size(&mut self, width: u32, height: u32) {
        if (width, height) != self.last_size && width > 0 && height > 0 {
            self.surface =
                surfaces::raster_n32_premul(skia_safe::ISize::new(width as i32, height as i32))
                    .unwrap();
            self.last_size = (width, height);
        }
    }

    fn draw(&mut self) -> PlatformResult<()> {
        let target_width = self.current_bounds.size.width;
        let target_height = self.current_bounds.size.height;
        self.ensure_surface_size(target_width, target_height);

        let canvas = self.surface.canvas();
        canvas.clear(Color::from_argb(0, 0, 0, 0));
        if !self.visible {
            return Ok(());
        }

        let width = target_width as f32;
        let height = target_height as f32;
        let rect = skia_safe::Rect::from_xywh(0.0, 0.0, width, height);
        let rounded_rect = skia_safe::RRect::new_rect_xy(
            rect,
            self.config.border_radius,
            self.config.border_radius,
        );

        // Draw background
        if let Some(background) = &self.config.background {
            let mut paint = skia_safe::Paint::default();
            paint.set_color(background.color.with_a((background.opacity * 255.0) as u8));
            paint.set_style(skia_safe::PaintStyle::Fill);
            canvas.draw_rrect(&rounded_rect, &paint);
        }

        // Draw border
        if let Some(border) = &self.config.border {
            if border.width > 0 {
                let mut border_paint = skia_safe::Paint::default();
                border_paint.set_color(border.color);
                border_paint.set_style(skia_safe::PaintStyle::Stroke);
                border_paint.set_stroke_width(border.width as f32);
                canvas.draw_rrect(&rounded_rect, &border_paint);
            }
        }

        // Draw content
        self.content.draw(canvas, &self.current_bounds)?;

        Ok(())
    }

    fn render(&mut self) -> PlatformResult<()> {
        self.draw()?;
        PlatformOverlay::render_to_window(&self.surface.image_snapshot(), self.handle)?;
        self.needs_render = false;
        Ok(())
    }
}
