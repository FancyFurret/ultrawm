use crate::layouts::ResizeDirection;
use crate::platform::{
    Bounds, MouseButton, Platform, PlatformEvent, PlatformImpl, PlatformWindowImpl, Position,
    WindowId,
};
use crate::window::WindowRef;
use crate::wm::WindowManager;

#[derive(Debug)]
pub enum WindowDragEvent {
    Start(WindowId, Position, WindowDragType),
    Drag(WindowId, Position, WindowDragType),
    End(WindowId, Position, WindowDragType),
}

#[derive(Debug, Clone, PartialEq)]
pub enum WindowDragType {
    Move,
    Resize(ResizeDirection),
}

const DRAG_TYPE_DETERMINATION_THRESHOLD: i32 = 5;

#[derive(Debug)]
struct DragContext {
    window: WindowRef,
    drag_type: Option<WindowDragType>,
    start_position: Position,
    start_bounds: Bounds,
}

#[derive(Debug)]
pub struct WindowDragTracker {
    left_mouse_down: bool,
    current_drag: Option<DragContext>,
}

impl WindowDragTracker {
    pub fn new() -> Self {
        Self {
            left_mouse_down: false,
            current_drag: None,
        }
    }

    pub fn handle_event(
        &mut self,
        event: &PlatformEvent,
        wm: &WindowManager,
    ) -> Option<WindowDragEvent> {
        match event {
            PlatformEvent::WindowTransformStarted(id) => {
                if !self.left_mouse_down || self.current_drag.is_some() {
                    return None;
                }

                let window = wm.get_window(*id)?;
                self.current_drag = Some(DragContext {
                    window: window.clone(),
                    drag_type: None,
                    start_position: Platform::get_mouse_position().ok()?,
                    start_bounds: Bounds::from_position(
                        window.platform_window().position(),
                        window.platform_window().size(),
                    ),
                });
            }
            PlatformEvent::MouseDown(_, MouseButton::Left) => {
                self.left_mouse_down = true;
            }
            PlatformEvent::MouseUp(pos, MouseButton::Left) => {
                self.left_mouse_down = false;

                if self.current_drag.is_none() {
                    return None;
                }

                let drag = self.current_drag.take().unwrap();
                return Some(WindowDragEvent::End(
                    drag.window.id(),
                    pos.clone(),
                    drag.drag_type?,
                ));
            }
            PlatformEvent::MouseMoved(pos) => {
                if self.current_drag.is_none() {
                    return None;
                }

                let drag = self.current_drag.as_mut().unwrap();

                if drag.drag_type.is_none() {
                    let delta_x = (pos.x - drag.start_position.x).abs();
                    let delta_y = (pos.y - drag.start_position.y).abs();

                    if delta_x > DRAG_TYPE_DETERMINATION_THRESHOLD
                        || delta_y > DRAG_TYPE_DETERMINATION_THRESHOLD
                    {
                        let current_bounds = Bounds::from_position(
                            drag.window.platform_window().position(),
                            drag.window.platform_window().size(),
                        );

                        // If the bounds haven't changed yet, then wait
                        if current_bounds == drag.start_bounds {
                            return None;
                        }

                        // If the size has changed, then we're resizing
                        if current_bounds.size != drag.start_bounds.size {
                            let start_bounds = drag.start_bounds.clone();
                            drag.drag_type = Some(WindowDragType::Resize(
                                Self::calculate_resize_direction(&start_bounds, &current_bounds),
                            ));
                        } else {
                            drag.drag_type = Some(WindowDragType::Move);
                        }
                    }
                }

                if let Some(drag_type) = drag.drag_type.clone() {
                    return Some(WindowDragEvent::Drag(
                        drag.window.id(),
                        pos.clone(),
                        drag_type,
                    ));
                }
            }
            _ => {}
        }

        None
    }

    fn calculate_resize_direction(old: &Bounds, new: &Bounds) -> ResizeDirection {
        let left_changed = new.position.x != old.position.x;
        let right_changed =
            (new.position.x + new.size.width as i32) != (old.position.x + old.size.width as i32);
        let top_changed = new.position.y != old.position.y;
        let bottom_changed =
            (new.position.y + new.size.height as i32) != (old.position.y + old.size.height as i32);

        match (left_changed, right_changed, top_changed, bottom_changed) {
            (true, false, false, false) => ResizeDirection::Left,
            (false, true, false, false) => ResizeDirection::Right,
            (false, false, true, false) => ResizeDirection::Top,
            (false, false, false, true) => ResizeDirection::Bottom,
            (true, false, true, false) => ResizeDirection::TopLeft,
            (false, true, true, false) => ResizeDirection::TopRight,
            (true, false, false, true) => ResizeDirection::BottomLeft,
            (false, true, false, true) => ResizeDirection::BottomRight,
            // Default/fallback
            _ => ResizeDirection::Right,
        }
    }
}
