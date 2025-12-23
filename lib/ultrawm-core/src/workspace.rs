use crate::layouts::{LayoutError, LayoutResult, WindowLayout};
use crate::platform::traits::PlatformImpl;
use crate::platform::{Bounds, Platform, PlatformResult, Position, WindowId};
use crate::resize_handle::{ResizeHandle, ResizeMode};
use crate::tile_result::InsertResult;
use crate::window::WindowRef;
use log::warn;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

pub type WorkspaceId = usize;

#[derive(Debug)]
pub struct Workspace {
    id: WorkspaceId,
    name: String,
    layout: Box<dyn WindowLayout>,
    windows: HashMap<WindowId, WindowRef>,
}

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

impl Workspace {
    pub fn new<TLayout: WindowLayout + 'static>(
        bounds: Bounds,
        name: String,
        layout: Option<Box<TLayout>>,
        floating: Option<HashMap<WindowId, WindowRef>>,
    ) -> Self {
        let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let layout = layout.unwrap_or_else(|| Box::new(TLayout::new(bounds)));

        let windows = layout
            .windows()
            .iter()
            .map(|w| (w.id(), w.clone()))
            .collect();

        if let Some(windows) = floating.as_ref() {
            for (_, window) in windows {
                window.set_floating(true);
            }
        }

        Self {
            id,
            name,
            layout,
            windows,
        }
    }

    pub fn id(&self) -> WorkspaceId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn layout(&self) -> &Box<dyn WindowLayout> {
        &self.layout
    }

    pub fn windows(&self) -> &HashMap<WindowId, WindowRef> {
        &self.windows
    }

    pub fn has_window(&self, id: &WindowId) -> bool {
        self.windows.contains_key(id)
    }

    pub fn get_window(&self, id: &WindowId) -> Option<&WindowRef> {
        self.windows.get(id)
    }

    pub fn get_tile_bounds(&self, window: &WindowRef, position: &Position) -> Option<Bounds> {
        self.layout.get_preview_bounds(window, position)
    }

    pub fn remove_window(&mut self, window: &WindowRef) -> LayoutResult<()> {
        let old = self.windows.remove(&window.id());
        if old.is_some() {
            if self.layout.windows().iter().any(|w| w.id() == window.id()) {
                self.layout.remove_window(window)?;
            }
        }
        Ok(())
    }

    pub fn replace_window(
        &mut self,
        old_window: &WindowRef,
        new_window: &WindowRef,
    ) -> LayoutResult<()> {
        self.windows.remove(&old_window.id());
        self.windows.insert(new_window.id(), new_window.clone());
        self.layout.replace_window(old_window, new_window)?;
        Ok(())
    }

    pub fn tile_window(
        &mut self,
        window: &WindowRef,
        position: &Position,
    ) -> LayoutResult<InsertResult> {
        let action = self.layout.insert_window(window, position)?;
        window.set_floating(false);
        self.windows.insert(window.id(), window.clone());
        Ok(action)
    }

    pub fn insert_window_relative(
        &mut self,
        window: &WindowRef,
        target: crate::layouts::PlacementTarget,
    ) -> LayoutResult<crate::tile_result::InsertResult> {
        let action = self.layout.insert_relative(window, target)?;
        window.set_floating(false);
        self.windows.insert(window.id(), window.clone());
        Ok(action)
    }

    pub fn float_window(&mut self, window: &WindowRef) -> LayoutResult<()> {
        if self.windows.contains_key(&window.id()) && window.tiled() {
            self.layout.remove_window(window)?
        };

        window.set_floating(true);
        self.windows.insert(window.id(), window.clone());
        Ok(())
    }

    pub fn resize_window(&mut self, window: &WindowRef, bounds: &Bounds) -> LayoutResult<()> {
        if let Some(managed_window) = self.windows.get_mut(&window.id()) {
            if managed_window.floating() {
                window.set_bounds(bounds.clone());
                Ok(())
            } else {
                self.layout.resize_window(window, bounds)
            }
        } else {
            Err(LayoutError::WindowNotFound(window.id()))
        }
    }

    pub fn flush_windows(&mut self) -> PlatformResult<()> {
        let window_count = self.windows.len() as u32;
        Platform::start_window_bounds_batch(window_count).unwrap();
        for window in self.windows.values_mut() {
            window.flush()?;
        }
        Platform::end_window_bounds_batch().unwrap();
        Ok(())
    }

    pub fn serialize(&self) -> serde_yaml::Value {
        self.layout.serialize()
    }

    pub fn resize_handles(&self) -> Vec<ResizeHandle> {
        self.layout.resize_handles()
    }

    pub fn resize_handle_moved(
        &mut self,
        handle: &ResizeHandle,
        position: &Position,
        mode: &ResizeMode,
    ) -> bool {
        self.layout.resize_handle_moved(handle, position, mode)
    }

    pub fn config_changed(&mut self) -> PlatformResult<()> {
        self.layout.config_changed();
        self.flush_windows()
    }

    pub fn cleanup(&mut self) {
        for window in self.windows.values_mut() {
            if window.floating() {
                window.set_floating(false);
                if window.flush().is_err() {
                    warn!("Could not restore always on top state of window")
                }
            }
        }
    }
}
