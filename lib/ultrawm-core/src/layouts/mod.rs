use crate::config::ConfigRef;
use crate::platform::{Bounds, Position};
use crate::window::WindowRef;
pub use container_tree::*;
use std::fmt::Debug;

mod container_tree;

pub trait WindowLayout: Debug {
    fn new(config: ConfigRef, bounds: Bounds, windows: &Vec<WindowRef>) -> Self
    where
        Self: Sized;

    fn serialize(&self) -> serde_yaml::Value;

    fn get_tile_bounds(&self, window: &WindowRef, position: &Position) -> Option<Bounds>;

    fn tile_window(&mut self, window: &WindowRef, position: &Position) -> Result<(), ()>;

    fn remove_window(&mut self, window: &WindowRef) -> Result<(), ()>;
}
