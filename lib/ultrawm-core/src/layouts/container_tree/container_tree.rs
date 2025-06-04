use std::cmp;
use std::collections::HashMap;

use crate::config::ConfigRef;
use crate::layouts::container_tree::container::{
    Container, ContainerChildRef, ContainerRef, ContainerWindow, ContainerWindowRef,
};
use crate::layouts::container_tree::serialize::serialize_tree;
use crate::layouts::container_tree::{
    Direction, TileAction, MOUSE_ADD_TO_PARENT_PREVIEW_RATIO, MOUSE_ADD_TO_PARENT_THRESHOLD,
    MOUSE_SPLIT_PREVIEW_RATIO, MOUSE_SPLIT_THRESHOLD, MOUSE_SWAP_THRESHOLD,
};
use crate::layouts::{Side, WindowLayout};
use crate::platform::{Bounds, PlatformWindowImpl, Position, WindowId};
use crate::tile_result::InsertResult;
use crate::window::WindowRef;

#[derive(Debug)]
pub struct ContainerTree {
    config: ConfigRef,
    bounds: Bounds,
    root: ContainerRef,
    windows: HashMap<WindowId, ContainerWindowRef>,
}

impl ContainerTree {
    pub fn bounds(&self) -> Bounds {
        self.bounds.clone()
    }

    pub fn root(&self) -> ContainerRef {
        self.root.clone()
    }

    /// Formats the container tree structure for debugging purposes
    fn debug_container(&self, container: &ContainerRef, prefix: &str, is_last: bool) -> String {
        let mut result = String::new();

        // Add the current container info
        let connector = if is_last { "└─ " } else { "├─ " };
        result.push_str(&format!(
            "{}{}Container [{}] {} children\n",
            prefix,
            connector,
            match container.direction() {
                Direction::Horizontal => "H",
                Direction::Vertical => "V",
            },
            container.children().len()
        ));

        // Prepare prefix for children
        let child_prefix = format!("{}{}", prefix, if is_last { "   " } else { "│  " });

        // Add children
        let children = container.children();
        for (i, child) in children.iter().enumerate() {
            let is_last_child = i == children.len() - 1;

            match child {
                ContainerChildRef::Container(child_container) => {
                    result.push_str(&self.debug_container(
                        child_container,
                        &child_prefix,
                        is_last_child,
                    ));
                }
                ContainerChildRef::Window(window) => {
                    let connector = if is_last_child { "└─ " } else { "├─ " };
                    result.push_str(&format!(
                        "{}{}Window [{}] \"{}\"\n",
                        child_prefix,
                        connector,
                        window.id(),
                        window.platform_window().title()
                    ));
                }
            }
        }

        result
    }

    fn find_window_at_position(&self, position: &Position) -> Option<ContainerWindowRef> {
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

    fn get_tile_action(&self, window: &WindowRef, position: &Position) -> Option<TileAction> {
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
        let is_same_window = target.platform_window().id() == window.id();

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
            MouseAction::Swap => {
                if is_same_window {
                    return None;
                }
                Some(TileAction::Swap(target))
            }
            MouseAction::Split => {
                if is_same_window {
                    return None;
                }

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
                // If it's at the edge, split the parent container
                // (Only if this is first or last window in parent container)
                //      If were splitting in the same direction, add to the container
                //      If were splitting in the other direction, create a new container
                if split_direction == parent_direction {
                    if is_same_window {
                        return None;
                    }

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
}

impl WindowLayout for ContainerTree {
    fn new(config: ConfigRef, bounds: Bounds, windows: &Vec<WindowRef>) -> Self
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
        let mut windows = windows
            .iter()
            .map(|w| w.clone())
            .collect::<Vec<WindowRef>>();
        windows.sort_by_key(|w| w.platform_window().position().x);
        let mut windows_map = HashMap::new();

        for window in windows {
            let new_window = root.add_window(ContainerWindow::new(window.clone()));
            windows_map.insert(new_window.id(), new_window);
        }

        Self {
            config,
            bounds,
            root,
            windows: windows_map,
        }
    }

    fn serialize(&self) -> serde_yaml::Value {
        serialize_tree(self)
    }

    fn get_preview_bounds(&self, window: &WindowRef, position: &Position) -> Option<Bounds> {
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

    fn insert_window(
        &mut self,
        window: &WindowRef,
        position: &Position,
    ) -> Result<InsertResult, ()> {
        // First, check if the drop position is valid
        let action = self.get_tile_action(window, position).ok_or(())?;

        println!("Action: {:?}", action);

        // Then, check if this window is already in the tree
        let existing_window = self.windows.get(&window.id()).map(|w| w.clone());

        // Perform the action
        match action {
            TileAction::FillRoot => {
                self.root.add_window(ContainerWindow::new(window.clone()));
            }
            TileAction::Swap(target_window) => {
                if let Some(existing_window) = existing_window {
                    let target_child = ContainerChildRef::Window(target_window.clone());
                    let existing_child = ContainerChildRef::Window(existing_window.clone());
                    Container::swap(&target_child, &existing_child);
                } else {
                    self.replace_window(&target_window.window(), window)?;
                    return Ok(InsertResult::Swap(target_window.window()));
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
                    let window = existing_window
                        .unwrap_or_else(|| ContainerWindow::new(window.clone()).clone());
                    parent.insert_window(index, window.clone());
                } else {
                    // Otherwise, split the root container
                    let window =
                        existing_window.unwrap_or_else(|| ContainerWindow::new(window.clone()));
                    self.root.split_self(window.clone(), side.into());
                }
            }
            TileAction::Split(target_window, side) => {
                let parent = target_window.parent();
                let window =
                    existing_window.unwrap_or_else(|| ContainerWindow::new(window.clone()));
                parent.split_window(&target_window, window.clone(), side.into());
            }
        }

        self.root().balance();

        Ok(InsertResult::None)
    }

    fn replace_window(&mut self, old_window: &WindowRef, new_window: &WindowRef) -> Result<(), ()> {
        let window = self.windows.get(&old_window.id()).ok_or(())?;
        let parent = window.parent();
        let new_window = ContainerWindow::new(new_window.clone());
        parent.replace_child(
            &ContainerChildRef::Window(window.clone()),
            ContainerChildRef::Window(new_window.clone()),
        );

        Ok(())
    }

    fn remove_window(&mut self, window: &WindowRef) -> Result<(), ()> {
        let window = self.windows.get(&window.id()).ok_or(())?;
        let parent = window.parent();
        parent.remove_child(&ContainerChildRef::Window(window.clone()));
        Ok(())
    }

    fn debug_layout(&self) -> String {
        let mut result = String::new();
        result.push_str(&format!(
            "ContainerTree Layout ({}x{} at {},{}):\n",
            self.bounds.size.width,
            self.bounds.size.height,
            self.bounds.position.x,
            self.bounds.position.y
        ));

        if self.root.children().is_empty() {
            result.push_str("└─ (empty)\n");
        } else {
            result.push_str(&self.debug_container(&self.root, "", true));
        }

        result
    }
}
