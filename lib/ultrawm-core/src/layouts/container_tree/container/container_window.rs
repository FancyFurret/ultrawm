use crate::layouts::container_tree::container::{
    ContainerRef, ContainerWindowRef, ParentContainerRef,
};
use crate::platform::{Bounds, PlatformResult, PlatformWindow, WindowId};
use crate::window::WindowRef;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub struct ContainerWindow {
    parent: RefCell<ParentContainerRef>,
    window: WindowRef,
}

impl PartialEq for ContainerWindow {
    fn eq(&self, other: &Self) -> bool {
        self as *const Self == other as *const Self
    }
}

impl ContainerWindow {
    pub fn new(parent: ParentContainerRef, window: WindowRef) -> ContainerWindowRef {
        let window = Self {
            parent: RefCell::new(parent),
            window,
        };
        Rc::new(window)
    }

    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn bounds(&self) -> Bounds {
        self.window.bounds().clone()
    }

    pub(super) fn set_bounds(&self, bounds: Bounds) {
        self.window.set_bounds(bounds);
    }

    pub fn parent(&self) -> ContainerRef {
        self.parent.borrow().upgrade().unwrap()
    }

    pub(super) fn set_parent(&self, parent: ParentContainerRef) {
        self.parent.replace(parent);
    }

    pub fn window(&self) -> WindowRef {
        self.window.clone()
    }

    pub fn platform_window(&self) -> PlatformWindow {
        self.window.platform_window().clone()
    }

    pub fn dirty(&self) -> bool {
        self.window.dirty()
    }

    pub fn flush(&self) -> PlatformResult<()> {
        self.window.flush()
    }
}
