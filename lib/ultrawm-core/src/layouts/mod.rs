use crate::config::ConfigRef;
use crate::drag_handle::DragHandle;
use crate::platform::{Bounds, Position};
use crate::tile_result::InsertResult;
use crate::window::WindowRef;
pub use container_tree::*;
use std::fmt::Debug;

mod container_tree;

pub trait WindowLayout: Debug {
    fn new(config: ConfigRef, bounds: Bounds, windows: &Vec<WindowRef>) -> Self
    where
        Self: Sized;

    fn serialize(&self) -> serde_yaml::Value;

    fn get_preview_bounds(&self, window: &WindowRef, position: &Position) -> Option<Bounds>;

    fn insert_window(
        &mut self,
        window: &WindowRef,
        position: &Position,
    ) -> Result<InsertResult, ()>;

    fn replace_window(&mut self, old_window: &WindowRef, new_window: &WindowRef) -> Result<(), ()>;

    fn remove_window(&mut self, window: &WindowRef) -> Result<(), ()>;

    fn resize_window(&mut self, window: &WindowRef, bounds: &Bounds, direction: ResizeDirection);

    /// Returns a list of drag handles for this layout (empty by default)
    fn drag_handles(&self) -> Vec<DragHandle> {
        Vec::new()
    }

    fn debug_layout(&self) -> String;
}
