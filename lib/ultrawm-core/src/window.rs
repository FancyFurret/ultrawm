use crate::config::Config;
use crate::platform::{Bounds, PlatformResult, PlatformWindow, PlatformWindowImpl, WindowId};
use std::cell::{Ref, RefCell};
use std::rc::Rc;

pub type WindowRef = Rc<Window>;

pub struct Window {
    bounds: RefCell<Bounds>,
    bounds_dirty: RefCell<bool>,
    always_on_top: RefCell<bool>,
    always_on_top_dirty: RefCell<bool>,
    platform_window: RefCell<PlatformWindow>,
    floating: RefCell<bool>,
}

impl std::fmt::Debug for Window {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Window")
            .field("id", &self.id())
            .field("title", &self.title())
            .field("bounds", &*self.bounds.borrow())
            .field("floating", &self.floating())
            .field("always_on_top", &*self.always_on_top.borrow())
            .field("bounds_dirty", &*self.bounds_dirty.borrow())
            .field("always_on_top_dirty", &*self.always_on_top_dirty.borrow())
            .finish()
    }
}

impl Window {
    pub fn new(platform_window: PlatformWindow) -> Self {
        Self {
            bounds: RefCell::new(Bounds {
                position: platform_window.position(),
                size: platform_window.size(),
            }),
            bounds_dirty: RefCell::new(false),
            always_on_top: RefCell::new(false),
            always_on_top_dirty: RefCell::new(false),
            platform_window: RefCell::new(platform_window),
            floating: RefCell::new(false),
        }
    }

    pub fn id(&self) -> WindowId {
        self.platform_window().id()
    }

    pub fn set_floating(&self, floating: bool) {
        self.always_on_top.replace(floating);
        self.always_on_top_dirty.replace(true);
        self.floating.replace(floating);
    }

    pub fn floating(&self) -> bool {
        self.floating.borrow().clone()
    }

    pub fn tiled(&self) -> bool {
        !self.floating()
    }

    pub fn title(&self) -> String {
        self.platform_window.borrow().title()
    }

    pub fn bounds(&self) -> Ref<Bounds> {
        self.bounds.borrow()
    }

    pub fn set_bounds(&self, bounds: Bounds) {
        let old = self.bounds.replace(bounds.clone());
        if old != bounds {
            self.bounds_dirty.replace(true);
        }
    }

    pub fn update_bounds(&self) {
        let bounds = Bounds {
            size: self.platform_window().size(),
            position: self.platform_window().position(),
        };
        self.bounds.replace(bounds);
    }

    /// Set the position of the raw window, without updating it's managed/tiled position.
    /// Useful for previewing a window location before it's finalized.
    pub fn set_preview_bounds(&self, bounds: Bounds) -> PlatformResult<()> {
        self.set_platform_bounds(bounds)?;
        Ok(())
    }

    pub fn platform_window(&self) -> Ref<PlatformWindow> {
        self.platform_window.borrow()
    }

    pub fn dirty(&self) -> bool {
        self.bounds_dirty.borrow().clone() || self.always_on_top_dirty.borrow().clone()
    }

    pub fn flush(&self) -> PlatformResult<()> {
        if self.bounds_dirty.borrow().clone() {
            self.bounds_dirty.replace(false);

            self.set_platform_bounds(self.window_bounds())?;
        }

        if self.always_on_top_dirty.borrow().clone() {
            let on_top = self.always_on_top.borrow().clone();
            self.always_on_top_dirty.replace(false);
            self.platform_window.borrow().set_always_on_top(on_top)?;
        }

        Ok(())
    }

    /// The bounds of the window, with tiling gaps applied
    pub fn window_bounds(&self) -> Bounds {
        let config = Config::current();
        let mut bounds = self.bounds.borrow().clone();

        if !self.floating() {
            bounds.position.x += config.window_gap as i32 / 2;
            bounds.position.y += config.window_gap as i32 / 2;
            bounds.size.width = bounds.size.width.saturating_sub(config.window_gap).max(100);
            bounds.size.height = bounds
                .size
                .height
                .saturating_sub(config.window_gap)
                .max(100);
        }

        bounds
    }

    pub fn platform_bounds(&self) -> Bounds {
        Bounds {
            position: self.platform_window().position().clone(),
            size: self.platform_window().size().clone(),
        }
    }

    pub fn focus(&self) -> PlatformResult<()> {
        self.platform_window.borrow().focus()
    }

