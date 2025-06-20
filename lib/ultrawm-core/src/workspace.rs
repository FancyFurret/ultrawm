use crate::drag_handle::DragHandle;
use crate::layouts::{LayoutResult, ResizeDirection, WindowLayout};
use crate::platform::traits::PlatformImpl;
use crate::platform::{Bounds, MouseButtons, Platform, PlatformResult, Position, WindowId};
use crate::tile_result::InsertResult;
use crate::window::WindowRef;
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
        windows: &Vec<WindowRef>,
        name: String,
    ) -> Self {
        Self::new_with_saved_layout::<TLayout>(bounds, windows, name, None)
    }

    pub fn new_with_saved_layout<TLayout: WindowLayout + 'static>(
        bounds: Bounds,
        windows: &Vec<WindowRef>,
        name: String,
        saved_layout: Option<&serde_yaml::Value>,
    ) -> Self {
        let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let layout = Box::new(TLayout::new_from_saved(bounds, windows, saved_layout));
        let windows = layout
            .windows()
            .iter()
            .map(|w| (w.id(), w.clone()))
            .collect();
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

    pub fn has_window(&self, id: &WindowId) -> bool {
        self.windows.contains_key(id)
    }

    pub fn get_tile_bounds(&self, window: &WindowRef, position: &Position) -> Option<Bounds> {
        self.layout.get_preview_bounds(window, position)
    }

    pub fn remove_window(&mut self, window: &WindowRef) -> LayoutResult<()> {
        self.windows.remove(&window.id());
        self.layout.remove_window(window)?;
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
        self.windows.insert(window.id(), window.clone());
        Ok(action)
    }

    pub fn resize_window(
        &mut self,
        window: &WindowRef,
        bounds: &Bounds,
        direction: ResizeDirection,
    ) -> LayoutResult<()> {
        self.layout.resize_window(window, bounds, direction)
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

    pub fn drag_handles(&self) -> Vec<DragHandle> {
        self.layout.drag_handles()
    }

    pub fn drag_handle_moved(
        &mut self,
        handle: &DragHandle,
        position: &Position,
        buttons: &MouseButtons,
    ) -> bool {
        self.layout.drag_handle_moved(handle, position, buttons)
    }
}
