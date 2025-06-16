use std::cmp;
use std::collections::HashMap;

use crate::config::Config;
use crate::drag_handle::{DragHandle, HandleOrientation};
use crate::layouts::container_tree::container::{
    Container, ContainerChildRef, ContainerRef, ContainerWindow, ContainerWindowRef,
    ResizeDistribution,
};
use crate::layouts::container_tree::serialization::{
    deserialize_container, serialize_container, SerializedContainerTree,
};
use crate::layouts::container_tree::{
    Direction, TileAction, MOUSE_ADD_TO_PARENT_PREVIEW_RATIO, MOUSE_ADD_TO_PARENT_THRESHOLD,
    MOUSE_SPLIT_PREVIEW_RATIO, MOUSE_SPLIT_THRESHOLD, MOUSE_SWAP_THRESHOLD,
};
use crate::layouts::{ContainerId, ResizeDirection, Side, WindowLayout};
use crate::platform::{Bounds, PlatformWindowImpl, Position, WindowId};
use crate::tile_result::InsertResult;
use crate::window::WindowRef;

#[derive(Debug)]
pub struct ContainerTree {
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

    fn serialize(&self) -> serde_yaml::Value {
        let serialized = SerializedContainerTree {
            root: serialize_container(&self.root()),
            bounds: self.bounds(),
        };

        serde_yaml::to_value(serialized).unwrap()
    }

    fn deserialize(
        bounds: Bounds,
        windows: &Vec<WindowRef>,
        saved_layout: &serde_yaml::Value,
    ) -> Option<Self> {
        // Try to deserialize the saved layout
        let serialized: SerializedContainerTree =
            serde_yaml::from_value(saved_layout.clone()).ok()?;

        // Create a map of available windows by ID
        let available_windows: HashMap<WindowId, WindowRef> =
            windows.iter().map(|w| (w.id(), w.clone())).collect();

        let mut windows_map = HashMap::new();
        let root = deserialize_container(
            &serialized.root,
            Self::get_root_bounds(&bounds),
            &available_windows,
            &mut windows_map,
            None,
        )?;

        println!(
            "Successfully reconstructed layout with {} windows placed",
            windows_map.len()
        );

        root.recalculate();

        Some(Self {
            bounds,
            root,
            windows: windows_map,
        })
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
        None
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

        match action {
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
        }
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

    /// Finds the container that owns the given drag handle.
    /// Returns the container and the index of the child that the handle is after.
    fn find_container_for_handle(&self, id: ContainerId) -> Option<ContainerRef> {
        let mut result = None;
        self.find_container_for_handle_recursive(&self.root, id, &mut result);
        result
    }

    fn find_container_for_handle_recursive(
        &self,
        container: &ContainerRef,
        id: ContainerId,
        out: &mut Option<ContainerRef>,
    ) {
        if container.id() == id {
            *out = Some(container.clone());
            return;
        }

        // Recurse into children containers
        for child in container.children().iter() {
            if let ContainerChildRef::Container(c) = child {
                self.find_container_for_handle_recursive(c, id, out);
            }
        }
    }

    fn collect_handles_recursive(&self, container: &ContainerRef, out: &mut Vec<DragHandle>) {
        let children = container.children();
        if children.len() <= 1 {
            // No split boundaries with single child
        } else {
            match container.direction() {
                Direction::Horizontal => {
                    // Vertical handles between horizontally arranged children
                    for idx in 0..children.len() - 1 {
                        let right_child = &children[idx + 1];
                        let boundary_x = right_child.bounds().position.x; // leading edge of right child
                        let center = Position {
                            x: boundary_x,
                            y: container.bounds().center().y,
                        };
                        let handle = DragHandle::new(
                            center,
                            container.bounds().size.height,
                            HandleOrientation::Vertical,
                            container.bounds().position.x,
                            container.bounds().position.x + container.bounds().size.width as i32,
                            container.id(),
                            idx + 1,
                        );
                        out.push(handle);
                    }
                }
                Direction::Vertical => {
                    // Horizontal handles between vertically stacked children
                    for idx in 0..children.len() - 1 {
                        let bottom_child = &children[idx + 1];
                        let boundary_y = bottom_child.bounds().position.y; // top edge of bottom child
                        let center = Position {
                            x: container.bounds().center().x,
                            y: boundary_y,
                        };
                        let handle = DragHandle::new(
                            center,
                            container.bounds().size.width,
                            HandleOrientation::Horizontal,
                            container.bounds().position.y,
                            container.bounds().position.y + container.bounds().size.height as i32,
                            container.id(),
                            idx + 1,
                        );
                        out.push(handle);
                    }
                }
            }
        }

        // Recurse into children containers
        for child in children.iter() {
            if let ContainerChildRef::Container(c) = child {
                self.collect_handles_recursive(c, out);
            }
        }
    }

    fn get_root_bounds(bounds: &Bounds) -> Bounds {
        let config = Config::current();

        // Apply partition gap and invert the window gap so that the outer gap is 0
        Bounds::new(
            bounds.position.x + config.partition_gap as i32 - config.window_gap as i32 / 2,
            bounds.position.y + config.partition_gap as i32 - config.window_gap as i32 / 2,
            bounds.size.width - config.partition_gap * 2 + config.window_gap,
            bounds.size.height - config.partition_gap * 2 + config.window_gap,
        )
    }
}

impl WindowLayout for ContainerTree {
    fn new(bounds: Bounds, windows: &Vec<WindowRef>) -> Self
    where
        Self: Sized,
    {
        Self::new_from_saved(bounds, windows, None)
    }

