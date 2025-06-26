use crate::drag_tracker::{WindowDragEvent, WindowDragTracker, WindowDragType};
use crate::event_loop_wm::WMOperationResult;
use crate::platform::{Position, WMEvent, WindowId};
use crate::tile_preview_handler::TilePreviewHandler;
use crate::wm::WindowManager;

pub struct WindowMoveHandler {
    preview: TilePreviewHandler,
    drag_tracker: WindowDragTracker,
}

impl WindowMoveHandler {
    pub async fn new() -> Self {
        Self {
            preview: TilePreviewHandler::new().await,
            drag_tracker: WindowDragTracker::new(),
        }
    }

    pub fn overlay_shown(&self) -> bool {
        self.preview.is_shown()
    }

    pub fn handle_event(
        &mut self,
        event: &WMEvent,
        wm: &mut WindowManager,
    ) -> WMOperationResult<bool> {
        match self.drag_tracker.handle_event(&event, &wm) {
            Some(WindowDragEvent::Drag(id, position, drag_type)) => {
                self.drag(id, position, drag_type, wm)?;
                Ok(true)
            }
            Some(WindowDragEvent::End(id, position, drag_type)) => {
                self.drop(id, position, drag_type, wm)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn drag(
        &mut self,
        id: WindowId,
        position: Position,
        drag_type: WindowDragType,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        if drag_type == WindowDragType::Move {
            self.preview.update_preview(id, &position, wm);
        }
        Ok(())
    }

    fn drop(
        &mut self,
        id: WindowId,
        position: Position,
        drag_type: WindowDragType,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        if drag_type == WindowDragType::Move {
            TilePreviewHandler::tile_on_drop(&mut self.preview, id, &position, wm)?;
        }
        Ok(())
    }
}
