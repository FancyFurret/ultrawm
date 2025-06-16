use crate::drag_handle::DragHandle;
use crate::platform::{Bounds, Position};
use crate::tile_result::InsertResult;
use crate::window::WindowRef;
pub use container_tree::*;
use std::fmt::Debug;

pub mod container_tree;

pub trait WindowLayout: Debug {
    fn new(bounds: Bounds, windows: &Vec<WindowRef>) -> Self
    where
        Self: Sized;

    fn new_from_saved(
        bounds: Bounds,
        windows: &Vec<WindowRef>,
        saved_layout: Option<&serde_yaml::Value>,
    ) -> Self
    where
        Self: Sized;

    fn serialize(&self) -> serde_yaml::Value;

    fn get_preview_bounds(&self, window: &WindowRef, position: &Position) -> Option<Bounds>;

    fn windows(&self) -> Vec<WindowRef>;

    fn insert_window(
        &mut self,
        window: &WindowRef,
        position: &Position,
    ) -> Result<InsertResult, ()>;

    fn replace_window(&mut self, old_window: &WindowRef, new_window: &WindowRef) -> Result<(), ()>;

    fn remove_window(&mut self, window: &WindowRef) -> Result<(), ()>;

    fn resize_window(&mut self, window: &WindowRef, bounds: &Bounds, direction: ResizeDirection);

    fn drag_handles(&self) -> Vec<DragHandle> {
        Vec::new()
    }

    fn drag_handle_moved(&mut self, _handle: &DragHandle, _position: &Position) -> bool {
        false
    }

    fn debug_layout(&self) -> String;
}
