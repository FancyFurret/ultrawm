use crate::platform::{MouseButtons, Position, WMEvent};
use crate::resize_handle::ResizeHandle;
use crate::wm::WindowManager;

#[derive(Debug, Clone)]
pub enum HandleDragEvent {
    Start(ResizeHandle, Position, MouseButtons),
    Drag(ResizeHandle, Position, MouseButtons),
    End(ResizeHandle, Position, MouseButtons),
}

#[derive(Debug)]
pub struct HandleDragTracker {
    active_handle: Option<ResizeHandle>,
    dragging: bool,
    current_buttons: MouseButtons,
    drag_buttons: MouseButtons,
}

impl HandleDragTracker {
    pub fn new() -> Self {
        Self {
            active_handle: None,
            dragging: false,
            current_buttons: MouseButtons::new(),
            drag_buttons: MouseButtons::new(),
        }
    }

    pub fn handle_event(&mut self, event: &WMEvent, wm: &WindowManager) -> Option<HandleDragEvent> {
        match event {
            WMEvent::MouseMoved(pos) => {
                if self.dragging {
                    let handle = self.active_handle.clone()?;
                    return Some(HandleDragEvent::Drag(
                        handle,
                        pos.clone(),
                        self.drag_buttons.clone(),
                    ));
                }
            }
            WMEvent::MouseDown(pos, button) => {
                self.current_buttons.update_button(button, true);

                if self.dragging {
                    self.drag_buttons.update_button(button, true);
                    return None;
                }

                // Check if mouse is over a handle
                if let Some(handle) = wm.resize_handle_at_position(pos) {
                    self.dragging = true;
                    self.active_handle = Some(handle.clone());
                    self.drag_buttons = self.current_buttons.clone();
                    return Some(HandleDragEvent::Start(
                        handle,
                        pos.clone(),
                        self.drag_buttons.clone(),
                    ));
                }
            }
            WMEvent::MouseUp(pos, button) => {
                self.current_buttons.update_button(button, false);

                if self.dragging && !self.any_button_pressed() {
                    self.dragging = false;
                    if let Some(handle) = self.active_handle.take() {
                        let final_drag_buttons = self.drag_buttons.clone();
                        self.drag_buttons = MouseButtons::new();
                        return Some(HandleDragEvent::End(
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

    fn any_button_pressed(&self) -> bool {
        self.current_buttons.any()
    }

    pub fn get_pressed_buttons(&self) -> &MouseButtons {
        &self.current_buttons
    }

    pub fn get_drag_buttons(&self) -> &MouseButtons {
        &self.drag_buttons
    }
}