    fn new_from_saved(
        bounds: Bounds,
        windows: &Vec<WindowRef>,
        saved_layout: Option<&serde_yaml::Value>,
    ) -> Self
    where
        Self: Sized,
    {
        if let Some(saved_layout) = saved_layout {
            if let Some(tree) = Self::deserialize(bounds.clone(), windows, saved_layout) {
                return tree;
            }

            println!("Failed to deserialize saved layout, starting from scratch");
        }

        let root_bounds = Self::get_root_bounds(&bounds);
        let root = Container::new_root(root_bounds);
        root.equalize_ratios();
        root.recalculate();

        Self {
            bounds,
            root,
            windows: HashMap::new()
        }
    }

    fn serialize(&self) -> serde_yaml::Value {
        self.serialize()
    }

    fn get_preview_bounds(&self, window: &WindowRef, position: &Position) -> Option<Bounds> {
        match self.get_tile_action(window, position)? {
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
        }
    }

    fn windows(&self) -> Vec<WindowRef> {
        self.windows.values().map(|w| w.window()).collect()
    }

    fn insert_window(
        &mut self,
        window: &WindowRef,
        position: &Position,
    ) -> Result<InsertResult, ()> {
        // First, check if the drop position is valid
        let action = self.get_tile_action(window, position).ok_or(())?;

        // Then, check if this window is already in the tree
        let existing_window = self.windows.get(&window.id()).map(|w| w.clone());
        let is_new_window = existing_window.is_none();

        // Perform the action
        match action {
            TileAction::FillRoot => {
                let container_window = ContainerWindow::new(window.clone());
                self.root.add_window(container_window.clone());
                // Update windows map if this is a new window
                if is_new_window {
                    self.windows.insert(window.id(), container_window);
                }
            }
            TileAction::Swap(target_window) => {
                if let Some(existing_window) = existing_window {
                    let target_child = ContainerChildRef::Window(target_window.clone());
                    let existing_child = ContainerChildRef::Window(existing_window.clone());
                    if is_new_window {
                        self.windows.insert(window.id(), existing_window.clone());
                        self.windows.remove(&existing_window.id());
                    }
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
                    let container_window =
                        existing_window.unwrap_or_else(|| ContainerWindow::new(window.clone()));
                    parent.insert_window(index, container_window.clone());
                    // Update windows map if this is a new window
                    if is_new_window {
                        self.windows.insert(window.id(), container_window);
                    }
                } else {
                    // Otherwise, split the root container
                    let container_window =
                        existing_window.unwrap_or_else(|| ContainerWindow::new(window.clone()));
                    self.root.split_self(container_window.clone(), side.into());
                    // Update windows map if this is a new window
                    if is_new_window {
                        self.windows.insert(window.id(), container_window);
                    }
                }
            }
            TileAction::Split(target_window, side) => {
                let parent = target_window.parent();
                let container_window =
                    existing_window.unwrap_or_else(|| ContainerWindow::new(window.clone()));
                parent.split_window(&target_window, container_window.clone(), side.into());
                // Update windows map if this is a new window
                if is_new_window {
                    self.windows.insert(window.id(), container_window);
                }
            }
        }

        self.root().recalculate();

        Ok(InsertResult::None)
    }

