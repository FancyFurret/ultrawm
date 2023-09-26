use crate::config::ConfigRef;
use crate::platform::{Bounds, PlatformResult};
use crate::window::Window;
pub use container_tree::*;
use std::fmt::Debug;

mod container_tree;

pub trait WindowLayout: Debug {
    fn new(config: ConfigRef, bounds: Bounds, windows: Vec<Window>) -> PlatformResult<Self>
    where
        Self: Sized;

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Window> + 'a>;
}
