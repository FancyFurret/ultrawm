use crate::event_handlers::mod_transform_tracker::{
    ModTransformDragEvent, ModTransformTracker, ModTransformType,
};
use crate::event_handlers::EventHandler;
use crate::event_loop_wm::{WMOperationError, WMOperationResult};
use crate::layouts::container_tree::ResizeDirection;
use crate::platform::traits::PlatformImpl;
use crate::platform::{Bounds, CursorType, Platform, Position, WMEvent, WindowId};
use crate::tile_preview_handler::TilePreviewHandler;
use crate::wm::WindowManager;
use log::warn;

pub struct ModTransformHandler {
    preview: TilePreviewHandler,
    tracker: ModTransformTracker,
    resize_direction: Option<ResizeDirection>,
    drag_start_position: Option<Position>,
    drag_start_bounds: Option<Bounds>,
    drag_type: Option<ModTransformType>,
}

impl ModTransformHandler {
    pub async fn new() -> Self {
        Self {
            preview: TilePreviewHandler::new().await,
            tracker: ModTransformTracker::new(),
            resize_direction: None,
            drag_start_position: None,
            drag_start_bounds: None,
            drag_type: None,
        }
    }

    fn start(
        &mut self,
        id: WindowId,
        pos: Position,
        drag_type: ModTransformType,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        let window = wm.get_window(id)?;
        let bounds = window.bounds();
        let floating = window.floating();
        let tiled = !floating;

        // Store start position and bounds for this drag
        self.drag_start_position = Some(pos.clone());
        self.drag_start_bounds = Some(bounds.clone());

        // Determine the drag type
        if drag_type == ModTransformType::Tile
            || (drag_type == ModTransformType::Shift && tiled)
            || (drag_type == ModTransformType::Toggle && floating)
        {
            self.drag_type = Some(ModTransformType::Tile);
        } else if drag_type == ModTransformType::Float
            || (drag_type == ModTransformType::Shift && floating)
            || (drag_type == ModTransformType::Toggle && tiled)
        {
            self.drag_type = Some(ModTransformType::Float);
            if tiled {
                wm.float_window(window.id())?;
            }
        } else {
            self.drag_type = Some(drag_type.clone());
        }

        // Set appropriate cursor for the drag type
        let cursor_type = match &drag_type {
            ModTransformType::Tile
            | ModTransformType::Float
            | ModTransformType::Shift
            | ModTransformType::Toggle
            | ModTransformType::Slide => CursorType::Move,
            ModTransformType::Resize(direction) | ModTransformType::ResizeSymmetric(direction) => {
                self.resize_direction = Some(*direction);
                Self::cursor_for_resize_mode(*direction)
            }
        };

        Platform::set_cursor(cursor_type).unwrap_or_else(|e| {
            warn!("Failed to set cursor: {}", e);
        });
        Ok(())
    }

