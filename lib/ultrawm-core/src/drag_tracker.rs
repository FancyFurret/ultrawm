use crate::platform::{
    MouseButton, Platform, PlatformEvent, PlatformImpl, PlatformWindow, Position,
};

pub enum WindowDragEvent {
    Start(PlatformWindow, Position),
    Move(PlatformWindow, Position),
    End(PlatformWindow, Position),
}

#[derive(Debug)]
pub struct DragTracker {
    left_mouse_down: bool,
    dragging: bool,
    dragging_window: Option<PlatformWindow>,
}

impl DragTracker {
    pub fn new() -> Self {
        Self {
            left_mouse_down: false,
            dragging: false,
            dragging_window: None,
        }
    }

    pub fn handle_event(&mut self, event: &PlatformEvent) -> Option<WindowDragEvent> {
        match event {
            PlatformEvent::MouseDown(_, MouseButton::Left) => {
                self.left_mouse_down = true;
            }
            PlatformEvent::MouseUp(_, MouseButton::Left) => {
                self.left_mouse_down = false;

                if self.dragging {
                    self.dragging = false;
                    return Some(WindowDragEvent::End(
                        self.dragging_window.take().unwrap(),
                        Platform::get_mouse_position().ok()?,
                    ));
                }

                self.dragging = false;
            }
            PlatformEvent::MouseMoved(_) => {
                if self.dragging {
                    return Some(WindowDragEvent::Move(
                        self.dragging_window.clone().unwrap(),
                        Platform::get_mouse_position().ok()?,
                    ));
                }
            }
            PlatformEvent::WindowMoved(window) => {
                if self.left_mouse_down {
                    return if !self.dragging {
                        self.dragging = true;
                        self.dragging_window = Some(window.clone());
                        Some(WindowDragEvent::Start(
                            window.clone(),
                            Platform::get_mouse_position().ok()?,
                        ))
                    } else {
                        Some(WindowDragEvent::Move(
                            window.clone(),
                            Platform::get_mouse_position().ok()?,
                        ))
                    };
                }
            }
            _ => {}
        }

        None
    }
}
