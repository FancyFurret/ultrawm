use std::cmp;

use crate::config::ConfigRef;
use crate::layouts::container_tree::container::{
    Container, ContainerChildRef, ContainerRef, WindowRef, WindowType,
};
use crate::layouts::container_tree::container_tree_iterator::ContainerTreeIterator;
use crate::layouts::container_tree::serialize::serialize_tree;
use crate::layouts::container_tree::{
    TileAction, MOUSE_ADD_TO_PARENT_PREVIEW_RATIO, MOUSE_ADD_TO_PARENT_THRESHOLD,
    MOUSE_SPLIT_PREVIEW_RATIO, MOUSE_SPLIT_THRESHOLD, MOUSE_SWAP_THRESHOLD,
};
use crate::layouts::{Side, WindowLayout};
use crate::platform::{Bounds, PlatformResult, PlatformWindow, PlatformWindowImpl, Position};
use crate::window::Window;

#[derive(Debug)]
pub struct ContainerTree {
    #[allow(dead_code)]
    config: ConfigRef,
    bounds: Bounds,
    root: ContainerRef,
}

impl ContainerTree {
    pub fn bounds(&self) -> Bounds {
        self.bounds.clone()
    }

    pub fn root(&self) -> ContainerRef {
        self.root.clone()
    }

    fn find_window_at_position(&self, position: &Position) -> Option<WindowRef> {
        let mut current_container = self.root.clone();
        while !current_container.children().is_empty() {
            let child = current_container
                .children()
                .iter()
                .find(|c| c.bounds().contains(position))?
                .clone();

            match child {
                ContainerChildRef::Container(container) => {
                    current_container = container.clone();
                }
                ContainerChildRef::Window(window) => {
                    return Some(window.clone());
                }
            }
        }
        return None;
    }

    fn find_window_from_platform(&self, platform_window: &PlatformWindow) -> Option<WindowRef> {
        self.window_iter()
            .find(|w| w.platform_window().id() == platform_window.id())
    }

    fn get_closest_distance_from_side(bounds: &Bounds, position: &Position) -> (i32, Side) {
        // First, find which side of the window the mouse is closest to
        let distance_l = (position.x - bounds.position.x).abs();
        let distance_r = (position.x - (bounds.position.x + bounds.size.width as i32)).abs();
        let distance_t = (position.y - bounds.position.y).abs();
        let distance_b = (position.y - (bounds.position.y + bounds.size.height as i32)).abs();

        let mut closest = distance_l;
        let mut side = Side::Left;

        if distance_r < closest {
            closest = distance_r;
            side = Side::Right;
        }

        if distance_t < closest {
            closest = distance_t;
            side = Side::Top;
        }

        if distance_b < closest {
            closest = distance_b;
            side = Side::Bottom;
        }

        (closest, side)
    }

    fn get_tile_action(&self, window: &PlatformWindow, position: &Position) -> Option<TileAction> {
        let target = self.find_window_at_position(position);
        if target.is_none() {
            return if self.root.children().is_empty() {
                // If there are no windows, then we can only insert into the entire root
                Some(TileAction::FillRoot)
            } else {
                None
            };
        }

        let target = target.unwrap();
        let target_child = ContainerChildRef::Window(target.clone());
        if target.platform_window().id() == window.id() {
            // Can't tile a window onto itself
            return None;
        }

        let window_bounds = target.bounds().clone();

        let (distance, side) = Self::get_closest_distance_from_side(&window_bounds, position);
        let split_direction = side.direction();
        let parent_direction = target.parent().direction();

        enum MouseAction {
            Swap,
            Split,
            SplitParent,
        }

        let window_size = cmp::min(window_bounds.size.width, window_bounds.size.height);
        let half_window_size = window_size / 2;
        let action = if (distance as f32) < half_window_size as f32 * MOUSE_ADD_TO_PARENT_THRESHOLD
        {
            MouseAction::SplitParent
        } else if (distance as f32) < half_window_size as f32 * MOUSE_SPLIT_THRESHOLD {
            MouseAction::Split
        } else if (distance as f32) < half_window_size as f32 * MOUSE_SWAP_THRESHOLD {
            MouseAction::Swap
        } else {
            return None;
        };

        return match action {
            MouseAction::Swap => Some(TileAction::Swap(target)),
            MouseAction::Split => {
                // If were splitting in the same direction, add to the parent container
                // If were splitting in the other direction, create a new container
                if split_direction == parent_direction {
                    // Add to parent
                    Some(TileAction::AddToParent(target_child, side))
                } else {
                    // Split child into new container
                    Some(TileAction::Split(target, side))
                }
            }
            MouseAction::SplitParent => {
                // If its at the edge, split the parent container
                // (Only if this is first or last window in parent container)
                //      If were splitting in the same direction, add to the container
                //      If were splitting in the other direction, create a new container
                if split_direction == parent_direction {
                    // Check if target is the first or last child
                    let parent = target.parent();
                    let index = parent.index_of_child(&target_child)?;

                    // If it's the first child, make sure we are splitting left or up
                    // or if it's the last child, make sure we are splitting right or down
                    let first_child = index == 0;
                    let last_child = index == parent.children().len() - 1;
                    if (first_child && (side == Side::Left || side == Side::Top))
                        || (last_child && (side == Side::Right || side == Side::Bottom))
                    {
                        // Add to the parent's parent, if it exists
                        if let Some(parent) = parent.parent() {
                            Some(TileAction::AddToParent(
                                ContainerChildRef::Container(parent),
                                side,
                            ))
                        } else {
                            // Just add to parent
                            Some(TileAction::AddToParent(target_child, side))
                        }
                    } else {
                        // Just add to parent
                        Some(TileAction::AddToParent(target_child, side))
                    }

                    // Add to parent's parent, if it exists
                } else {
                    // Split child into new container
                    let parent = target.parent();
                    Some(TileAction::AddToParent(
                        ContainerChildRef::Container(parent),
                        side,
                    ))
                }
            }
        };
    }

