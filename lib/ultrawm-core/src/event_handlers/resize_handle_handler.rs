use crate::config::Config;
use crate::event_handlers::resize_handle_tracker::{ResizeHandleEvent, ResizeHandleTracker};
use crate::event_handlers::EventHandler;
use crate::event_loop_wm::{WMOperationError, WMOperationResult};
use crate::overlay_window::{
    OverlayWindow, OverlayWindowBackgroundStyle, OverlayWindowBorderStyle, OverlayWindowConfig,
};
use crate::platform::input_state::InputState;
use crate::platform::traits::PlatformImpl;
use crate::platform::{CursorType, Platform, Position, WMEvent};
use crate::resize_handle::{ResizeHandle, ResizeMode};
use crate::wm::WindowManager;
use skia_safe::Color;

pub struct ResizeHandleHandler {
    overlay: OverlayWindow,
    tracker: ResizeHandleTracker,
    hover_resize_handle: Option<ResizeHandle>,
    handles_enabled: bool,
    handle_width: u32,
}

impl ResizeHandleHandler {
    pub async fn new() -> Self {
        let config = Config::current();

        let overlay = OverlayWindow::new(OverlayWindowConfig {
            fade_animation_ms: if config.tile_preview_fade_animate {
                config.tile_preview_animation_ms
            } else {
                0
            },
            move_animation_ms: 0,
            animation_fps: config.tile_preview_fps,
            border_radius: 20.0,
            blur: true,
            background: Some(OverlayWindowBackgroundStyle {
                color: Color::from_rgb(35, 35, 35),
                opacity: 0.75,
            }),
            border: Some(OverlayWindowBorderStyle {
                width: 10,
                color: Color::from_rgb(30, 30, 30),
            }),
        })
        .await;

        Self {
            overlay,
            tracker: ResizeHandleTracker::new(),
            hover_resize_handle: None,
            handles_enabled: config.resize_handles,
            handle_width: config.resize_handle_width,
        }
    }

    fn mouse_moved(&mut self, pos: &Position, wm: &WindowManager) -> WMOperationResult<()> {
        // Prevent normal window resizing
        if let Some(_) = wm.find_window_at_resize_edge(pos) {
            Platform::set_cursor(CursorType::Normal)
                .map_err(|e| WMOperationError::Error(e.into()))?;
            // PlatformEvents::set_intercept_clicks(true)?;
        } else {
            Platform::reset_cursor().map_err(|e| WMOperationError::Error(e.into()))?;
            // PlatformEvents::set_intercept_clicks(false)?;
        }

        let handle_under_cursor = wm.resize_handle_at_position(pos);
        if handle_under_cursor.is_some() && self.hover_resize_handle.is_none() {
            self.hover_resize_handle = handle_under_cursor.clone();

            let preview_bounds = handle_under_cursor
                .as_ref()
                .unwrap()
                .preview_bounds(self.handle_width);

            self.overlay.move_to(&preview_bounds);
            self.overlay.show();
        } else if (handle_under_cursor.is_none()) && self.hover_resize_handle.is_some() {
            self.hover_resize_handle = None;
            if !self.tracker.active() {
                self.overlay.hide();
            }
        }

        Ok(())
    }

    fn start(&mut self, handle: ResizeHandle, _pos: Position) -> WMOperationResult<()> {
        let preview_bounds = handle.preview_bounds(self.handle_width);
        self.overlay.move_to(&preview_bounds);
        self.overlay.show();

        Ok(())
    }

    fn drag(
        &mut self,
        handle: ResizeHandle,
        pos: Position,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        let mut preview_bounds = handle.preview_bounds(self.handle_width);
        if handle.orientation == crate::resize_handle::HandleOrientation::Vertical {
            let clamped_x = handle.clamp_coordinate(pos.x);
            preview_bounds.position.x = clamped_x - (preview_bounds.size.width as i32 / 2);
        } else {
            let clamped_y = handle.clamp_coordinate(pos.y);
            preview_bounds.position.y = clamped_y - (preview_bounds.size.height as i32 / 2);
        }

        self.overlay.move_to(&preview_bounds);
        self.overlay.show();

        if Config::current().live_window_resize {
            if let Some(mode) = Self::get_mode() {
                wm.resize_handle_moved(&handle, &pos, &mode)?;
            }
        }

        Ok(())
    }

    fn drop(
        &mut self,
        handle: ResizeHandle,
        pos: Position,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        self.overlay.hide();

        if let Some(mode) = Self::get_mode() {
            wm.resize_handle_moved(&handle, &pos, &mode)?;
        }

        Ok(())
    }

    fn get_mode() -> Option<ResizeMode> {
        let config = Config::current();
        let binds = config.resize_handle_bindings.clone();
        // if binds.resize_evenly.matches_buttons(buttons) {
        if InputState::binding_matches_mouse(&binds.resize_evenly) {
            Some(ResizeMode::Evenly)
        } else if InputState::binding_matches_mouse(&binds.resize_before) {
            Some(ResizeMode::Before)
        } else if InputState::binding_matches_mouse(&binds.resize_after) {
            Some(ResizeMode::After)
        } else if InputState::binding_matches_mouse(&binds.resize_before_symmetric) {
            Some(ResizeMode::BeforeSymmetric)
        } else if InputState::binding_matches_mouse(&binds.resize_after_symmetric) {
            Some(ResizeMode::AfterSymmetric)
        } else {
            None
        }
    }
}

impl EventHandler for ResizeHandleHandler {
    fn handle_event(&mut self, event: &WMEvent, wm: &mut WindowManager) -> WMOperationResult<bool> {
        if !self.handles_enabled {
            return Ok(false);
        }

        match &event {
            WMEvent::MouseMoved(pos) => self.mouse_moved(pos, wm),
            _ => Ok(()),
        }?;

        match self.tracker.handle_event(&event, &wm) {
            Some(ResizeHandleEvent::Start(handle, pos, _)) => {
                self.start(handle, pos)?;
                Ok(true)
            }
            Some(ResizeHandleEvent::Drag(handle, pos, _)) => {
                self.drag(handle, pos, wm)?;
                Ok(true)
            }
            Some(ResizeHandleEvent::End(handle, pos, _)) => {
                self.drop(handle, pos, wm)?;
                Ok(true)
            }
            None => Ok(self.tracker.active()),
        }
    }
}
