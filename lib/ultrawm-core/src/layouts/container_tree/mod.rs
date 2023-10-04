pub use container_tree::*;

use crate::layouts::container_tree::container::{ContainerChildRef, WindowRef};

mod container;
mod container_tree;
mod container_tree_iterator;
mod serialize;

// Percentage of half the container size that the mouse must be within
const MOUSE_SWAP_THRESHOLD: f32 = 1.0;
const MOUSE_SPLIT_THRESHOLD: f32 = 0.6;
const MOUSE_ADD_TO_PARENT_THRESHOLD: f32 = 0.2;
const MOUSE_SPLIT_PREVIEW_RATIO: f32 = 0.5;
const MOUSE_ADD_TO_PARENT_PREVIEW_RATIO: f32 = 0.25;

#[derive(Debug, PartialEq, Clone, Copy)]
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

#[derive(Debug, PartialEq, Clone, Copy)]
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

#[derive(Debug)]
enum TileAction {
    FillRoot,
    Swap(WindowRef),
    AddToParent(ContainerChildRef, Side),
    Split(WindowRef, Side),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ConfigRef};
    use crate::layouts::container_tree::container::{Container, ContainerRef};
    use crate::platform::mock::MockPlatformWindow;
    use crate::platform::{Bounds, PlatformWindowImpl};
    use crate::window::Window;
    use std::fmt::Debug;
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

    pub fn assert_is_container(child: &ContainerChildRef) -> ContainerRef {
        match child {
            ContainerChildRef::Container(c) => c.clone(),
            _ => panic!("Expected {:?} to be a container", child),
        }
    }

    pub fn assert_is_window(child: &ContainerChildRef) -> WindowRef {
        match child {
            ContainerChildRef::Window(w) => w.clone(),
            _ => panic!("Expected {:?} to be a window", child),
        }
    }

    pub fn assert_window(child: &ContainerChildRef, window: &WindowRef) {
        let child_window = assert_is_window(child);
        assert_eq!(child_window, *window);
    }

    pub fn new_bounds() -> Bounds {
        Bounds::new(0, 0, 500, 500)
    }

    pub fn new_config() -> ConfigRef {
        Rc::new(Config::default())
    }

    pub fn new_container() -> ContainerRef {
        Container::new_root(new_config(), new_bounds())
    }

    pub fn new_window() -> Window {
        let bounds = new_bounds();
        Window::new(MockPlatformWindow::new(
            bounds.position,
            bounds.size,
            "Mock Window".to_owned(),
        ))
    }

    pub fn new_window_with_bounds(bounds: Bounds) -> Window {
        Window::new(MockPlatformWindow::new(
            bounds.position,
            bounds.size,
            "Mock Window".to_owned(),
        ))
    }
}