    fn replace_window(&mut self, old_window: &WindowRef, new_window: &WindowRef) -> Result<(), ()> {
        let old_window_id = old_window.id();
        let old_container_window = self.windows.get(&old_window_id).ok_or(())?.clone();
        let parent = old_container_window.parent();
        let new_container_window = ContainerWindow::new(new_window.clone());

        parent.replace_child(
            &ContainerChildRef::Window(old_container_window),
            ContainerChildRef::Window(new_container_window.clone()),
        );

        // Update windows map: remove old window and add new window
        self.windows.remove(&old_window_id);
        self.windows.insert(new_window.id(), new_container_window);

        Ok(())
    }

    fn remove_window(&mut self, window: &WindowRef) -> Result<(), ()> {
        let window_id = window.id();
        let container_window = self.windows.get(&window_id).ok_or(())?.clone();
        let parent = container_window.parent();
        parent.remove_child(&ContainerChildRef::Window(container_window));

        // Remove from windows map
        self.windows.remove(&window_id);
        self.root.recalculate();

        Ok(())
    }

    fn resize_window(&mut self, window: &WindowRef, bounds: &Bounds, direction: ResizeDirection) {
        let container_window = if let Some(w) = self.windows.get(&window.id()) {
            w.clone()
        } else {
            return; // Not managed by this layout
        };

        let mut parent = container_window.parent();

        let mut needs_horizontal = direction.has_left() || direction.has_right();
        let mut needs_vertical = direction.has_top() || direction.has_bottom();

        let bounds = bounds.clone();

        let mut child_ref = ContainerChildRef::Window(container_window.clone());
        while let Some(p) = Some(parent.clone()) {
            let child_index = match p.index_of_child(&child_ref) {
                Some(idx) => idx,
                None => break,
            };
            let is_first = child_index == 0;
            let is_last = child_index == p.children().len() - 1;

            if needs_horizontal && p.direction() == Direction::Horizontal {
                if (direction.has_left() && is_first) || (direction.has_right() && is_last) {
                } else {
                    p.resize_child(&child_ref, &bounds, direction, ResizeDistribution::Spread);
                    needs_horizontal = false;
                }
            } else if needs_vertical && p.direction() == Direction::Vertical {
                if (direction.has_top() && is_first) || (direction.has_bottom() && is_last) {
                } else {
                    p.resize_child(&child_ref, &bounds, direction, ResizeDistribution::Spread);
                    needs_vertical = false;
                }
            }

            if !needs_horizontal && !needs_vertical {
                break;
            }

            child_ref = ContainerChildRef::Container(p.clone());
            if let Some(grandparent) = p.parent() {
                parent = grandparent.clone();
            } else {
                break;
            }
        }

        self.root.recalculate();
    }

    fn drag_handles(&self) -> Vec<DragHandle> {
        let mut handles = Vec::new();
        self.collect_handles_recursive(&self.root, &mut handles);
        handles
    }

