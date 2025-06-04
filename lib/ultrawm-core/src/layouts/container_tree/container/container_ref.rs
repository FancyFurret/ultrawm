use crate::layouts::container_tree::container::container_window::ContainerWindow;
use crate::layouts::container_tree::container::{Container, ParentContainerRef};
use crate::platform::Bounds;
use std::rc::Rc;

pub type ContainerRef = Rc<Container>;
pub type ContainerWindowRef = Rc<ContainerWindow>;

#[derive(Debug, Clone, PartialEq)]
pub enum ContainerChildRef {
    Container(ContainerRef),
    Window(ContainerWindowRef),
}

impl ContainerChildRef {
    pub fn bounds(&self) -> Bounds {
        match self {
            ContainerChildRef::Container(container) => container.bounds(),
            ContainerChildRef::Window(window) => window.bounds(),
        }
    }

    pub(super) fn set_bounds(&self, bounds: Bounds) {
        match self {
            ContainerChildRef::Container(container) => container.set_bounds(bounds),
            ContainerChildRef::Window(window) => window.set_bounds(bounds),
        }
    }

    pub fn parent(&self) -> Option<ContainerRef> {
        match self {
            ContainerChildRef::Container(container) => container.parent(),
            ContainerChildRef::Window(window) => Some(window.parent()),
        }
    }

    pub(super) fn set_parent(&self, parent: ParentContainerRef) {
        match self {
            ContainerChildRef::Container(container) => container.set_parent(parent),
            ContainerChildRef::Window(window) => window.set_parent(parent),
        }
    }
}

impl PartialEq<ContainerRef> for ContainerChildRef {
    fn eq(&self, other: &ContainerRef) -> bool {
        match self {
            ContainerChildRef::Container(container) => container == other,
            ContainerChildRef::Window(_) => false,
        }
    }
}

impl PartialEq<ContainerWindowRef> for ContainerChildRef {
    fn eq(&self, other: &ContainerWindowRef) -> bool {
        match self {
            ContainerChildRef::Container(_) => false,
            ContainerChildRef::Window(window) => window == other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::container_tree::container::tests::{
        new_container_with_bounds, new_container_with_parent,
    };
    use crate::layouts::container_tree::tests::{
        new_bounds, new_container, new_window, new_window_with_bounds,
    };

    #[test]
    fn test_container_bounds() {
        let container_child = ContainerChildRef::Container(new_container_with_bounds(new_bounds()));
        assert_eq!(container_child.bounds(), new_bounds());
    }

    #[test]
    fn test_container_parent() {
        let root = new_container();
        let container_child = ContainerChildRef::Container(new_container_with_parent(root.clone()));
        assert_eq!(&container_child.parent(), &Some(root));
    }

    #[test]
    fn test_window_bounds() {
        let root = new_container();
        let window = root.add_window(new_window_with_bounds(new_bounds()).into());
        let container_child = ContainerChildRef::Window(window);
        assert_eq!(container_child.bounds(), new_bounds());
    }

    #[test]
    fn test_window_parent() {
        let root = new_container();
        let window = root.add_window(new_window().into());
        let container_child = ContainerChildRef::Window(window);
        assert_eq!(&container_child.parent(), &Some(root));
    }
}
