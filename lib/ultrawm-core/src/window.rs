use crate::platform::{Bounds, PlatformResult, PlatformWindow, PlatformWindowImpl};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct Window {
    bounds: Bounds,
    platform_window: PlatformWindow,
    dirty: AtomicBool,
}

impl Window {
    pub fn new(platform_window: PlatformWindow) -> Self {
        Self {
            bounds: Bounds {
                position: platform_window.position(),
                size: platform_window.size(),
            },
            platform_window,
            dirty: AtomicBool::new(false),
        }
    }

    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    pub fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
        self.dirty.store(true, Ordering::Relaxed);
    }

    pub fn platform_window(&self) -> &PlatformWindow {
        &self.platform_window
    }

    pub fn dirty(&self) -> bool {
        self.dirty.load(Ordering::Relaxed)
    }

    pub fn flush(&mut self) -> PlatformResult<()> {
        self.dirty.store(false, Ordering::Relaxed);
        self.platform_window.set_bounds(&self.bounds)?;

        Ok(())
    }
}
