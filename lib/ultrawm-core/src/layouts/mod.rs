use crate::config::ConfigRef;
use crate::platform::{Bounds, PlatformResult, PlatformWindow, Position};
use crate::window::Window;
pub use container_tree::*;
use std::fmt::Debug;

mod container_tree;

pub trait WindowLayout: Debug {
    fn new(config: ConfigRef, bounds: Bounds, windows: Vec<Window>) -> Self
    where
        Self: Sized;

    fn serialize(&self) -> serde_yaml::Value;

    fn get_tile_preview_for_position(
        &self,
        window: &PlatformWindow,
        position: &Position,
    ) -> Option<Bounds>;

    fn insert_window_at_position(
        &mut self,
        window: &PlatformWindow,
        position: &Position,
    ) -> Result<(), ()>;

    fn flush(&mut self) -> PlatformResult<()>;
}