    fn drag(
        &mut self,
        id: WindowId,
        pos: Position,
        drag_type: ModTransformType,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        let start_pos = match &self.drag_start_position {
            Some(pos) => pos.clone(),
            None => return Ok(()), // No drag in progress
        };
        let start_bounds = match &self.drag_start_bounds {
            Some(bounds) => bounds.clone(),
            None => return Ok(()), // No drag in progress
        };
        let move_bounds = Self::calculate_move_bounds(&start_bounds, &start_pos, &pos);
        let window = wm.get_window(id)?;

        match drag_type {
            ModTransformType::Tile => {
                self.preview.update_preview(id, &pos, wm);
                let _ = window.set_preview_bounds(move_bounds.clone());
            }
            ModTransformType::Float => {
                window.set_bounds(move_bounds.clone());
                window.flush().unwrap_or_else(|e| {
                    warn!("Failed to flush window: {}", e);
                });
                wm.update_floating_window(window.id())?;
            }
            ModTransformType::Slide => {
                wm.resize_window(id, &move_bounds)
                    .map_err(WMOperationError::Resize)?;

                if window.floating() {
                    wm.update_floating_window(window.id())?;
                }
            }
            ModTransformType::Resize(direction) => {
                let new_bounds =
                    Self::calculate_resize_bounds(&start_bounds, &start_pos, &pos, direction);
                wm.resize_window(id, &new_bounds)
                    .map_err(WMOperationError::Resize)?;
            }
            ModTransformType::ResizeSymmetric(direction) => {
                let new_bounds = Self::calculate_resize_bounds_symmetric(
                    &start_bounds,
                    &start_pos,
                    &pos,
                    direction,
                );
                wm.resize_window(id, &new_bounds)
                    .map_err(WMOperationError::Resize)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn cursor_for_resize_mode(direction: ResizeDirection) -> CursorType {
        match direction {
            ResizeDirection::Top | ResizeDirection::Bottom => CursorType::ResizeNorth,
            ResizeDirection::Left | ResizeDirection::Right => CursorType::ResizeEast,
            ResizeDirection::TopLeft | ResizeDirection::BottomRight => CursorType::ResizeNorthWest,
            ResizeDirection::TopRight | ResizeDirection::BottomLeft => CursorType::ResizeNorthEast,
        }
    }

    fn calculate_move_bounds(
        start_bounds: &Bounds,
        start_pos: &Position,
        pos: &Position,
    ) -> Bounds {
        let dx = pos.x - start_pos.x;
        let dy = pos.y - start_pos.y;
        let mut new_bounds = start_bounds.clone();
        new_bounds.position.x += dx;
        new_bounds.position.y += dy;
        new_bounds
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
        drag_type: ModTransformType,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        if drag_type == ModTransformType::Tile {
            self.preview.tile_on_drop(id, &pos, wm)?;
        }

        Ok(())
    }

    fn cancel(
        &mut self,
        id: WindowId,
        drag_type: ModTransformType,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        let window = wm.get_window(id)?;
        if drag_type == ModTransformType::Tile {
            if window.floating() {
                window.update_bounds();
            }
            self.preview.cancel(id, wm)?;
        }

        Ok(())
    }

    fn finalize(&mut self, drag_type: ModTransformType) {
        match drag_type {
            ModTransformType::Resize(_) | ModTransformType::ResizeSymmetric(_) => {
                self.resize_direction = None;
            }
            _ => {}
        }

        self.preview.hide();
        self.drag_start_position = None;
        self.drag_start_bounds = None;
        self.drag_type = None;

        // Reset cursor to default
        Platform::reset_cursor().unwrap_or_else(|e| {
            warn!("Failed to reset cursor: {}", e);
        });
    }
}

impl EventHandler for ModTransformHandler {
    fn handle_event(&mut self, event: &WMEvent, wm: &mut WindowManager) -> WMOperationResult<bool> {
        let events = self.tracker.handle_event(event, wm);

        for drag_event in events {
            match drag_event {
                ModTransformDragEvent::Start(id, pos, drag_type) => {
                    self.start(id, pos, drag_type, wm)?
                }
                ModTransformDragEvent::Drag(id, pos, drag_type) => {
                    self.drag(id, pos, self.drag_type.clone().unwrap_or(drag_type), wm)?
                }
                ModTransformDragEvent::End(id, pos, drag_type) => {
                    self.drop(
                        id,
                        pos,
                        self.drag_type.clone().unwrap_or(drag_type.clone()),
                        wm,
                    )?;
                    self.finalize(self.drag_type.clone().unwrap_or(drag_type.clone()));
                }
                ModTransformDragEvent::Cancel(id, drag_type) => {
                    self.cancel(id, self.drag_type.clone().unwrap_or(drag_type.clone()), wm)?;
                    self.finalize(self.drag_type.clone().unwrap_or(drag_type.clone()));
                }
            }
        }

        Ok(self.tracker.active())
    }
}
