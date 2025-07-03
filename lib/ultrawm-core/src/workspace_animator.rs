use crate::animation::{ease_in_out_cubic, Animator};
use crate::coalescing_channel::CoalescingAsyncChannel;
use crate::platform::{Bounds, PlatformWindow, PlatformWindowImpl, WindowId};
use log::{error, warn};
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum WorkspaceAnimationCommand {
    AnimateWindow {
        window_id: WindowId,
        platform_window: PlatformWindow,
        from_bounds: Bounds,
        to_bounds: Bounds,
        duration_ms: u32,
    },
    StopWindow(WindowId),
    RemoveWindow(WindowId),
    Exit,
}

#[derive(Debug, Clone)]
pub struct WorkspaceAnimationConfig {
    pub animation_fps: u32,
}

impl Default for WorkspaceAnimationConfig {
    fn default() -> Self {
        Self { animation_fps: 30 }
    }
}

struct AnimatedWindow {
    platform_window: PlatformWindow,
    animator: Animator<Bounds>,
}

pub struct WorkspaceAnimationThread {
    config: WorkspaceAnimationConfig,
    command_sender: mpsc::UnboundedSender<WorkspaceAnimationCommand>,
    animator_thread: Option<thread::JoinHandle<()>>,
}

struct WorkspaceAnimationThreadAnimator {
    config: WorkspaceAnimationConfig,
    animated_windows: HashMap<WindowId, AnimatedWindow>,
    command_channel: CoalescingAsyncChannel<WorkspaceAnimationCommand>,
}

impl WorkspaceAnimationThread {
    pub fn new(config: WorkspaceAnimationConfig) -> Self {
        let command_channel = CoalescingAsyncChannel::new();
        let command_sender = command_channel.sender();

        let config_clone = config.clone();
        let animator_thread = thread::spawn(move || {
            let mut animator = WorkspaceAnimationThreadAnimator {
                config: config_clone,
                animated_windows: HashMap::new(),
                command_channel,
            };

            // Create a tokio runtime for the animator thread
            let rt = Runtime::new().unwrap();
            rt.block_on(animator.run_loop());
        });

        Self {
            config,
            command_sender,
            animator_thread: Some(animator_thread),
        }
    }

    pub fn animate_window(
        &mut self,
        window_id: WindowId,
        platform_window: PlatformWindow,
        from_bounds: Bounds,
        to_bounds: Bounds,
        duration_ms: u32,
    ) {
        if let Err(e) = self
            .command_sender
            .send(WorkspaceAnimationCommand::AnimateWindow {
                window_id,
                platform_window,
                from_bounds,
                to_bounds,
                duration_ms,
            })
        {
            error!("Failed to send AnimateWindow command to workspace animation thread: {e}");
        }
    }

    /// Stop animating a specific window
    pub fn stop_window(&mut self, window_id: WindowId) {
        if let Err(e) = self
            .command_sender
            .send(WorkspaceAnimationCommand::StopWindow(window_id))
        {
            warn!("Failed to send StopWindow command to workspace animation thread: {e}");
        }
    }

    /// Remove a window from the animation thread
    pub fn remove_window(&mut self, window_id: WindowId) {
        if let Err(e) = self
            .command_sender
            .send(WorkspaceAnimationCommand::RemoveWindow(window_id))
        {
            warn!("Failed to send RemoveWindow command to workspace animation thread: {e}");
        }
    }
}

impl Drop for WorkspaceAnimationThread {
    fn drop(&mut self) {
        let _ = self.command_sender.send(WorkspaceAnimationCommand::Exit);
        if let Some(thread) = self.animator_thread.take() {
            let _ = thread.join();
        }
    }
}

impl WorkspaceAnimationThreadAnimator {
    async fn run_loop(&mut self) {
        let mut running = true;
        let frame_duration = Duration::from_secs_f64(1.0 / self.config.animation_fps as f64);
        let mut last_frame_time = Instant::now();

        while running {
            while let Some(cmd) = self.command_channel.try_coalesce(|_| false) {
                self.handle_command(cmd, &mut running);
            }

            if !running {
                break;
            }

            if !self.animated_windows.is_empty() {
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

    fn handle_command(&mut self, cmd: WorkspaceAnimationCommand, running: &mut bool) {
        match cmd {
            WorkspaceAnimationCommand::AnimateWindow {
                window_id,
                platform_window,
                from_bounds,
                to_bounds,
                duration_ms,
            } => {
                let mut animator =
                    Animator::new(from_bounds.clone(), to_bounds.clone(), ease_in_out_cubic);
                animator.start_from(from_bounds, to_bounds, duration_ms);

                let animated_window = AnimatedWindow {
                    platform_window,
                    animator,
                };

                self.animated_windows.insert(window_id, animated_window);
            }
            WorkspaceAnimationCommand::StopWindow(window_id) => {
                if let Some(animated_window) = self.animated_windows.get_mut(&window_id) {
                    // Stop animation by setting it to not animating
                    animated_window.animator = Animator::new(
                        animated_window.animator.current_value().clone(),
                        animated_window.animator.current_value().clone(),
                        ease_in_out_cubic,
                    );
                }
            }
            WorkspaceAnimationCommand::RemoveWindow(window_id) => {
                self.animated_windows.remove(&window_id);
            }
            WorkspaceAnimationCommand::Exit => {
                *running = false;
            }
        }
    }

    fn animate_frame(&mut self) {
        let mut completed_windows = Vec::new();

        for (window_id, animated_window) in self.animated_windows.iter_mut() {
            if let Some(new_bounds) = animated_window.animator.update() {
                if let Err(e) = animated_window.platform_window.set_bounds(&new_bounds) {
                    warn!("Failed to set bounds for window {}: {}", window_id, e);
                }
            }

            if !animated_window.animator.is_animating() {
                completed_windows.push(*window_id);
            }
        }

        for window_id in completed_windows {
            self.animated_windows.remove(&window_id);
        }
    }
}
