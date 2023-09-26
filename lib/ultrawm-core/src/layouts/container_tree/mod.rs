use crate::config::ConfigRef;
use crate::layouts::container_tree::container::{Container, ContainerChild};
use crate::layouts::WindowLayout;
use crate::platform::{Bounds, PlatformResult, PlatformWindowImpl};
use crate::window::Window;

mod container;

#[derive(Debug)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(Debug)]
pub struct ContainerTree {
    config: ConfigRef,
    bounds: Bounds,
    root: Container,
}

impl ContainerTree {}

impl WindowLayout for ContainerTree {
    fn new(config: ConfigRef, bounds: Bounds, mut windows: Vec<Window>) -> PlatformResult<Self>
    where
        Self: Sized,
    {
        // For now, just add each window to the root container.
        // Later, we should try to keep the windows in similar positions to how they were before
        // as to not mess up the user's layout.

        let root_bounds = Bounds::new(
            bounds.position.x + config.partition_gap as i32,
            bounds.position.y + config.partition_gap as i32,
            bounds.size.width - config.partition_gap * 2,
            bounds.size.height - config.partition_gap * 2,
        );

        let mut root = Container::new(config.clone(), root_bounds, Direction::Horizontal);

        // Sort by x position so that they stay in somewhat the same order
        windows.sort_by_key(|w| w.platform_window().position().x);

        for window in windows {
            root.add_window(window);
        }

        Ok(Self {
            config,
            bounds,
            root,
        })
    }

    fn iter<'a>(&'a self) -> Box<(dyn Iterator<Item = &'a Window> + 'a)> {
        Box::new(ContainerTreeIterator::<'a>::new(&self.root))
    }
}

struct ContainerTreeIterator<'a> {
    stack: Vec<&'a Container>,
    current_windows: Vec<&'a Window>,
}

impl<'a> ContainerTreeIterator<'a> {
    fn new(root: &'a Container) -> Self {
        let mut stack = Vec::new();
        stack.push(root);
        Self {
            stack,
            current_windows: Vec::new(),
        }
    }
}

impl<'a> Iterator for ContainerTreeIterator<'a> {
    type Item = &'a Window;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(container) = self.stack.pop() {
            for child in container.children() {
                match child {
                    ContainerChild::Container(container) => self.stack.push(container),
                    ContainerChild::Window(window) => self.current_windows.push(window),
                }
            }
        }

        if let Some(window) = self.current_windows.pop() {
            return Some(window);
        }

        None
    }
}