    fn get_preview_for_side(bounds: &Bounds, side: Side, size_ratio: f32) -> Bounds {
        let mut preview_bounds = bounds.clone();
        match side {
            Side::Left => {
                preview_bounds.size.width = (bounds.size.width as f32 * size_ratio) as u32;
            }
            Side::Right => {
                preview_bounds.size.width = (bounds.size.width as f32 * size_ratio) as u32;
                preview_bounds.position.x += (bounds.size.width as f32 * (1.0 - size_ratio)) as i32;
            }
            Side::Top => {
                preview_bounds.size.height = (bounds.size.height as f32 * size_ratio) as u32;
            }
            Side::Bottom => {
                preview_bounds.size.height = (bounds.size.height as f32 * size_ratio) as u32;
                preview_bounds.position.y +=
                    (bounds.size.height as f32 * (1.0 - size_ratio)) as i32;
            }
        }

        preview_bounds
    }

    fn window_iter(&self) -> Box<dyn Iterator<Item = WindowRef>> {
        Box::new(ContainerTreeIterator::new(self.root.clone()))
    }
}

impl WindowLayout for ContainerTree {
    fn new(config: ConfigRef, bounds: Bounds, mut windows: Vec<Window>) -> Self
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

        let root = Container::new_root(config.clone(), root_bounds);

        // Sort by x position so that they stay in somewhat the same order
        windows.sort_by_key(|w| w.platform_window().position().x);

        for window in windows {
            root.add_window(window.into());
        }

        Self {
            config,
            bounds,
            root,
        }
    }

    fn serialize(&self) -> serde_yaml::Value {
        serialize_tree(self)
    }

    fn get_window_bounds(&self, window: &PlatformWindow) -> Option<Bounds> {
        let window = self.find_window_from_platform(window)?;
        Some(window.bounds())
    }

    fn get_tile_preview_for_position(
        &self,
        window: &PlatformWindow,
        position: &Position,
    ) -> Option<Bounds> {
        return match self.get_tile_action(window, position)? {
            TileAction::FillRoot => Some(self.root().bounds().clone()),
            TileAction::Swap(child) => Some(child.bounds().clone()),
            TileAction::AddToParent(child, side) => Some(Self::get_preview_for_side(
                &child.bounds(),
                side,
                MOUSE_ADD_TO_PARENT_PREVIEW_RATIO,
            )),
            TileAction::Split(window, side) => Some(Self::get_preview_for_side(
                &window.bounds(),
                side,
                MOUSE_SPLIT_PREVIEW_RATIO,
            )),
        };
    }

    fn insert_window_at_position(
        &mut self,
        window: &PlatformWindow,
        position: &Position,
    ) -> Result<(), ()> {
        // First, check if the drop position is valid
        let action = self.get_tile_action(window, position).ok_or(())?;

        // Then, check if this window is already in the tree
        let source_window = self.find_window_from_platform(window);
        let window = if let Some(source_window) = source_window.as_ref() {
            WindowType::Existing(source_window.clone())
        } else {
            WindowType::New(Window::new(window.clone()))
        };

        // Finally, perform the action
        match action {
            TileAction::FillRoot => {
                self.root.add_window(window);
            }
            TileAction::Swap(target_window) => {
                if let Some(source_window) = source_window.as_ref() {
                    Container::swap(
                        &ContainerChildRef::Window(source_window.clone()),
                        &ContainerChildRef::Window(target_window.clone()),
                    );
                } else {
                    // We need an existing window to swap with
                    return Err(());
                }
            }
            TileAction::AddToParent(child, side) => {
                if let Some(parent) = child.parent() {
                    // If there is a parent, insert into the parent
                    let mut index = parent.index_of_child(&child).ok_or(())?;
                    if side == Side::Right || side == Side::Bottom {
                        index += 1;
                    }

                    let parent = child.parent().ok_or(())?;
                    parent.insert_window(index, window);
                } else {
                    // Otherwise, split the root container
                    let split_container = self.root.split_self(window);
                    match side {
                        Side::Left | Side::Top => {
                            let child_a = split_container.children()[0].clone();
                            let child_b = split_container.children()[1].clone();
                            Container::swap(&child_a, &child_b);
                        }
                        _ => {}
                    }
                }
            }
            TileAction::Split(target_window, side) => {
                let parent = target_window.parent();
                let split_container = parent.split_window(&target_window, window);

                // Swap the windows if necessary
                match side {
                    Side::Left | Side::Top => {
                        let child_a = split_container.children()[0].clone();
                        let child_b = split_container.children()[1].clone();
                        Container::swap(&child_a, &child_b);
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn remove_window(&mut self, window: crate::platform::WindowId) -> Result<(), ()> {
        let window = self
            .window_iter()
            .find(|w| w.platform_window().id() == window)
            .ok_or(())?;

        let parent = window.parent();
        parent.remove_child(&ContainerChildRef::Window(window));

        Ok(())
    }

    fn flush(&mut self) -> PlatformResult<()> {
        for window in self.window_iter().filter(|w| w.dirty()) {
            window.flush()?;
        }

        Ok(())
    }
}
