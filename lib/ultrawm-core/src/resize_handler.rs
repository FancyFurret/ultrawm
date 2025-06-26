use crate::config::Config;
use crate::event_loop_wm::{WMOperationError, WMOperationResult};
use crate::overlay_window::{
    OverlayWindow, OverlayWindowBackgroundStyle, OverlayWindowBorderStyle, OverlayWindowConfig,
};
use crate::platform::traits::PlatformImpl;
use crate::platform::{Bounds, CursorType, MouseButtons, Platform, PlatformEvent, Position};
use crate::resize_handle::ResizeHandle;
use crate::resize_handle_tracker::{HandleDragEvent, HandleDragTracker};
use crate::wm::WindowManager;
use skia_safe::Color;

pub struct WindowResizeHandler {
    overlay: OverlayWindow,
    last_preview_bounds: Option<Bounds>,
    handle_tracker: HandleDragTracker,
    handle_drag_active: bool,
    active_resize_handle: Option<ResizeHandle>,
    hover_resize_handle: Option<ResizeHandle>,
    handles_enabled: bool,
    handle_width: u32,
}

impl WindowResizeHandler {
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
            last_preview_bounds: None,
            handle_tracker: HandleDragTracker::new(),
            handle_drag_active: false,
            active_resize_handle: None,
            hover_resize_handle: None,
            handles_enabled: config.resize_handles,
            handle_width: config.resize_handle_width,
        }
    }

    pub fn handle_event(
        &mut self,
        event: &PlatformEvent,
        wm: &mut WindowManager,
    ) -> WMOperationResult<bool> {
        if !self.handles_enabled {
            return Ok(false);
        }

        match &event {
            PlatformEvent::MouseMoved(pos) => self.mouse_moved(pos, wm),
            _ => Ok(()),
        }?;

        match self.handle_tracker.handle_event(&event, &wm) {
            Some(HandleDragEvent::Start(handle, pos, _)) => {
                self.start(handle, pos)?;
                Ok(true)
            }
            Some(HandleDragEvent::Drag(handle, pos, buttons)) => {
                self.drag(handle, pos, buttons, wm)?;
                Ok(true)
            }
            Some(HandleDragEvent::End(handle, pos, buttons)) => {
                self.drop(handle, pos, buttons, wm)?;
                Ok(true)
            }
            None => Ok(false),
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

            self.overlay.show();
            self.overlay.move_to(&preview_bounds);
            self.last_preview_bounds = Some(preview_bounds);
        } else if (handle_under_cursor.is_none()) && self.hover_resize_handle.is_some() {
            self.hover_resize_handle = None;
            if !self.handle_drag_active {
                self.overlay.hide();
                self.last_preview_bounds = None;
            }
        }

        Ok(())
    }

    fn start(&mut self, handle: ResizeHandle, _pos: Position) -> WMOperationResult<()> {
        self.handle_drag_active = true;
        self.active_resize_handle = Some(handle.clone());

        let preview_bounds = handle.preview_bounds(self.handle_width);
        self.overlay.show();
        self.overlay.move_to(&preview_bounds);
        self.last_preview_bounds = Some(preview_bounds);

        Ok(())
    }

    fn drag(
        &mut self,
        handle: ResizeHandle,
        pos: Position,
        buttons: MouseButtons,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        if self.handle_drag_active {
            let mut preview_bounds = handle.preview_bounds(self.handle_width);
            if handle.orientation == crate::resize_handle::HandleOrientation::Vertical {
                let clamped_x = handle.clamp_coordinate(pos.x);
                preview_bounds.position.x = clamped_x - (preview_bounds.size.width as i32 / 2);
            } else {
                let clamped_y = handle.clamp_coordinate(pos.y);
                preview_bounds.position.y = clamped_y - (preview_bounds.size.height as i32 / 2);
            }

            if Some(&preview_bounds) != self.last_preview_bounds.as_ref() {
                self.overlay.show();
                self.overlay.move_to(&preview_bounds);
                self.last_preview_bounds = Some(preview_bounds);
            }

            if Config::current().live_window_resize {
                wm.resize_handle_moved(&handle, &pos, &buttons)?;
            }
        }

        Ok(())
    }

    fn drop(
        &mut self,
        handle: ResizeHandle,
        pos: Position,
        buttons: MouseButtons,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        if self.handle_drag_active {
            self.overlay.hide();
            self.last_preview_bounds = None;
            self.handle_drag_active = false;
            wm.resize_handle_moved(&handle, &pos, &buttons)?;
        }

        Ok(())
    }
}
