use crate::layouts::container_tree::container::{
    Container, ContainerRef, ContainerWindowRef, ParentContainerRef,
};
use crate::platform::{Bounds, PlatformResult, PlatformWindow, Position, Size, WindowId};
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

thread_local! {
    static TEMP_CONTAINER: ContainerRef = Container::new_root(
        Bounds {
            position: Position { x: 0, y: 0 },
            size: Size {
                width: 0,
                height: 0,
            },
        },
    );
}

fn get_temp_container_ref() -> ParentContainerRef {
    TEMP_CONTAINER.with(|container| container.self_ref())
}

impl ContainerWindow {
    pub fn new(window: WindowRef) -> ContainerWindowRef {
        let window = Self {
            parent: RefCell::new(get_temp_container_ref()),
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