    fn set_platform_bounds(&self, bounds: Bounds) -> PlatformResult<()> {
        self.platform_window.borrow().set_bounds(&bounds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{mock::MockPlatformWindow, Position, Size};

    fn new_tracking_window() -> (Window, MockPlatformWindow) {
        let platform_window = MockPlatformWindow::new(
            Position { x: 0, y: 0 },
            Size {
                width: 100,
                height: 100,
            },
            "Test Window".to_string(),
        );
        let window = Window::new(platform_window.clone());
        (window, platform_window)
    }

    #[test]
    fn test_set_bounds_marks_dirty_but_no_platform_call() {
        let (window, platform_window) = new_tracking_window();

        // Initially not dirty
        assert!(!window.dirty());
        assert_eq!(platform_window.get_set_bounds_calls().len(), 0);

        // Setting bounds should mark dirty but not call platform immediately
        window.set_bounds(Bounds {
            position: Position { x: 10, y: 20 },
            size: Size {
                width: 200,
                height: 300,
            },
        });
        assert!(window.dirty());
        assert_eq!(platform_window.get_set_bounds_calls().len(), 0);

        // Bounds should be updated internally
        assert_eq!(window.bounds().position.x, 10);
        assert_eq!(window.bounds().position.y, 20);
        assert_eq!(window.bounds().size.width, 200);
        assert_eq!(window.bounds().size.height, 300);
    }

    #[test]
    fn test_flush_calls_platform_when_dirty() {
        let (window, platform_window) = new_tracking_window();

        // Set bounds to make window dirty
        window.set_bounds(Bounds {
            position: Position { x: 10, y: 20 },
            size: Size {
                width: 200,
                height: 300,
            },
        });
        assert!(window.dirty());

        // Flush should call platform and clear dirty flag
        window.flush().unwrap();
        assert!(!window.dirty());
        assert_eq!(platform_window.get_set_bounds_calls().len(), 1);

        // Check that the platform was called with the correct bounds (adjusted for gaps)
        let calls = platform_window.get_set_bounds_calls();
        let config = Config::current();
        let expected_bounds = Bounds {
            position: Position {
                x: 10 + config.window_gap as i32 / 2,
                y: 20 + config.window_gap as i32 / 2,
            },
            size: Size {
                width: 200 - config.window_gap,
                height: 300 - config.window_gap,
            },
        };
        assert_eq!(calls[0], expected_bounds);
    }

    #[test]
    fn test_flush_no_call_when_not_dirty() {
        let (window, platform_window) = new_tracking_window();

        // Window starts not dirty
        assert!(!window.dirty());

        // Flush should not call platform when not dirty
        window.flush().unwrap();
        assert_eq!(platform_window.get_set_bounds_calls().len(), 0);
    }

    #[test]
    fn test_multiple_set_bounds_single_flush() {
        let (window, platform_window) = new_tracking_window();

        // Set bounds multiple times - this simulates multiple layout calculations
        window.set_bounds(Bounds {
            position: Position { x: 10, y: 20 },
            size: Size {
                width: 200,
                height: 300,
            },
        });
        window.set_bounds(Bounds {
            position: Position { x: 15, y: 25 },
            size: Size {
                width: 250,
                height: 350,
            },
        });
        window.set_bounds(Bounds {
            position: Position { x: 20, y: 30 },
            size: Size {
                width: 300,
                height: 400,
            },
        });

        assert!(window.dirty());
        assert_eq!(platform_window.get_set_bounds_calls().len(), 0);

        // Only one platform call should happen on flush, with the final bounds
        window.flush().unwrap();
        assert!(!window.dirty());
        assert_eq!(platform_window.get_set_bounds_calls().len(), 1);

        // Should use the last set bounds
        let calls = platform_window.get_set_bounds_calls();
        let config = Config::current();
        let expected_bounds = Bounds {
            position: Position {
                x: 20 + config.window_gap as i32 / 2,
                y: 30 + config.window_gap as i32 / 2,
            },
            size: Size {
                width: 300 - config.window_gap,
                height: 400 - config.window_gap,
            },
        };
        assert_eq!(calls[0], expected_bounds);
    }

    #[test]
    fn test_duplicate_flush_calls_no_extra_platform_calls() {
        let (window, platform_window) = new_tracking_window();

        // Set bounds and flush
        window.set_bounds(Bounds {
            position: Position { x: 10, y: 20 },
            size: Size {
                width: 200,
                height: 300,
            },
        });
        window.flush().unwrap();
        assert_eq!(platform_window.get_set_bounds_calls().len(), 1);

        // Additional flush calls should not trigger more platform calls
        window.flush().unwrap();
        window.flush().unwrap();
        assert_eq!(platform_window.get_set_bounds_calls().len(), 1);
    }

    #[test]
    fn test_set_same_bounds_still_marks_dirty() {
        let (window, platform_window) = new_tracking_window();

        let initial_bounds = window.bounds().clone();

        // Setting the same bounds should still mark dirty
        // (this is current behavior - could be optimized in the future)
        window.set_bounds(initial_bounds);
        assert!(window.dirty());

        window.flush().unwrap();
        assert_eq!(platform_window.get_set_bounds_calls().len(), 1);
    }

    #[test]
    fn test_gap_calculation_in_flush() {
        let (window, platform_window) = new_tracking_window();

        // Set specific bounds
        window.set_bounds(Bounds {
            position: Position { x: 100, y: 200 },
            size: Size {
                width: 400,
                height: 300,
            },
        });

        window.flush().unwrap();

        let calls = platform_window.get_set_bounds_calls();
        assert_eq!(calls.len(), 1);

        let config = Config::current();
        let expected_bounds = Bounds {
            position: Position {
                x: 100 + config.window_gap as i32 / 2,
                y: 200 + config.window_gap as i32 / 2,
            },
            size: Size {
                width: 400 - config.window_gap,
                height: 300 - config.window_gap,
            },
        };
        assert_eq!(calls[0], expected_bounds);
    }

    #[test]
    fn test_simulated_batched_update() {
        // Simulate what happens during a layout update with multiple windows
        let (window1, platform1) = new_tracking_window();
        let (window2, platform2) = new_tracking_window();
        let (window3, platform3) = new_tracking_window();

        // Phase 1: Layout calculation - multiple set_bounds calls, no platform calls yet
        // This simulates the container tree calculating new bounds for all windows
        window1.set_bounds(Bounds {
            position: Position { x: 0, y: 0 },
            size: Size {
                width: 300,
                height: 400,
            },
        });
        window2.set_bounds(Bounds {
            position: Position { x: 300, y: 0 },
            size: Size {
                width: 300,
                height: 400,
            },
        });
        window3.set_bounds(Bounds {
            position: Position { x: 600, y: 0 },
            size: Size {
                width: 300,
                height: 400,
            },
        });

        // Verify no platform calls during layout calculation
        assert_eq!(platform1.get_set_bounds_calls().len(), 0);
        assert_eq!(platform2.get_set_bounds_calls().len(), 0);
        assert_eq!(platform3.get_set_bounds_calls().len(), 0);

        // All windows should be marked dirty but no platform calls happened yet
        assert!(window1.dirty());
        assert!(window2.dirty());
        assert!(window3.dirty());

        // Phase 2: Batch flush - this is where platform calls happen
        // In the real system, this happens in workspace.flush_windows()
        window1.flush().unwrap();
        window2.flush().unwrap();
        window3.flush().unwrap();

        // Each window should have exactly one platform call
        assert_eq!(platform1.get_set_bounds_calls().len(), 1);
        assert_eq!(platform2.get_set_bounds_calls().len(), 1);
        assert_eq!(platform3.get_set_bounds_calls().len(), 1);

        // All windows should now be clean
        assert!(!window1.dirty());
        assert!(!window2.dirty());
        assert!(!window3.dirty());
    }

    #[test]
    fn test_dirty_state_management() {
        let (window, _) = new_tracking_window();

        // Start clean
        assert!(!window.dirty());

        // Set bounds makes dirty
        window.set_bounds(Bounds {
            position: Position { x: 10, y: 20 },
            size: Size {
                width: 100,
                height: 200,
            },
        });
        assert!(window.dirty());

        // Flush clears dirty
        window.flush().unwrap();
        assert!(!window.dirty());

        // Another set_bounds makes dirty again
        window.set_bounds(Bounds {
            position: Position { x: 20, y: 30 },
            size: Size {
                width: 150,
                height: 250,
            },
        });
        assert!(window.dirty());

        // Another flush clears dirty again
        window.flush().unwrap();
        assert!(!window.dirty());
    }

    #[test]
    fn test_bounds_optimization_opportunity() {
        let (window, platform_window) = new_tracking_window();

        // This test documents a potential optimization:
        // If we set the same bounds twice, we still mark as dirty
        // Future optimization could avoid this
        let bounds = Bounds {
            position: Position { x: 100, y: 200 },
            size: Size {
                width: 300,
                height: 400,
            },
        };
        window.set_bounds(bounds.clone());
        window.flush().unwrap();
        assert!(!window.dirty());
        assert_eq!(platform_window.get_set_bounds_calls().len(), 1);

        // Setting same bounds again still marks dirty (current behavior)
        window.set_bounds(bounds);
        assert!(window.dirty()); // Could be optimized to stay clean

        // But flush still works correctly
        window.flush().unwrap();
        assert!(!window.dirty());
        assert_eq!(platform_window.get_set_bounds_calls().len(), 2); // Second call made
    }
}
