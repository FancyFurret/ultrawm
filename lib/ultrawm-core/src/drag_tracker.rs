use crate::layouts::ResizeDirection;
use crate::platform::{
    Bounds, MouseButton, Platform, PlatformImpl, PlatformWindowImpl, Position, WMEvent, WindowId,
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

    pub fn handle_event(&mut self, event: &WMEvent, wm: &WindowManager) -> Option<WindowDragEvent> {
        match event {
            WMEvent::WindowTransformStarted(id) => {
                if !self.left_mouse_down || self.current_drag.is_some() {
                    return None;
                }

                let window = wm.get_window(*id).ok()?;
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
            WMEvent::MouseDown(_, MouseButton::Left) => {
                self.left_mouse_down = true;
            }
            WMEvent::MouseUp(pos, MouseButton::Left) => {
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
            WMEvent::MouseMoved(pos) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::Bounds;

    // === WindowDragType Tests ===

    #[test]
    fn test_window_drag_type_equality() {
        assert_eq!(WindowDragType::Move, WindowDragType::Move);
        assert_eq!(
            WindowDragType::Resize(ResizeDirection::Left),
            WindowDragType::Resize(ResizeDirection::Left)
        );
        assert_ne!(
            WindowDragType::Move,
            WindowDragType::Resize(ResizeDirection::Right)
        );
        assert_ne!(
            WindowDragType::Resize(ResizeDirection::Left),
            WindowDragType::Resize(ResizeDirection::Right)
        );
    }

    #[test]
    fn test_window_drag_type_clone() {
        let move_type = WindowDragType::Move;
        let cloned = move_type.clone();
        assert_eq!(move_type, cloned);

        let resize_type = WindowDragType::Resize(ResizeDirection::TopLeft);
        let cloned = resize_type.clone();
        assert_eq!(resize_type, cloned);
    }

    #[test]
    fn test_window_drag_type_debug() {
        let move_type = WindowDragType::Move;
        let debug_str = format!("{:?}", move_type);
        assert!(debug_str.contains("Move"));

        let resize_type = WindowDragType::Resize(ResizeDirection::BottomRight);
        let debug_str = format!("{:?}", resize_type);
        assert!(debug_str.contains("Resize"));
        assert!(debug_str.contains("BottomRight"));
    }

    // === Calculate Resize Direction Tests ===

    #[test]
    fn test_calculate_resize_direction_left() {
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(90, 100, 210, 200); // Left edge moved left

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::Left);
    }

    #[test]
    fn test_calculate_resize_direction_right() {
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(100, 100, 210, 200); // Right edge moved right

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::Right);
    }

    #[test]
    fn test_calculate_resize_direction_top() {
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(100, 90, 200, 210); // Top edge moved up

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::Top);
    }

    #[test]
    fn test_calculate_resize_direction_bottom() {
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(100, 100, 200, 210); // Bottom edge moved down

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::Bottom);
    }

    #[test]
    fn test_calculate_resize_direction_top_left() {
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(90, 90, 210, 210); // Top-left corner moved

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::TopLeft);
    }

    #[test]
    fn test_calculate_resize_direction_top_right() {
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(100, 90, 210, 210); // Top-right corner moved

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::TopRight);
    }

    #[test]
    fn test_calculate_resize_direction_bottom_left() {
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(90, 100, 210, 210); // Bottom-left corner moved

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::BottomLeft);
    }

    #[test]
    fn test_calculate_resize_direction_bottom_right() {
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(100, 100, 210, 210); // Bottom-right corner moved

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::BottomRight);
    }

    #[test]
    fn test_calculate_resize_direction_no_change() {
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(100, 100, 200, 200); // No change

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::Right); // Default fallback
    }

    #[test]
    fn test_calculate_resize_direction_complex_case() {
        // Test a case where multiple edges change in an unexpected way
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(90, 90, 190, 190); // All edges changed

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        // Should default to Right since it doesn't match any specific pattern
        assert_eq!(direction, ResizeDirection::Right);
    }

    #[test]
    fn test_calculate_resize_direction_zero_size() {
        let old = Bounds::new(100, 100, 0, 0);
        let new = Bounds::new(100, 100, 50, 50);

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::BottomRight);
    }

    #[test]
    fn test_calculate_resize_direction_negative_coords() {
        let old = Bounds::new(-100, -100, 200, 200);
        let new = Bounds::new(-110, -100, 210, 200); // Left edge moved left

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::Left);
    }

    #[test]
    fn test_calculate_resize_direction_large_numbers() {
        let old = Bounds::new(1000, 1000, 2000, 2000);
        let new = Bounds::new(1000, 1000, 2000, 2100); // Bottom edge moved

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::Bottom);
    }

    // === WindowDragTracker Constructor Tests ===

    #[test]
    fn test_window_drag_tracker_new() {
        let tracker = WindowDragTracker::new();
        assert_eq!(tracker.left_mouse_down, false);
        assert!(tracker.current_drag.is_none());
    }

    // === Threshold Constant Tests ===

    #[test]
    fn test_drag_type_determination_threshold() {
        assert_eq!(DRAG_TYPE_DETERMINATION_THRESHOLD, 5);
        assert!(DRAG_TYPE_DETERMINATION_THRESHOLD > 0); // Should be positive
    }

    // === Edge Cases for Resize Direction ===

    #[test]
    fn test_calculate_resize_direction_only_width_increase() {
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(100, 100, 300, 200); // Only width increased

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::Right);
    }

    #[test]
    fn test_calculate_resize_direction_only_height_increase() {
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(100, 100, 200, 300); // Only height increased

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::Bottom);
    }

    #[test]
    fn test_calculate_resize_direction_width_decrease() {
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(120, 100, 180, 200); // Width decreased from left

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::Left);
    }

    #[test]
    fn test_calculate_resize_direction_height_decrease() {
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(100, 120, 200, 180); // Height decreased from top

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::Top);
    }

    #[test]
    fn test_calculate_resize_direction_position_only() {
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(150, 150, 200, 200); // Only position changed, size same

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        // This represents a complex case where position changed but size didn't
        // Should fall through to default
        assert_eq!(direction, ResizeDirection::Right);
    }

    #[test]
    fn test_resize_direction_calculation_precision() {
        // Test with very small changes that might cause precision issues
        let old = Bounds::new(100, 100, 200, 200);
        let new = Bounds::new(100, 100, 201, 200); // Tiny width increase

        let direction = WindowDragTracker::calculate_resize_direction(&old, &new);
        assert_eq!(direction, ResizeDirection::Right);
    }

    #[test]
    fn test_bounds_edge_calculations() {
        // Test the edge calculation logic directly
        let bounds = Bounds::new(100, 150, 200, 300);

        // Right edge should be x + width = 100 + 200 = 300
        let right_edge = bounds.position.x + bounds.size.width as i32;
        assert_eq!(right_edge, 300);

        // Bottom edge should be y + height = 150 + 300 = 450
        let bottom_edge = bounds.position.y + bounds.size.height as i32;
        assert_eq!(bottom_edge, 450);
    }
}