    fn drag_handle_moved(&mut self, handle: &DragHandle, position: &Position) -> bool {
        // Find the container that owns this handle
        let container = match self.find_container_for_handle(handle.id) {
            Some(result) => result,
            None => return false,
        };

        // Determine the new position based on the handle orientation
        let new_position = match handle.orientation {
            HandleOrientation::Vertical => position.x,
            HandleOrientation::Horizontal => position.y,
        };

        // Resize the split at the handle index
        let success = container.resize_between(handle.index, new_position);

        if success {
            self.root.recalculate();
        }

        success
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::mock::MockPlatformWindow;
    use crate::platform::{Bounds, Position, Size};
    use crate::window::Window;
    use std::rc::Rc;

    fn create_mock_window(id: u64) -> WindowRef {
        let mut platform_window = MockPlatformWindow::new(
            Position { x: 0, y: 0 },
            Size {
                width: 800,
                height: 600,
            },
            format!("Test Window {}", id),
        );
        platform_window.id = id;
        Rc::new(Window::new(platform_window))
    }

    fn create_test_bounds() -> Bounds {
        Bounds::new(0, 0, 1920, 1080)
    }

    #[test]
    fn test_windows_map_updated_on_swap() {
        let bounds = create_test_bounds();
        let initial_windows = vec![create_mock_window(1)];
        let mut tree = ContainerTree::new(bounds, &initial_windows);

        // Initial window should be in the map
        assert_eq!(tree.windows.len(), 1);
        assert!(tree.windows.contains_key(&1));

        // Insert a new window
        let new_window = create_mock_window(2);
        let position = Position { x: 500, y: 500 };

        let result = tree.insert_window(&new_window, &position);
        assert!(result.is_ok());

        assert_eq!(tree.windows.len(), 1,);
        assert!(tree.windows.contains_key(&2),);
    }

    #[test]
    fn test_windows_map_updated_on_remove() {
        let bounds = create_test_bounds();
        let initial_windows = vec![create_mock_window(1), create_mock_window(2)];
        let mut tree = ContainerTree::new(bounds, &initial_windows);

        // Both windows should be in the map initially
        assert_eq!(tree.windows.len(), 2);
        assert!(tree.windows.contains_key(&1));
        assert!(tree.windows.contains_key(&2));

        // Remove a window
        let window_to_remove = &initial_windows[0];
        let result = tree.remove_window(window_to_remove);
        assert!(result.is_ok());

        assert_eq!(tree.windows.len(), 1,);
        assert!(!tree.windows.contains_key(&1),);
        assert!(tree.windows.contains_key(&2),);
    }

    #[test]
    fn test_windows_map_updated_on_replace() {
        let bounds = create_test_bounds();
        let initial_windows = vec![create_mock_window(1)];
        let mut tree = ContainerTree::new(bounds, &initial_windows);

        // Initial window should be in the map
        assert_eq!(tree.windows.len(), 1);
        assert!(tree.windows.contains_key(&1));

        // Replace the window with a new one
        let old_window = &initial_windows[0];
        let new_window = create_mock_window(2);
        let result = tree.replace_window(old_window, &new_window);
        assert!(result.is_ok());

        assert_eq!(tree.windows.len(), 1,);
        assert!(!tree.windows.contains_key(&1),);
        assert!(tree.windows.contains_key(&2),);
    }

    #[test]
    fn test_windows_map_updated_on_split() {
        let bounds = create_test_bounds();
        let initial_windows = vec![create_mock_window(1)];
        let mut tree = ContainerTree::new(bounds, &initial_windows);

        // Initial window should be in the map
        assert_eq!(tree.windows.len(), 1);
        assert!(tree.windows.contains_key(&1));

        // Insert a window that will trigger a split action
        // Position it near the edge of the first window to trigger a split
        let new_window = create_mock_window(2);
        let position = Position { x: 400, y: 300 }; // Close to left edge to trigger split

        let result = tree.insert_window(&new_window, &position);
        assert!(result.is_ok());

        // Windows map should now contain both windows
        assert_eq!(tree.windows.len(), 2,);
        assert!(tree.windows.contains_key(&1),);
        assert!(tree.windows.contains_key(&2),);
    }

    #[test]
    fn test_windows_map_updated_on_add_to_parent() {
        let bounds = create_test_bounds();
        let initial_windows = vec![create_mock_window(1), create_mock_window(2)];
        let mut tree = ContainerTree::new(bounds, &initial_windows);

        // Both windows should be in the map initially
        assert_eq!(tree.windows.len(), 2);
        assert!(tree.windows.contains_key(&1));
        assert!(tree.windows.contains_key(&2));

        // Insert a third window that will trigger AddToParent action
        // Position it at the far edge to trigger adding to parent container
        let new_window = create_mock_window(3);
        let position = Position { x: 100, y: 300 }; // Far left to trigger add to parent

        let result = tree.insert_window(&new_window, &position);
        assert!(result.is_ok());

        // Windows map should now contain all three windows
        assert_eq!(tree.windows.len(), 3,);
        assert!(tree.windows.contains_key(&1),);
        assert!(tree.windows.contains_key(&2),);
        assert!(tree.windows.contains_key(&3),);
    }

    #[test]
    fn test_windows_map_updated_on_fill_root() {
        let bounds = create_test_bounds();
        let initial_windows = vec![];
        let mut tree = ContainerTree::new(bounds, &initial_windows);

        // Initially no windows
        assert_eq!(tree.windows.len(), 0);

        // Insert first window into empty tree (should trigger FillRoot)
        let new_window = create_mock_window(1);
        let position = Position { x: 500, y: 500 };

        let result = tree.insert_window(&new_window, &position);
        assert!(result.is_ok());

        // Windows map should now contain the window
        assert_eq!(tree.windows.len(), 1,);
        assert!(tree.windows.contains_key(&1),);
    }
}
