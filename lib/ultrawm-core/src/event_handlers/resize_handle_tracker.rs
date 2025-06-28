use crate::platform::{input_state::InputState, MouseButtons, Position, WMEvent};
use crate::resize_handle::ResizeHandle;
use crate::wm::WindowManager;

#[derive(Debug, Clone)]
pub enum ResizeHandleEvent {
    Start(ResizeHandle, Position, MouseButtons),
    Drag(ResizeHandle, Position, MouseButtons),
    End(ResizeHandle, Position, MouseButtons),
}

#[derive(Debug)]
pub struct ResizeHandleTracker {
    active_handle: Option<ResizeHandle>,
    dragging: bool,
    drag_buttons: MouseButtons,
}

impl ResizeHandleTracker {
    pub fn new() -> Self {
        Self {
            active_handle: None,
            dragging: false,
            drag_buttons: MouseButtons::new(),
        }
    }

    pub fn active(&self) -> bool {
        self.dragging
    }

    pub fn handle_event(
        &mut self,
        event: &WMEvent,
        wm: &WindowManager,
    ) -> Option<ResizeHandleEvent> {
        match event {
            WMEvent::MouseMoved(pos) => {
                if self.dragging {
                    let handle = self.active_handle.clone()?;
                    return Some(ResizeHandleEvent::Drag(
                        handle,
                        pos.clone(),
                        self.drag_buttons.clone(),
                    ));
                }
            }
            WMEvent::MouseDown(pos, _) => {
                if let Some(handle) = wm.resize_handle_at_position(pos) {
                    self.dragging = true;
                    self.active_handle = Some(handle.clone());
                    return Some(ResizeHandleEvent::Start(
                        handle,
                        pos.clone(),
                        self.drag_buttons.clone(),
                    ));
                }
            }
            WMEvent::MouseUp(pos, _) => {
                // If no buttons are pressed anymore, end the drag
                if !InputState::pressed_mouse_buttons().any() {
                    self.dragging = false;
                    if let Some(handle) = self.active_handle.take() {
                        let final_drag_buttons = self.drag_buttons.clone();
                        return Some(ResizeHandleEvent::End(
                            handle,
                            pos.clone(),
                            final_drag_buttons,
                        ));
                    }
                }
            }
            _ => {}
        }
        None
    }

    pub fn get_drag_buttons(&self) -> &MouseButtons {
        &self.drag_buttons
    }
}
