pub use container_tree::*;

use crate::{
    layouts::container_tree::container::{ContainerChildRef, ContainerWindowRef},
    WindowId,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU64;

mod container;
mod container_tree;
pub(crate) mod serialization;

pub type ContainerId = u64;

static CONTAINER_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

// Percentage of half the container size that the mouse must be within
const MOUSE_SWAP_THRESHOLD: f32 = 1.0;
const MOUSE_SPLIT_THRESHOLD: f32 = 0.6;
const MOUSE_ADD_TO_PARENT_THRESHOLD: f32 = 0.2;
const MOUSE_SPLIT_PREVIEW_RATIO: f32 = 0.5;
const MOUSE_ADD_TO_PARENT_PREVIEW_RATIO: f32 = 0.25;

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Horizontal,
    Vertical,
}

impl Direction {
    fn opposite(&self) -> Self {
        match self {
            Direction::Horizontal => Direction::Vertical,
            Direction::Vertical => Direction::Horizontal,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    fn direction(&self) -> Direction {
        match self {
            Side::Left | Side::Right => Direction::Horizontal,
            Side::Top | Side::Bottom => Direction::Vertical,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ResizeDirection {
    Left,
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
}

impl ResizeDirection {
    pub fn has_left(&self) -> bool {
        match self {
            ResizeDirection::Left | ResizeDirection::TopLeft | ResizeDirection::BottomLeft => true,
            _ => false,
        }
    }

    pub fn has_right(&self) -> bool {
        match self {
            ResizeDirection::Right | ResizeDirection::TopRight | ResizeDirection::BottomRight => {
                true
            }
            _ => false,
        }
    }

    pub fn has_top(&self) -> bool {
        match self {
            ResizeDirection::Top | ResizeDirection::TopLeft | ResizeDirection::TopRight => true,
            _ => false,
        }
    }

    pub fn has_bottom(&self) -> bool {
        match self {
            ResizeDirection::Bottom
            | ResizeDirection::BottomLeft
            | ResizeDirection::BottomRight => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
enum TileAction {
    FillRoot,
    Swap(ContainerWindowRef),
    AddToParent(ContainerChildRef, Side),
    Split(ContainerWindowRef, Side),
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ContainerTreePlacementTarget {
    #[serde(flatten)]
    pub target: ContainerTreePlacementTargetType,
    #[serde(default)]
    pub side: Option<Side>,
    #[serde(default)]
    pub ratio: Option<f32>,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContainerTreePlacementTargetType {
    Window { id: WindowId },
    Container { id: ContainerId },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::container_tree::container::{
        Container, ContainerRef, ContainerWindow, InsertOrder,
    };
    use crate::platform::mock::MockPlatformWindow;
    use crate::platform::Bounds;
    use crate::window::Window;
    use std::rc::Rc;

    #[test]
    fn test_direction_opposite() {
        assert_eq!(Direction::Horizontal.opposite(), Direction::Vertical);
        assert_eq!(Direction::Vertical.opposite(), Direction::Horizontal);
    }

    #[test]
    fn test_side_direction() {
        assert_eq!(Side::Left.direction(), Direction::Horizontal);
        assert_eq!(Side::Right.direction(), Direction::Horizontal);
        assert_eq!(Side::Top.direction(), Direction::Vertical);
        assert_eq!(Side::Bottom.direction(), Direction::Vertical);
    }

    // === ResizeDirection Tests ===

    #[test]
    fn test_resize_direction_has_left() {
        assert!(ResizeDirection::Left.has_left());
        assert!(ResizeDirection::TopLeft.has_left());
        assert!(ResizeDirection::BottomLeft.has_left());

        assert!(!ResizeDirection::Right.has_left());
        assert!(!ResizeDirection::TopRight.has_left());
        assert!(!ResizeDirection::BottomRight.has_left());
        assert!(!ResizeDirection::Top.has_left());
        assert!(!ResizeDirection::Bottom.has_left());
    }

    #[test]
    fn test_resize_direction_has_right() {
        assert!(ResizeDirection::Right.has_right());
        assert!(ResizeDirection::TopRight.has_right());
        assert!(ResizeDirection::BottomRight.has_right());

        assert!(!ResizeDirection::Left.has_right());
        assert!(!ResizeDirection::TopLeft.has_right());
        assert!(!ResizeDirection::BottomLeft.has_right());
        assert!(!ResizeDirection::Top.has_right());
        assert!(!ResizeDirection::Bottom.has_right());
    }

    #[test]
    fn test_resize_direction_has_top() {
        assert!(ResizeDirection::Top.has_top());
        assert!(ResizeDirection::TopLeft.has_top());
        assert!(ResizeDirection::TopRight.has_top());

        assert!(!ResizeDirection::Bottom.has_top());
        assert!(!ResizeDirection::BottomLeft.has_top());
        assert!(!ResizeDirection::BottomRight.has_top());
        assert!(!ResizeDirection::Left.has_top());
        assert!(!ResizeDirection::Right.has_top());
    }

    #[test]
    fn test_resize_direction_has_bottom() {
        assert!(ResizeDirection::Bottom.has_bottom());
        assert!(ResizeDirection::BottomLeft.has_bottom());
        assert!(ResizeDirection::BottomRight.has_bottom());

        assert!(!ResizeDirection::Top.has_bottom());
        assert!(!ResizeDirection::TopLeft.has_bottom());
        assert!(!ResizeDirection::TopRight.has_bottom());
        assert!(!ResizeDirection::Left.has_bottom());
        assert!(!ResizeDirection::Right.has_bottom());
    }

    #[test]
    fn test_resize_direction_corner_cases() {
        // Corner directions should have two directional flags
        assert!(ResizeDirection::TopLeft.has_top());
        assert!(ResizeDirection::TopLeft.has_left());
        assert!(!ResizeDirection::TopLeft.has_right());
        assert!(!ResizeDirection::TopLeft.has_bottom());

        assert!(ResizeDirection::TopRight.has_top());
        assert!(ResizeDirection::TopRight.has_right());
        assert!(!ResizeDirection::TopRight.has_left());
        assert!(!ResizeDirection::TopRight.has_bottom());

        assert!(ResizeDirection::BottomLeft.has_bottom());
        assert!(ResizeDirection::BottomLeft.has_left());
        assert!(!ResizeDirection::BottomLeft.has_right());
        assert!(!ResizeDirection::BottomLeft.has_top());

        assert!(ResizeDirection::BottomRight.has_bottom());
        assert!(ResizeDirection::BottomRight.has_right());
        assert!(!ResizeDirection::BottomRight.has_left());
        assert!(!ResizeDirection::BottomRight.has_top());
    }

    // === InsertOrder Tests ===

    #[test]
    fn test_insert_order_from_side() {
        assert_eq!(InsertOrder::from(Side::Left), InsertOrder::Before);
        assert_eq!(InsertOrder::from(Side::Top), InsertOrder::Before);
        assert_eq!(InsertOrder::from(Side::Right), InsertOrder::After);
        assert_eq!(InsertOrder::from(Side::Bottom), InsertOrder::After);
    }

    #[test]
    fn test_insert_order_default() {
        assert_eq!(InsertOrder::default(), InsertOrder::After);
    }

    #[test]
    fn test_insert_order_equality() {
        assert_eq!(InsertOrder::Before, InsertOrder::Before);
        assert_eq!(InsertOrder::After, InsertOrder::After);
        assert_ne!(InsertOrder::Before, InsertOrder::After);
    }

    // === Direction Tests ===

    #[test]
    fn test_direction_equality() {
        assert_eq!(Direction::Horizontal, Direction::Horizontal);
        assert_eq!(Direction::Vertical, Direction::Vertical);
        assert_ne!(Direction::Horizontal, Direction::Vertical);
    }

    #[test]
    fn test_direction_debug() {
        assert!(format!("{:?}", Direction::Horizontal).contains("Horizontal"));
        assert!(format!("{:?}", Direction::Vertical).contains("Vertical"));
    }

    #[test]
    fn test_direction_clone() {
        let original = Direction::Horizontal;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_direction_copy() {
        let original = Direction::Vertical;
        let copied = original;
        assert_eq!(original, copied);
    }

    // === Side Tests ===

    #[test]
    fn test_side_equality() {
        assert_eq!(Side::Left, Side::Left);
        assert_eq!(Side::Right, Side::Right);
        assert_eq!(Side::Top, Side::Top);
        assert_eq!(Side::Bottom, Side::Bottom);

        assert_ne!(Side::Left, Side::Right);
        assert_ne!(Side::Top, Side::Bottom);
        assert_ne!(Side::Left, Side::Top);
    }

    #[test]
    fn test_side_debug() {
        assert!(format!("{:?}", Side::Left).contains("Left"));
        assert!(format!("{:?}", Side::Right).contains("Right"));
        assert!(format!("{:?}", Side::Top).contains("Top"));
        assert!(format!("{:?}", Side::Bottom).contains("Bottom"));
    }

    #[test]
    fn test_side_clone() {
        let original = Side::Left;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_side_copy() {
        let original = Side::Right;
        let copied = original;
        assert_eq!(original, copied);
    }

    // === ResizeDirection Tests ===

    #[test]
    fn test_resize_direction_equality() {
        assert_eq!(ResizeDirection::Left, ResizeDirection::Left);
        assert_eq!(ResizeDirection::TopLeft, ResizeDirection::TopLeft);
        assert_ne!(ResizeDirection::Left, ResizeDirection::Right);
        assert_ne!(ResizeDirection::TopLeft, ResizeDirection::BottomRight);
    }

    #[test]
    fn test_resize_direction_debug() {
        assert!(format!("{:?}", ResizeDirection::Left).contains("Left"));
        assert!(format!("{:?}", ResizeDirection::TopLeft).contains("TopLeft"));
        assert!(format!("{:?}", ResizeDirection::BottomRight).contains("BottomRight"));
    }

    #[test]
    fn test_resize_direction_clone() {
        let original = ResizeDirection::TopLeft;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_resize_direction_copy() {
        let original = ResizeDirection::BottomRight;
        let copied = original;
        assert_eq!(original, copied);
    }

    // === Integration Tests ===

    #[test]
    fn test_side_to_insert_order_integration() {
        // Test that the conversion works correctly for layout logic
        let left_side = Side::Left;
        let right_side = Side::Right;
        let top_side = Side::Top;
        let bottom_side = Side::Bottom;

        // Before sides should convert to Before
        assert_eq!(InsertOrder::from(left_side), InsertOrder::Before);
        assert_eq!(InsertOrder::from(top_side), InsertOrder::Before);

        // After sides should convert to After
        assert_eq!(InsertOrder::from(right_side), InsertOrder::After);
        assert_eq!(InsertOrder::from(bottom_side), InsertOrder::After);
    }

    #[test]
    fn test_direction_side_consistency() {
        // Test that sides map to correct directions
        assert_eq!(Side::Left.direction(), Direction::Horizontal);
        assert_eq!(Side::Right.direction(), Direction::Horizontal);
        assert_eq!(Side::Top.direction(), Direction::Vertical);
        assert_eq!(Side::Bottom.direction(), Direction::Vertical);

        // Test that opposite directions are consistent
        let horizontal = Direction::Horizontal;
        let vertical = Direction::Vertical;
        assert_eq!(horizontal.opposite(), vertical);
        assert_eq!(vertical.opposite(), horizontal);
        assert_eq!(horizontal.opposite().opposite(), horizontal);
    }

    #[test]
    fn test_resize_direction_combinations() {
        // Test all combinations of resize directions
        let directions = [
            ResizeDirection::Left,
            ResizeDirection::TopLeft,
            ResizeDirection::Top,
            ResizeDirection::TopRight,
            ResizeDirection::Right,
            ResizeDirection::BottomRight,
            ResizeDirection::Bottom,
            ResizeDirection::BottomLeft,
        ];

        for direction in &directions {
            // Each direction should have at least one flag set
            let has_any = direction.has_left()
                || direction.has_right()
                || direction.has_top()
                || direction.has_bottom();
            assert!(
                has_any,
                "Direction {:?} should have at least one flag set",
                direction
            );

            // Opposite flags should not be set simultaneously
            assert!(
                !(direction.has_left() && direction.has_right()),
                "Direction {:?} should not have both left and right",
                direction
            );
            assert!(
                !(direction.has_top() && direction.has_bottom()),
                "Direction {:?} should not have both top and bottom",
                direction
            );
        }
    }

    pub fn assert_is_container(child: &ContainerChildRef) -> ContainerRef {
        match child {
            ContainerChildRef::Container(c) => c.clone(),
            _ => panic!("Expected {:?} to be a container", child),
        }
    }

    pub fn assert_is_window(child: &ContainerChildRef) -> ContainerWindowRef {
        match child {
            ContainerChildRef::Window(w) => w.clone(),
            _ => panic!("Expected {:?} to be a window", child),
        }
    }

    pub fn assert_window(child: &ContainerChildRef, window: &ContainerWindowRef) {
        let child_window = assert_is_window(child);
        assert_eq!(child_window, *window);
    }

    pub fn new_bounds() -> Bounds {
        Bounds::new(0, 0, 500, 500)
    }

    pub fn new_container() -> ContainerRef {
        Container::new_root(new_bounds())
    }

    pub fn new_window() -> ContainerWindowRef {
        let bounds = new_bounds();
        let window = Rc::new(Window::new(MockPlatformWindow::new(
            bounds.position,
            bounds.size,
            "Mock Window".to_owned(),
        )));
        ContainerWindow::new(window)
    }

    pub fn new_window_with_bounds(bounds: Bounds) -> ContainerWindowRef {
        let window = Rc::new(Window::new(MockPlatformWindow::new(
            bounds.position,
            bounds.size,
            "Mock Window".to_owned(),
        )));
        ContainerWindow::new(window)
    }
}
