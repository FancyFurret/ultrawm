use crate::event_loop_wm::{WMOperationError, WMOperationResult};
use crate::layouts::container_tree::ResizeDirection;
use crate::platform::{Bounds, PlatformEvent, Position, WindowId};
use crate::tile_preview_handler::TilePreviewHandler;
use crate::window_area_tracker::{WindowAreaDragEvent, WindowAreaDragType, WindowAreaTracker};
use crate::wm::WindowManager;

pub struct WindowAreaHandler {
    preview: TilePreviewHandler,
    tracker: WindowAreaTracker,
    resize_direction: Option<ResizeDirection>,
}

impl WindowAreaHandler {
    pub async fn new() -> Self {
        Self {
            preview: TilePreviewHandler::new().await,
            tracker: WindowAreaTracker::new(),
            resize_direction: None,
        }
    }

    pub fn handle_event(
        &mut self,
        event: &PlatformEvent,
        wm: &mut WindowManager,
    ) -> WMOperationResult<bool> {
        match self.tracker.handle_event(event, wm) {
            Some(WindowAreaDragEvent::Start(id, pos, drag_type)) => {
                self.start(id, pos, drag_type, wm)?;
                Ok(true)
            }
            Some(WindowAreaDragEvent::Drag(id, pos, drag_type)) => {
                self.drag(id, pos, drag_type, wm)?;
                Ok(true)
            }
            Some(WindowAreaDragEvent::End(id, pos, drag_type)) => {
                self.drop(id, pos, drag_type, wm)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn start(
        &mut self,
        id: WindowId,
        pos: Position,
        drag_type: WindowAreaDragType,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        match drag_type {
            WindowAreaDragType::Tile => {
                self.preview.update_preview(id, &pos, wm);
            }
            WindowAreaDragType::Resize | WindowAreaDragType::ResizeSymmetric => {
                let window = wm.get_window(id)?;
                let bounds = window.bounds();
                let direction = Self::resize_direction(&bounds, &pos);
                self.resize_direction = Some(direction);
            }
            _ => {}
        }
        Ok(())
    }

    fn drag(
        &mut self,
        id: WindowId,
        pos: Position,
        drag_type: WindowAreaDragType,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        let start = self.tracker.get_drag_start(id);
        if start.is_none() {
            return Ok(());
        }
        let (start_pos, start_bounds) = start.unwrap();
        let window = wm.get_window(id)?;

        match drag_type {
            WindowAreaDragType::Tile => {
                self.preview.update_preview(id, &pos, wm);
                let dx = pos.x - start_pos.x;
                let dy = pos.y - start_pos.y;
                let mut new_bounds = start_bounds.clone();
                new_bounds.position.x += dx;
                new_bounds.position.y += dy;
                let _ = window.set_bounds_immediate(new_bounds);
            }
            WindowAreaDragType::Resize => {
                if let Some(direction) = self.resize_direction {
                    let new_bounds =
                        Self::calculate_resize_bounds(&start_bounds, &start_pos, &pos, direction);
                    wm.resize_window(id, &new_bounds)
                        .map_err(WMOperationError::Resize)?;
                }
            }
            WindowAreaDragType::ResizeSymmetric => {
                if let Some(direction) = self.resize_direction {
                    let new_bounds = Self::calculate_resize_bounds_symmetric(
                        &start_bounds,
                        &start_pos,
                        &pos,
                        direction,
                    );
                    wm.resize_window(id, &new_bounds)
                        .map_err(WMOperationError::Resize)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn resize_direction(bounds: &Bounds, pos: &Position) -> ResizeDirection {
        if let Some(dir) = Self::detect_corner_wedge(bounds, pos) {
            return dir;
        }
        Self::detect_edge(bounds, pos)
    }

    fn detect_corner_wedge(bounds: &Bounds, pos: &Position) -> Option<ResizeDirection> {
        let left = bounds.position.x;
        let right = bounds.position.x + bounds.size.width as i32;
        let top = bounds.position.y;
        let bottom = bounds.position.y + bounds.size.height as i32;
        let w = bounds.size.width as f32;
        let h = bounds.size.height as f32;
        let corner_w = (w * 0.25).round() as i32;
        let corner_h = (h * 0.25).round() as i32;

        // Top-left
        if pos.x >= left && pos.x < left + corner_w && pos.y >= top && pos.y < top + corner_h {
            return Some(ResizeDirection::TopLeft);
        }
        // Top-right
        if pos.x <= right && pos.x > right - corner_w && pos.y >= top && pos.y < top + corner_h {
            return Some(ResizeDirection::TopRight);
        }
        // Bottom-left
        if pos.x >= left && pos.x < left + corner_w && pos.y <= bottom && pos.y > bottom - corner_h
        {
            return Some(ResizeDirection::BottomLeft);
        }
        // Bottom-right
        if pos.x <= right
            && pos.x > right - corner_w
            && pos.y <= bottom
            && pos.y > bottom - corner_h
        {
            return Some(ResizeDirection::BottomRight);
        }
        None
    }

    fn detect_edge(bounds: &Bounds, pos: &Position) -> ResizeDirection {
        let left = bounds.position.x;
        let right = bounds.position.x + bounds.size.width as i32;
        let top = bounds.position.y;
        let bottom = bounds.position.y + bounds.size.height as i32;
        let left_dist = (pos.x - left).abs();
        let right_dist = (pos.x - right).abs();
        let top_dist = (pos.y - top).abs();
        let bottom_dist = (pos.y - bottom).abs();
        let mut min_dist = left_dist;
        let mut dir = ResizeDirection::Left;
        if right_dist < min_dist {
            min_dist = right_dist;
            dir = ResizeDirection::Right;
        }
        if top_dist < min_dist {
            min_dist = top_dist;
            dir = ResizeDirection::Top;
        }
        if bottom_dist < min_dist {
            dir = ResizeDirection::Bottom;
        }
        dir
    }

    fn calculate_resize_bounds(
        start_bounds: &Bounds,
        start_pos: &Position,
        current_pos: &Position,
        direction: ResizeDirection,
    ) -> Bounds {
        let mut new_bounds = start_bounds.clone();
        let dx = current_pos.x - start_pos.x;
        let dy = current_pos.y - start_pos.y;

        if direction.has_left() {
            new_bounds.position.x += dx;
            new_bounds.size.width = (start_bounds.size.width as i32 - dx).max(1) as u32;
        }
        if direction.has_right() {
            new_bounds.size.width = (start_bounds.size.width as i32 + dx).max(1) as u32;
        }
        if direction.has_top() {
            new_bounds.position.y += dy;
            new_bounds.size.height = (start_bounds.size.height as i32 - dy).max(1) as u32;
        }
        if direction.has_bottom() {
            new_bounds.size.height = (start_bounds.size.height as i32 + dy).max(1) as u32;
        }
        new_bounds
    }

    fn calculate_resize_bounds_symmetric(
        start_bounds: &Bounds,
        start_pos: &Position,
        current_pos: &Position,
        direction: ResizeDirection,
    ) -> Bounds {
        let mut new_bounds = start_bounds.clone();
        let dx = current_pos.x - start_pos.x;
        let dy = current_pos.y - start_pos.y;

        // For each direction, grow/shrink symmetrically
        if direction == ResizeDirection::Left {
            new_bounds.position.x += dx;
            new_bounds.size.width = (start_bounds.size.width as i32 - 2 * dx).max(1) as u32;
        } else if direction == ResizeDirection::Right {
            new_bounds.position.x -= dx;
            new_bounds.size.width = (start_bounds.size.width as i32 + 2 * dx).max(1) as u32;
        } else if direction == ResizeDirection::Top {
            new_bounds.position.y += dy;
            new_bounds.size.height = (start_bounds.size.height as i32 - 2 * dy).max(1) as u32;
        } else if direction == ResizeDirection::Bottom {
            new_bounds.position.y -= dy;
            new_bounds.size.height = (start_bounds.size.height as i32 + 2 * dy).max(1) as u32;
        } else {
            let (sign_x, sign_y) = match direction {
                ResizeDirection::TopLeft => (-1, -1),
                ResizeDirection::TopRight => (1, -1),
                ResizeDirection::BottomLeft => (-1, 1),
                ResizeDirection::BottomRight => (1, 1),
                _ => (0, 0),
            };
            if sign_x != 0 {
                new_bounds.position.x -= sign_x * dx;
                new_bounds.size.width =
                    (start_bounds.size.width as i32 + 2 * sign_x * dx).max(1) as u32;
            }
            if sign_y != 0 {
                new_bounds.position.y -= sign_y * dy;
                new_bounds.size.height =
                    (start_bounds.size.height as i32 + 2 * sign_y * dy).max(1) as u32;
            }
        }
        new_bounds
    }

    fn drop(
        &mut self,
        id: WindowId,
        pos: Position,
        drag_type: WindowAreaDragType,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        match drag_type {
            WindowAreaDragType::Tile => {
                TilePreviewHandler::tile_on_drop(&mut self.preview, id, &pos, wm)?;
            }
            WindowAreaDragType::Resize | WindowAreaDragType::ResizeSymmetric => {
                self.resize_direction = None;
            }
            WindowAreaDragType::Slide => {
                // TODO: Implement slide logic
            }
        }
        Ok(())
    }
}
