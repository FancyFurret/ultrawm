use crate::config::Config;
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

        let mut bounds = self.bounds.borrow().clone();
        let config = Config::current();

        // Apply gap (offset from screen edge)
        bounds.position.x += config.window_gap as i32 / 2;
        bounds.position.y += config.window_gap as i32 / 2;

        bounds.size.width = bounds.size.width.saturating_sub(config.window_gap);
        bounds.size.height = bounds.size.height.saturating_sub(config.window_gap);

        self.platform_window.borrow().set_bounds(&bounds)?;

        Ok(())
    }

    pub fn window_bounds(&self) -> Bounds {
        let config = Config::current();

        let mut bounds = self.bounds.borrow().clone();
        bounds.position.x += config.window_gap as i32 / 2;
        bounds.position.y += config.window_gap as i32 / 2;
        bounds.size.width -= config.window_gap;
        bounds.size.height -= config.window_gap;
        bounds
    }

    pub fn platform_bounds(&self) -> Bounds {
        let config = Config::current();

        let mut bounds = self.platform_window().size().clone();
        bounds.width += config.window_gap;
        bounds.height += config.window_gap;

        let mut position = self.platform_window().position().clone();
        position.x -= config.window_gap as i32 / 2;
        position.y -= config.window_gap as i32 / 2;

        Bounds {
            position,
            size: bounds,
        }
    }
}
