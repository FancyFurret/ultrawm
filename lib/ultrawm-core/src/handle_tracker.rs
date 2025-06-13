use crate::drag_handle::DragHandle;
use crate::platform::{MouseButton, PlatformEvent, Position};
use crate::wm::WindowManager;

#[derive(Debug, Clone)]
pub enum HandleDragEvent {
    Start(DragHandle, Position),
    Drag(DragHandle, Position),
    End(DragHandle, Position),
}

#[derive(Debug)]
pub struct HandleDragTracker {
    active_handle: Option<DragHandle>,
    dragging: bool,
}

impl HandleDragTracker {
    pub fn new() -> Self {
        Self {
            active_handle: None,
            dragging: false,
        }
    }

    pub fn handle_event(
        &mut self,
        event: &PlatformEvent,
        wm: &WindowManager,
    ) -> Option<HandleDragEvent> {
        match event {
            PlatformEvent::MouseMoved(pos) => {
                if self.dragging {
                    let handle = self.active_handle.clone()?;
                    return Some(HandleDragEvent::Drag(handle, pos.clone()));
                }
            }
            PlatformEvent::MouseDown(pos, MouseButton::Left) => {
                if self.dragging {
                    return None; // already dragging
                }

                // Check if mouse is over a handle
                if let Some(handle) = wm.drag_handle_at_position(pos) {
                    self.dragging = true;
                    self.active_handle = Some(handle.clone());
                    return Some(HandleDragEvent::Start(handle, pos.clone()));
                }
            }
            PlatformEvent::MouseUp(pos, MouseButton::Left) => {
                if self.dragging {
                    self.dragging = false;
                    if let Some(handle) = self.active_handle.take() {
                        return Some(HandleDragEvent::End(handle, pos.clone()));
                    }
                }
            }
            _ => {}
        }
        None
    }
}
