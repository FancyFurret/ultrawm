use crate::platform::{Bounds, PlatformResult, PlatformWindow, PlatformWindowImpl, WindowId};
use std::cell::{Ref, RefCell};
use std::rc::Rc;

pub type WindowRef = Rc<Window>;

#[derive(Debug)]
pub struct Window {
    bounds: RefCell<Bounds>,
    platform_window: RefCell<PlatformWindow>,
    dirty: RefCell<bool>,
}

impl Window {
    pub fn new(platform_window: PlatformWindow) -> Self {
        Self {
            bounds: RefCell::new(Bounds {
                position: platform_window.position(),
                size: platform_window.size(),
            }),
            platform_window: RefCell::new(platform_window),
            dirty: RefCell::new(false),
        }
    }

    pub fn id(&self) -> WindowId {
        self.platform_window().id()
    }

    pub fn bounds(&self) -> Ref<Bounds> {
        self.bounds.borrow()
    }

    pub fn set_bounds(&self, bounds: Bounds) {
        self.bounds.replace(bounds);
        self.dirty.replace(true);
    }

    pub fn platform_window(&self) -> Ref<PlatformWindow> {
        self.platform_window.borrow()
    }

    pub fn dirty(&self) -> bool {
        self.dirty.borrow().clone()
    }

    pub fn flush(&self) -> PlatformResult<()> {
        self.dirty.replace(false);

        let bounds = self.bounds.borrow().clone();
        self.platform_window.borrow_mut().set_bounds(&bounds)?;

        Ok(())
    }
}
