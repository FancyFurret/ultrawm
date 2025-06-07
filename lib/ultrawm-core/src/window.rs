use crate::config::ConfigRef;
use crate::platform::{Bounds, PlatformResult, PlatformWindow, PlatformWindowImpl, WindowId};
use std::cell::{Ref, RefCell};
use std::rc::Rc;

pub type WindowRef = Rc<Window>;

#[derive(Debug)]
pub struct Window {
    bounds: RefCell<Bounds>,
    platform_window: RefCell<PlatformWindow>,
    dirty: RefCell<bool>,
    config: ConfigRef,
}

impl Window {
    pub fn new(platform_window: PlatformWindow, config: ConfigRef) -> Self {
        Self {
            bounds: RefCell::new(Bounds {
                position: platform_window.position(),
                size: platform_window.size(),
            }),
            platform_window: RefCell::new(platform_window),
            dirty: RefCell::new(false),
            config,
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

        let mut bounds = self.bounds.borrow().clone();

        // Apply gap (offset from screen edge)
        bounds.position.x += self.config.window_gap as i32 / 2;
        bounds.position.y += self.config.window_gap as i32 / 2;

        bounds.size.width = bounds
            .size
            .width
            .saturating_sub(self.config.window_gap as u32);
        bounds.size.height = bounds
            .size
            .height
            .saturating_sub(self.config.window_gap as u32);

        self.platform_window.borrow().set_bounds(&bounds)?;

        Ok(())
    }
}
