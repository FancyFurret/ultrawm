use crate::platform::{Bounds, Position, WindowId};
use crate::resize_handle::{ResizeHandle, ResizeMode};
use crate::tile_result::InsertResult;
use crate::window::WindowRef;
pub use container_tree::*;
use std::fmt::Debug;
use thiserror::Error;

pub mod container_tree;

pub type PlacementTarget = serde_yaml::Value;

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("{0}")]
    Error(String),

    #[error("Window not found: {0}")]
    WindowNotFound(WindowId),

    #[error("Position not valid for insertion: {0:?}")]
    InvalidInsertPosition(Position),

    #[error("Placement target not found or invalid: {0}")]
    PlacementTargetNotFound(String),
}

pub type LayoutResult<T> = Result<T, LayoutError>;

pub trait WindowLayout: Debug {
    fn new(bounds: Bounds) -> Self
    where
        Self: Sized;

    fn layout_description(&self) -> String;

    fn placement_help(&self) -> String;

    fn example_layout(&self) -> serde_yaml::Value;

    fn deserialize(
        bounds: Bounds,
        available_windows: &Vec<WindowRef>,
        saved_layout: &serde_yaml::Value,
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
    ) -> LayoutResult<InsertResult>;

    fn insert_relative(
        &mut self,
        _window: &WindowRef,
        _target: PlacementTarget,
    ) -> LayoutResult<InsertResult>;

    fn replace_window(
        &mut self,
        old_window: &WindowRef,
        new_window: &WindowRef,
    ) -> LayoutResult<()>;

    fn remove_window(&mut self, window: &WindowRef) -> LayoutResult<()>;

    fn resize_window(&mut self, window: &WindowRef, bounds: &Bounds) -> LayoutResult<()>;

    fn resize_handles(&self) -> Vec<ResizeHandle> {
        Vec::new()
    }

    fn resize_handle_moved(
        &mut self,
        _handle: &ResizeHandle,
        _position: &Position,
        _mode: &ResizeMode,
    ) -> bool {
        false
    }

    fn debug_layout(&self) -> String;

    fn config_changed(&mut self) {}
}
