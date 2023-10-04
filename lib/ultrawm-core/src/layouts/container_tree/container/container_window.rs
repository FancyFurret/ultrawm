use crate::layouts::container_tree::container::{ContainerRef, ParentContainerRef, WindowRef};
use crate::platform::{Bounds, PlatformResult, PlatformWindow};
use crate::window::Window;
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

#[derive(Debug)]
pub struct ContainerWindow {
    parent: RefCell<ParentContainerRef>,
    window: RefCell<Window>,
}

impl PartialEq for ContainerWindow {
    fn eq(&self, other: &Self) -> bool {
        self as *const Self == other as *const Self
    }
}

impl ContainerWindow {
    pub fn new(parent: ParentContainerRef, window: Window) -> WindowRef {
        let window = Self {
            parent: RefCell::new(parent),
            window: RefCell::new(window),
        };
        Rc::new(window)
    }

    pub fn bounds(&self) -> Bounds {
        self.window().bounds().clone()
    }

    pub(super) fn set_bounds(&self, bounds: Bounds) {
        self.window_mut().set_bounds(bounds);
    }

    pub fn parent(&self) -> ContainerRef {
        self.parent.borrow().upgrade().unwrap()
    }

    pub(super) fn set_parent(&self, parent: ParentContainerRef) {
        self.parent.replace(parent);
    }

    pub fn window(&self) -> Ref<Window> {
        self.window.borrow()
    }

    fn window_mut(&self) -> RefMut<Window> {
        self.window.borrow_mut()
    }

    pub fn platform_window(&self) -> PlatformWindow {
        self.window().platform_window().clone()
    }

    pub fn dirty(&self) -> bool {
        self.window().dirty()
    }

    pub fn flush(&self) -> PlatformResult<()> {
        self.window_mut().flush()
    }
}
