use crate::event_handlers::native_transform_tracker::{
    NativeTransformTracker, WindowDragEvent, WindowDragType,
};
use crate::event_handlers::EventHandler;
use crate::event_loop_wm::WMOperationResult;
use crate::platform::{Position, WMEvent, WindowId};
use crate::tile_preview_handler::TilePreviewHandler;
use crate::wm::WindowManager;
use log::debug;

pub struct NativeTransformHandler {
    preview: TilePreviewHandler,
    tracker: NativeTransformTracker,
}

impl NativeTransformHandler {
    pub async fn new() -> Self {
        Self {
            preview: TilePreviewHandler::new().await,
            tracker: NativeTransformTracker::new(),
        }
    }

    pub fn overlay_shown(&self) -> bool {
        self.preview.is_shown()
    }

    fn drag(
        &mut self,
        id: WindowId,
        position: Position,
        drag_type: WindowDragType,
        wm: &mut WindowManager,
    ) -> WMOperationResult<()> {
        let window = wm.get_window(id)?;
        if window.floating() {
            window.update_bounds();
            wm.update_floating_window(window.id())?;
            return Ok(());
        }

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
        debug!(
            "Native drop: id={} pos={:?} type={:?}",
            id, position, drag_type
        );
        let window = wm.get_window(id)?;
        if window.floating() {
            return Ok(());
        }

        if drag_type == WindowDragType::Move {
            TilePreviewHandler::tile_on_drop(&mut self.preview, id, &position, wm)?;
        } else if let WindowDragType::Resize(_) = drag_type {
            let window = wm.get_window(id)?;
            let bounds = window.platform_bounds();
            wm.resize_window(id, &bounds)?;
        }
        Ok(())
    }
}

impl EventHandler for NativeTransformHandler {
    fn handle_event(&mut self, event: &WMEvent, wm: &mut WindowManager) -> WMOperationResult<bool> {
        match self.tracker.handle_event(&event, &wm) {
            Some(WindowDragEvent::Drag(id, position, drag_type)) => {
                self.drag(id, position, drag_type, wm)?;
                Ok(true)
            }
            Some(WindowDragEvent::End(id, position, drag_type)) => {
                self.drop(id, position, drag_type, wm)?;
                Ok(true)
            }
            _ => Ok(self.tracker.active()),
        }
    }
}
