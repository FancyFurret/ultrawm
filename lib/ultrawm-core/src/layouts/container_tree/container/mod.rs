pub use container_ref::*;
pub use container_window::*;

use crate::layouts::{Direction, ResizeDirection};
use crate::platform::Bounds;
use std::cell::{Ref, RefCell, RefMut};
use std::rc::{Rc, Weak};
use std::sync::atomic::Ordering;

use super::Side;
use crate::layouts::container_tree::{ContainerId, CONTAINER_ID_COUNTER};

pub mod container_ref;
mod container_window;

pub type ParentContainerRef = Weak<Container>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InsertOrder {
    Before,
    After,
}

impl From<Side> for InsertOrder {
    fn from(side: Side) -> InsertOrder {
        match side {
            Side::Left | Side::Top => InsertOrder::Before,
            Side::Right | Side::Bottom => InsertOrder::After,
        }
    }
}

impl Default for InsertOrder {
    fn default() -> Self {
        InsertOrder::After
    }
}

/// Behavior for distributing size changes when a child is resized.
///
/// * `Spread` ‒ the delta is spread equally across all other siblings (current default).
/// * `Symmetric` ‒ the delta is applied equally to the *two* groups on either side of the
///   resized child so it grows / shrinks outward symmetrically, keeping the child centred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeDistribution {
    Spread,
    Symmetric,
}

#[derive(Debug)]
pub struct Container {
    id: ContainerId,
    bounds: RefCell<Bounds>,
    direction: Direction,
    parent: RefCell<Option<ParentContainerRef>>,
    children: RefCell<Vec<ContainerChildRef>>,
    ratios: RefCell<Vec<f32>>,
    self_ref: RefCell<ParentContainerRef>,
}

impl PartialEq for Container {
    fn eq(&self, other: &Self) -> bool {
        self as *const Self == other as *const Self
    }
}

impl Container {
    pub fn new_root(bounds: Bounds) -> ContainerRef {
        Self::new(bounds, Direction::Horizontal, None)
    }

    pub fn new(
        bounds: Bounds,
        direction: Direction,
        parent: Option<ParentContainerRef>,
    ) -> ContainerRef {
        let id = CONTAINER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let self_rc = Rc::new(Self {
            id,
            bounds: RefCell::new(bounds),
            direction,
            parent: RefCell::new(parent),
            children: RefCell::new(Vec::new()),
            ratios: RefCell::new(Vec::new()),
            self_ref: RefCell::new(Weak::new()),
        });

        self_rc.self_ref.replace(Rc::downgrade(&self_rc));
        self_rc
    }

    pub fn id(&self) -> ContainerId {
        self.id
    }

    pub fn bounds(&self) -> Bounds {
        self.bounds.borrow().clone()
    }

    fn set_bounds(&self, bounds: Bounds) {
        self.bounds.replace(bounds);
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn ratios(&self) -> Ref<Vec<f32>> {
        self.ratios.borrow()
    }

    pub fn set_ratios(&self, ratios: Vec<f32>) {
        self.ratios.replace(ratios);
        self.normalize_ratios();
    }

    pub fn parent(&self) -> Option<ContainerRef> {
        self.parent
            .borrow()
            .as_ref()
            .map(|parent| parent.upgrade().unwrap())
    }

    fn set_parent(&self, parent: ParentContainerRef) {
        self.parent.replace(Some(parent));
    }

    pub fn children(&self) -> Ref<Vec<ContainerChildRef>> {
        self.children.borrow()
    }

    fn children_mut(&self) -> RefMut<Vec<ContainerChildRef>> {
        self.children.borrow_mut()
    }

    pub fn self_ref(&self) -> ParentContainerRef {
        self.self_ref.borrow().clone()
    }

    pub fn index_of_child(&self, child: &ContainerChildRef) -> Option<usize> {
        self.children().iter().position(|c| c == child)
    }

    pub fn add_window(&self, window: ContainerWindowRef) -> ContainerWindowRef {
        let index = self.children().len();
        self.insert_window(index, window)
    }

    pub fn insert_window(
        &self,
        index: usize,
        window_ref: ContainerWindowRef,
    ) -> ContainerWindowRef {
        let child = ContainerChildRef::Window(window_ref.clone());
        self.insert_child(index, child);
        window_ref
    }

    pub fn add_container(&self, container: ContainerRef) -> ContainerRef {
        let index = self.children().len();
        self.insert_container(index, container)
    }

    pub fn insert_container(&self, index: usize, container: ContainerRef) -> ContainerRef {
        let child = ContainerChildRef::Container(container.clone());
        self.insert_child(index, child);
        container
    }

    fn insert_child(&self, mut index: usize, child: ContainerChildRef) {
        // If the window is already in this container, remove it
        let current_index = self.children().iter().position(|c| c == &child);
        if let Some(current_index) = current_index {
            if current_index < index {
                index -= 1;
            }

            self.children_mut().remove(current_index);
            self.ratios.borrow_mut().remove(current_index);
        }

        // Insert the window into this container
        self.children_mut().insert(index, child.clone());

        // Handle ratios for the new insertion
        self.insert_ratio_at_index(index);

        // If the window has a different parent, remove it from its old parent
        if let Some(parent) = child.parent() {
            if self.self_ref.as_ptr() != parent.self_ref.as_ptr() {
                // Remove the window from its current parent
                child.set_parent(self.self_ref());
                // Do this very last, since it can potentially remove self, if self is now the only child
                parent.remove_child(&child);
            }
        } else {
            child.set_parent(self.self_ref());
        }
    }

    /// Insert a new ratio at the given index, making the new item get 1/N of the space
    /// while existing items maintain their relative ratios
    fn insert_ratio_at_index(&self, index: usize) {
        let children_count = self.children().len();
        if children_count == 0 {
            return;
        }

        if children_count == 1 {
            // First child gets ratio of 1.0
            self.ratios.replace(vec![1.0]);
            return;
        }

        let new_ratio = 1.0 / (children_count - 1) as f32;
        self.ratios.borrow_mut().insert(index, new_ratio);
        self.normalize_ratios();
    }

    /// Normalize ratios so they sum to the target total (usually 1.0)
    fn normalize_ratios(&self) {
        let mut ratios = self.ratios.borrow_mut();
        let current_total: f32 = ratios.iter().sum();

        if current_total > 0.0 && (current_total - 1.0).abs() > f32::EPSILON {
            let scale_factor = 1.0 / current_total;
            for ratio in ratios.iter_mut() {
                *ratio *= scale_factor;
            }
        }
    }

    /// Distribute a delta across specified indices
    fn distribute_delta_across_indices(&self, indices: &[usize], delta: f32, min_weight: f32) {
        if indices.is_empty() {
            return;
        }

        let mut ratios = self.ratios.borrow_mut();
        let per_delta = delta / indices.len() as f32;

        for &idx in indices {
            if idx < ratios.len() {
                ratios[idx] += per_delta;
                if ratios[idx] < min_weight {
                    ratios[idx] = min_weight;
                }
            }
        }
    }

    pub fn split_window(
        &self,
        window_to_split: &ContainerWindowRef,
        new_window: ContainerWindowRef,
        order: InsertOrder,
    ) -> ContainerRef {
        let new_container = Container::new(
            window_to_split.bounds().clone(),
            self.direction.opposite(),
            Some(self.self_ref()),
        );

        self.replace_child(
            &ContainerChildRef::Window(window_to_split.clone()),
            ContainerChildRef::Container(new_container.clone()),
        );

        match order {
            InsertOrder::Before => {
                new_container.add_window(window_to_split.clone());

                // Do this last, in case new_window is already in the container
                new_container.insert_window(0, new_window.clone());
            }
            InsertOrder::After => {
                new_container.add_window(window_to_split.clone());
                new_container.add_window(new_window.clone());
            }
        }

        new_container
    }

    pub fn split_self(&self, new_window: ContainerWindowRef, order: InsertOrder) -> ContainerRef {
        let split_container =
            Container::new(self.bounds().clone(), self.direction.opposite(), None);

        let new_container = Container::new(self.bounds().clone(), self.direction, None);

        for child in self.children().iter() {
            new_container.children_mut().push(child.clone());
            new_container.ratios.replace(self.ratios.borrow().clone());
            child.set_parent(new_container.self_ref());
        }

        self.children_mut().clear();
        self.ratios.borrow_mut().clear();
        let split_container = self.add_container(split_container);

        match order {
            InsertOrder::Before => {
                split_container.add_window(new_window.clone());
                split_container.add_container(new_container.clone());
            }
            InsertOrder::After => {
                split_container.add_container(new_container.clone());
                split_container.add_window(new_window.clone());
            }
        }

        split_container
    }

    pub fn replace_child(&self, old_child: &ContainerChildRef, new_child: ContainerChildRef) {
        // Ensure the new child has the correct parent
        let index = self.index_of_child(old_child);
        if index.is_none() {
            return;
        }

        let index = index.unwrap();
        new_child.set_parent(self.self_ref());
        self.children_mut()[index] = new_child.clone();
    }

    pub fn swap(a: &ContainerChildRef, b: &ContainerChildRef) {
        let a_parent = a.parent().unwrap();
        let b_parent = b.parent().unwrap();
        let a_index = a_parent.children().iter().position(|c| c == a).unwrap();
        let b_index = b_parent.children().iter().position(|c| c == b).unwrap();
        a_parent.children_mut()[a_index] = b.clone();
        b_parent.children_mut()[b_index] = a.clone();
        a.set_parent(b_parent.self_ref());
        b.set_parent(a_parent.self_ref());
    }

    pub fn remove_child(&self, child: &ContainerChildRef) {
        // Remove the child and its corresponding ratio weight
        if let Some(index) = self.index_of_child(child) {
            self.children_mut().remove(index);
            if index < self.ratios.borrow().len() {
                self.ratios.borrow_mut().remove(index);
                self.normalize_ratios();
            }
        }

        // If there is only one child left, remove ourselves
        if self.children().len() == 1 && self.parent().is_some() {
            let parent = self.parent().unwrap();
            let self_ref = self.self_ref().upgrade().unwrap();
            let child = self.children_mut().pop().unwrap();
            self.ratios.borrow_mut().pop();
            let self_index = parent
                .index_of_child(&ContainerChildRef::Container(self_ref.clone()))
                .unwrap();

            match child {
                // If it's a container, add all of its children to the parent
                ContainerChildRef::Container(c) => {
                    for child in c.children().iter().rev() {
                        let grandchild = child.clone();
                        grandchild.set_parent(parent.self_ref());
                        parent.children_mut().insert(self_index, grandchild);
                        parent.insert_ratio_at_index(self_index);
                    }
                    parent.remove_child(&ContainerChildRef::Container(self_ref));
                }

                // If it's a window, just add it to the parent
                ContainerChildRef::Window(w) => {
                    parent.replace_child(
                        &ContainerChildRef::Container(self_ref),
                        ContainerChildRef::Window(w.clone()),
                    );
                }
            };
        }
    }

    pub fn equalize_ratios(&self) {
        let children = self.children();
        if children.is_empty() {
            return;
        }

        let ratio = 1.0 / children.len() as f32;
        self.ratios.replace(vec![ratio; children.len()]);
    }

    pub fn calculate_bounds(&self) {
        // Early exit if no children
        let children = self.children();
        if children.is_empty() {
            return;
        }

        // Get all data we need upfront to minimize borrows
        let ratios = self.ratios.borrow();
        let total_weight: f32 = ratios.iter().sum::<f32>().max(1.0);
        let container_size: u32 = match self.direction {
            Direction::Horizontal => self.bounds().size.width,
            Direction::Vertical => self.bounds().size.height,
        };
        let start_position: i32 = match self.direction {
            Direction::Horizontal => self.bounds().position.x,
            Direction::Vertical => self.bounds().position.y,
        };

        // Pre-calculate all sizes to avoid floating point errors accumulating
        let mut sizes: Vec<u32> = Vec::with_capacity(children.len());
        let mut remaining_size = container_size as i32;

        for (idx, weight) in ratios.iter().enumerate() {
            let is_last = idx == children.len() - 1;
            let size = if is_last {
                remaining_size.max(0) as u32
            } else {
                let size = ((container_size as f32 * *weight) / total_weight).round() as u32;
                remaining_size -= size as i32;
                size
            };
            sizes.push(size);
        }

        // Apply all sizes in a single pass
        let mut current_position = start_position;
        for (child, &size) in children.iter().zip(sizes.iter()) {
            let new_bounds = match self.direction {
                Direction::Horizontal => Bounds::new(
                    current_position,
                    self.bounds().position.y,
                    size,
                    self.bounds().size.height,
                ),
                Direction::Vertical => Bounds::new(
                    self.bounds().position.x,
                    current_position,
                    self.bounds().size.width,
                    size,
                ),
            };
            child.set_bounds(new_bounds);
            current_position += size as i32;

            // Only recurse if it's a container
            if let ContainerChildRef::Container(c) = child {
                c.calculate_bounds();
            }
        }
    }

    pub fn resize_child(
        &self,
        child: &ContainerChildRef,
        bounds: &Bounds,
        direction: ResizeDirection,
        mode: ResizeDistribution,
    ) {
        let index = match self.index_of_child(child) {
            Some(i) => i,
            None => return,
        };

        // Can't adjust ratios meaningfully with only one child.
        if self.children().len() < 2 {
            return;
        }

        let container_size = match self.direction {
            Direction::Horizontal => self.bounds().size.width as f32,
            Direction::Vertical => self.bounds().size.height as f32,
        };

        if container_size <= 0.0 {
            return;
        }

        // Calculate new_weight and delta before borrowing ratios mutably
        let (total_weight, old_weight, ratios_len): (f32, f32, usize);
        {
            let ratios = self.ratios.borrow();
            total_weight = ratios.iter().sum::<f32>();
            old_weight = ratios[index];
            ratios_len = ratios.len();
        }

        let new_size = match self.direction {
            Direction::Horizontal => bounds.size.width as f32,
            Direction::Vertical => bounds.size.height as f32,
        };

        let mut new_weight = (new_size / container_size) * total_weight;
        let min_weight = 0.1_f32;
        if new_weight < min_weight {
            new_weight = min_weight;
        }
        let delta = new_weight - old_weight;

        // Update the ratio for the resized child
        {
            let mut ratios = self.ratios.borrow_mut();
            ratios[index] = new_weight;
        }

        match mode {
            ResizeDistribution::Spread => {
                let affect_left = match self.direction {
                    Direction::Horizontal => direction.has_left(),
                    Direction::Vertical => direction.has_top(),
                };
                let indices: Vec<usize> = if affect_left {
                    (0..index).collect()
                } else {
                    (index + 1..ratios_len).collect()
                };
                if indices.is_empty() {
                    let all_indices: Vec<usize> = (0..ratios_len).filter(|&i| i != index).collect();
                    self.distribute_delta_across_indices(&all_indices, -delta, min_weight);
                } else {
                    self.distribute_delta_across_indices(&indices, -delta, min_weight);
                }
            }
            ResizeDistribution::Symmetric => {
                let left_count = index;
                let right_count = ratios_len - index - 1;
                if left_count == 0 || right_count == 0 {
                    // Fallback to Spread logic: distribute among all siblings except the resized one
                    let all_indices: Vec<usize> = (0..ratios_len).filter(|&i| i != index).collect();
                    self.distribute_delta_across_indices(&all_indices, -delta, min_weight);
                } else {
                    let left_indices: Vec<usize> = (0..index).collect();
                    let right_indices: Vec<usize> = (index + 1..ratios_len).collect();
                    self.distribute_delta_across_indices(&left_indices, -delta / 2.0, min_weight);
                    self.distribute_delta_across_indices(&right_indices, -delta / 2.0, min_weight);
                }
            }
        }

        // Re-normalize weights so their sum equals original total_weight
        self.normalize_ratios();
    }

    /// Resize the split between children at the given index based on a new position
    /// The index represents the first child of the right group (children at index and after)
    pub fn resize_between(&self, split_index: usize, new_position: i32) -> bool {
        let children = self.children();
        if children.len() <= 1 || split_index >= children.len() || split_index == 0 {
            return false;
        }

        let container_bounds = self.bounds();

        // Calculate the new split position based on handle movement
        let (container_start, container_size) = match self.direction {
            Direction::Horizontal => {
                // Horizontal layout - split position is vertical (x coordinate)
                let start = container_bounds.position.x;
                (start, container_bounds.size.width as f32)
            }
            Direction::Vertical => {
                // Vertical layout - split position is horizontal (y coordinate)
                let start = container_bounds.position.y;
                (start, container_bounds.size.height as f32)
            }
        };

        // Calculate the ratio for the left side based on the new split position
        let left_ratio = (new_position - container_start) as f32 / container_size;
        let right_ratio = 1.0 - left_ratio;

        // Ensure minimum ratios
        let min_ratio = 0.1;
        let (left_ratio, right_ratio) = if left_ratio < min_ratio {
            (min_ratio, 1.0 - min_ratio)
        } else if right_ratio < min_ratio {
            (1.0 - min_ratio, min_ratio)
        } else {
            (left_ratio, right_ratio)
        };

        // Get current ratios and calculate scaling factors
        {
            let mut ratios = self.ratios.borrow_mut();
            let total_weight: f32 = ratios.iter().sum();

            // Calculate current left and right weights
            let current_left_weight: f32 = ratios[0..split_index].iter().sum();
            let current_right_weight: f32 = ratios[split_index..].iter().sum();

            // Calculate new total weights for left and right sides
            let new_left_weight = left_ratio * total_weight;
            let new_right_weight = right_ratio * total_weight;

            // Scale left side ratios
            if current_left_weight > 0.0 {
                let left_scale = new_left_weight / current_left_weight;
                for i in 0..split_index {
                    ratios[i] *= left_scale;
                }
            }

            // Scale right side ratios
            if current_right_weight > 0.0 {
                let right_scale = new_right_weight / current_right_weight;
                for i in split_index..ratios.len() {
                    ratios[i] *= right_scale;
                }
            }
        }

        self.normalize_ratios();

        true
    }
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::container_tree::tests::{
        assert_is_container, assert_is_window, assert_window, new_bounds, new_container, new_window,
    };

    pub(super) fn new_container_with_bounds(bounds: Bounds) -> ContainerRef {
        Container::new(bounds.clone(), Direction::Horizontal, None)
    }

    pub(super) fn new_container_with_direction(direction: Direction) -> ContainerRef {
        Container::new(new_bounds(), direction, None)
    }

    pub(super) fn new_container_with_parent(parent: ContainerRef) -> ContainerRef {
        Container::new(new_bounds(), Direction::Horizontal, Some(parent.self_ref()))
    }

    #[test]
    fn test_bounds() {
        let root = new_container_with_bounds(new_bounds());
        assert_eq!(root.bounds(), new_bounds());
    }

    #[test]
    fn test_direction() {
        let root = new_container_with_direction(Direction::Horizontal);
        assert_eq!(root.direction(), Direction::Horizontal);
    }

    #[test]
    fn test_parent() {
        let root = new_container();
        let container = new_container_with_parent(root.clone());
        assert_eq!(&container.parent(), &Some(root));
    }

    #[test]
    fn test_children() {
        let root = new_container();
        assert_eq!(root.children().len(), 0);
    }

    #[test]
    fn test_add_new_window() {
        let root = new_container();
        let window = root.add_window(new_window());
        assert_eq!(root.children().len(), 1);
        assert_window(&root.children()[0], &window);
        assert_eq!(window.parent(), root);
    }

    #[test]
    fn test_add_new_window_multiple() {
        let root = new_container();

        let ref_a = root.add_window(new_window());
        let ref_b = root.add_window(new_window());
        let ref_c = root.add_window(new_window());

        assert_eq!(root.children().len(), 3);
        assert_window(&root.children()[0], &ref_a);
        assert_window(&root.children()[1], &ref_b);
        assert_window(&root.children()[2], &ref_c);
        assert_eq!(ref_a.parent(), root);
        assert_eq!(ref_b.parent(), root);
        assert_eq!(ref_c.parent(), root);
    }

    #[test]
    fn test_add_existing_window() {
        let root_a = new_container();
        let root_b = new_container();
        let ref_a = root_a.add_window(new_window());
        let ref_b = root_b.add_window(new_window());

        root_a.add_window(ref_b.clone());

        assert_eq!(root_a.children().len(), 2);
        assert_window(&root_a.children()[0], &ref_a);
        assert_window(&root_a.children()[1], &ref_b);
        assert_eq!(ref_a.parent(), root_a);
        assert_eq!(ref_b.parent(), root_a);
        assert_eq!(root_b.children().len(), 0);
    }

    #[test]
    fn test_add_existing_window_same_window() {
        let root = new_container();
        let ref_a = root.add_window(new_window());

        root.add_window(ref_a.clone());

        assert_eq!(root.children().len(), 1);
        assert_window(&root.children()[0], &ref_a);
        assert_eq!(ref_a.parent(), root);
    }

    #[test]
    fn test_insert_existing_window_same_parent() {
        let root = new_container();
        let ref_a = root.add_window(new_window());
        let ref_b = root.add_window(new_window());

        root.insert_window(0, ref_b.clone());

        assert_eq!(root.children().len(), 2);
        assert_window(&root.children()[0], &ref_b);
        assert_window(&root.children()[1], &ref_a);
        assert_eq!(ref_a.parent(), root);
        assert_eq!(ref_b.parent(), root);
    }

    #[test]
    fn test_insert_existing_window_collapsing_container() {
        let root = new_container();
        let ref_a = root.add_window(new_window());
        let ref_b = root.add_window(new_window());
        let container = root.split_self(new_window(), InsertOrder::default());
        let child_container = assert_is_container(&container.children()[0]);
        let ref_c = assert_is_window(&container.children()[1]);

        // Should remove container/child_container, since it is now the only child of container,
        // and all 3 windows should now be children of root
        assert_eq!(child_container.children().len(), 2);
        child_container.insert_window(0, ref_c.clone());

        assert_eq!(root.children().len(), 3);
        assert_window(&root.children()[0], &ref_c);
        assert_window(&root.children()[1], &ref_a);
        assert_window(&root.children()[2], &ref_b);
        assert_eq!(ref_a.parent(), root);
        assert_eq!(ref_b.parent(), root);
        assert_eq!(ref_c.parent(), root);
    }

    #[test]
    fn test_insert_new_window() {
        let root = new_container();
        let ref_a = root.add_window(new_window());
        let ref_b = root.add_window(new_window());

        root.insert_window(0, new_window());

        assert_eq!(root.children().len(), 3);
        assert_is_window(&root.children()[0]);
        assert_window(&root.children()[1], &ref_a);
        assert_window(&root.children()[2], &ref_b);
    }

    #[test]
    fn test_split_new_window() {
        let root = new_container();
        let ref_a = root.add_window(new_window());
        let window_b = new_window();

        let new_container = root.split_window(&ref_a, window_b, InsertOrder::default());
        let ref_b = new_container.children()[1].clone();

        assert_eq!(root.children().len(), 1);
        assert_is_container(&root.children()[0]);
        assert_eq!(new_container.children().len(), 2);
        assert_window(&new_container.children()[0], &ref_a);
        assert_is_window(&ref_b);
        assert_eq!(ref_a.parent(), new_container);
        assert_eq!(ref_b.parent(), Some(new_container.clone()));
        assert_eq!(new_container.parent(), Some(root));
    }

    #[test]
    fn test_split_existing_window() {
        let root_a = new_container();
        let root_b = new_container();
        let ref_a = root_a.add_window(new_window());
        let ref_b = root_b.add_window(new_window());

        let new_container = root_a.split_window(&ref_a, ref_b.clone(), InsertOrder::default());

        assert_eq!(root_a.children().len(), 1);
        assert_is_container(&root_a.children()[0]);
        assert_eq!(new_container.children().len(), 2);
        assert_window(&new_container.children()[0], &ref_a);
        assert_window(&new_container.children()[1], &ref_b);
        assert_eq!(ref_a.parent(), new_container);
        assert_eq!(ref_b.parent(), new_container);
        assert_eq!(new_container.parent(), Some(root_a));
        assert_eq!(root_b.children().len(), 0);
    }

    #[test]
    fn test_split_existing_window_same_parent() {
        let root = new_container();
        let ref_a = root.add_window(new_window());
        let ref_b = root.add_window(new_window());

        let new_container = root.split_window(&ref_a, ref_b.clone(), InsertOrder::default());

        assert_eq!(root.children().len(), 1);
        assert_is_container(&root.children()[0]);
        assert_eq!(new_container.children().len(), 2);
        assert_window(&new_container.children()[0], &ref_a);
        assert_window(&new_container.children()[1], &ref_b);
    }

    #[test]
    fn test_swap_same_parent() {
        let root = new_container();
        let ref_a = root.add_window(new_window());
        let ref_b = root.add_window(new_window());
        let ref_c = root.add_window(new_window());

        Container::swap(
            &ContainerChildRef::Window(ref_a.clone()),
            &ContainerChildRef::Window(ref_c.clone()),
        );

        assert_eq!(root.children().len(), 3);
        assert_window(&root.children()[0], &ref_c);
        assert_window(&root.children()[1], &ref_b);
        assert_window(&root.children()[2], &ref_a);
        assert_eq!(ref_a.parent(), root);
        assert_eq!(ref_b.parent(), root);
        assert_eq!(ref_c.parent(), root);
    }

    #[test]
    fn test_swap_child_parent() {
        let root = new_container();
        let ref_a = root.add_window(new_window());
        let ref_b = root.add_window(new_window());
        let new_container = root.split_window(&ref_b, new_window(), InsertOrder::default());
        let ref_c = assert_is_window(&new_container.children()[1]);

        Container::swap(
            &ContainerChildRef::Window(ref_a.clone()),
            &ContainerChildRef::Window(ref_c.clone()),
        );

        assert_eq!(root.children().len(), 2);
        assert_window(&root.children()[0], &ref_c);
        assert_eq!(
            root.children()[1],
            ContainerChildRef::Container(new_container.clone())
        );
        assert_eq!(new_container.children().len(), 2);
        assert_window(&new_container.children()[0], &ref_b);
        assert_window(&new_container.children()[1], &ref_a);
    }

    #[test]
    fn test_add_existing_window_last_child() {
        let root_a = new_container();
        let root_b = new_container();
        let ref_a = root_a.add_window(new_window());

        root_b.add_window(ref_a.clone());

        assert_eq!(root_a.children().len(), 0);
        assert_eq!(root_b.children().len(), 1);
        assert_window(&root_b.children()[0], &ref_a);
        assert_eq!(&ref_a.parent(), &root_b);
    }

    #[test]
    fn test_add_existing_window_last_of_split() {
        let root_a = new_container();
        let root_b = new_container();
        let ref_a = root_a.add_window(new_window());
        let new_container = root_a.split_window(&ref_a, new_window(), InsertOrder::default());
        let ref_b = assert_is_window(&new_container.children()[1]);

        root_b.add_window(ref_b.clone());

        assert_eq!(root_a.children().len(), 1);
        // Since there is only one child left in that container, it should be turned back window
        assert_window(&root_a.children()[0], &ref_a);
        assert_eq!(&ref_a.parent(), &root_a);
        assert_eq!(root_b.children().len(), 1);
        assert_window(&root_b.children()[0], &ref_b);
        assert_eq!(&ref_b.parent(), &root_b);
    }

    #[test]
    fn test_add_existing_window_last_of_split_with_splits() {
        let root_a = new_container();
        let root_b = new_container();
        let ref_a = root_a.add_window(new_window());
        let new_container_a = root_a.split_window(&ref_a, new_window(), InsertOrder::default());
        let ref_b = assert_is_window(&new_container_a.children()[1]);
        let new_container_b =
            new_container_a.split_window(&ref_b, new_window(), InsertOrder::default());
        let ref_c = assert_is_window(&new_container_b.children()[1]);
        let ref_d = new_container_b.add_window(new_window());

        root_b.add_window(ref_a.clone());

        assert_eq!(root_a.children().len(), 3);
        assert_window(&root_a.children()[0], &ref_b);
        assert_window(&root_a.children()[1], &ref_c);
        assert_window(&root_a.children()[2], &ref_d);
        assert_eq!(&ref_b.parent(), &root_a);
        assert_eq!(&ref_c.parent(), &root_a);
        assert_eq!(&ref_d.parent(), &root_a);
        assert_eq!(root_b.children().len(), 1);
        assert_window(&root_b.children()[0], &ref_a);
        assert_eq!(&ref_a.parent(), &root_b);
    }

    // === Container Operations Tests ===

    #[test]
    fn test_add_new_container() {
        let root = new_container();
        let child_container = new_container_with_bounds(new_bounds());

        root.add_container(child_container.clone());

        assert_eq!(root.children().len(), 1);
        assert_is_container(&root.children()[0]);
        assert_eq!(child_container.parent(), Some(root.clone()));
        assert_eq!(root.ratios().len(), 1);
        assert_eq!(root.ratios()[0], 1.0);
    }

    #[test]
    fn test_add_multiple_containers() {
        let root = new_container();
        let container_a = new_container_with_bounds(new_bounds());
        let container_b = new_container_with_bounds(new_bounds());
        let container_c = new_container_with_bounds(new_bounds());

        root.add_container(container_a.clone());
        root.add_container(container_b.clone());
        root.add_container(container_c.clone());

        assert_eq!(root.children().len(), 3);
        assert_is_container(&root.children()[0]);
        assert_is_container(&root.children()[1]);
        assert_is_container(&root.children()[2]);
        assert_eq!(container_a.parent(), Some(root.clone()));
        assert_eq!(container_b.parent(), Some(root.clone()));
        assert_eq!(container_c.parent(), Some(root.clone()));

        // Check ratios are normalized
        assert_eq!(root.ratios().len(), 3);
        let ratio_sum: f32 = root.ratios().iter().sum();
        assert!((ratio_sum - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_insert_container() {
        let root = new_container();
        let container_a = new_container_with_bounds(new_bounds());
        let container_b = new_container_with_bounds(new_bounds());
        let container_c = new_container_with_bounds(new_bounds());

        root.add_container(container_a.clone());
        root.add_container(container_c.clone());
        root.insert_container(1, container_b.clone());

        assert_eq!(root.children().len(), 3);
        assert_eq!(assert_is_container(&root.children()[0]), container_a);
        assert_eq!(assert_is_container(&root.children()[1]), container_b);
        assert_eq!(assert_is_container(&root.children()[2]), container_c);
    }

    #[test]
    fn test_add_existing_container_different_parent() {
        let root_a = new_container();
        let root_b = new_container();
        let container = new_container_with_bounds(new_bounds());

        root_a.add_container(container.clone());
        assert_eq!(root_a.children().len(), 1);
        assert_eq!(container.parent(), Some(root_a.clone()));

        root_b.add_container(container.clone());

        assert_eq!(root_a.children().len(), 0);
        assert_eq!(root_b.children().len(), 1);
        assert_eq!(container.parent(), Some(root_b.clone()));
    }

    #[test]
    fn test_add_existing_container_same_parent() {
        let root = new_container();
        let container = new_container_with_bounds(new_bounds());

        root.add_container(container.clone());
        root.add_container(container.clone());

        assert_eq!(root.children().len(), 1);
        assert_eq!(container.parent(), Some(root.clone()));
    }

    // === Ratio Operations Tests ===

    #[test]
    fn test_equalize_ratios_empty() {
        let root = new_container();
        root.equalize_ratios();
        assert_eq!(root.ratios().len(), 0);
    }

    #[test]
    fn test_equalize_ratios_single_child() {
        let root = new_container();
        root.add_window(new_window());
        root.equalize_ratios();

        assert_eq!(root.ratios().len(), 1);
        assert_eq!(root.ratios()[0], 1.0);
    }

    #[test]
    fn test_equalize_ratios_multiple_children() {
        let root = new_container();
        root.add_window(new_window());
        root.add_window(new_window());
        root.add_window(new_window());

        // Set unequal ratios
        root.set_ratios(vec![0.5, 0.3, 0.2]);

        root.equalize_ratios();

        assert_eq!(root.ratios().len(), 3);
        for ratio in root.ratios().iter() {
            assert!((ratio - (1.0 / 3.0)).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_set_ratios_normalization() {
        let root = new_container();
        root.add_window(new_window());
        root.add_window(new_window());

        // Set ratios that don't sum to 1.0
        root.set_ratios(vec![2.0, 4.0]);

        // Should be normalized to sum to 1.0
        assert_eq!(root.ratios().len(), 2);
        assert!((root.ratios()[0] - (1.0 / 3.0)).abs() < f32::EPSILON);
        assert!((root.ratios()[1] - (2.0 / 3.0)).abs() < f32::EPSILON);

        let sum: f32 = root.ratios().iter().sum();
        assert!((sum - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_insert_ratio_first_child() {
        let root = new_container();
        root.add_window(new_window());

        assert_eq!(root.ratios().len(), 1);
        assert_eq!(root.ratios()[0], 1.0);
    }

    #[test]
    fn test_insert_ratio_second_child() {
        let root = new_container();
        root.add_window(new_window());
        root.add_window(new_window());

        assert_eq!(root.ratios().len(), 2);
        let sum: f32 = root.ratios().iter().sum();
        assert!((sum - 1.0).abs() < f32::EPSILON);
    }

    // === Resize Tests ===

    #[test]
    fn test_resize_child_horizontal_spread() {
        let root = new_container_with_bounds(Bounds::new(0, 0, 1000, 500));
        root.add_window(new_window());
        let window_b = root.add_window(new_window());
        root.add_window(new_window());

        // Make window_b larger using spread distribution
        let new_bounds = Bounds::new(250, 0, 500, 500); // Takes up 50% of container
        root.resize_child(
            &ContainerChildRef::Window(window_b.clone()),
            &new_bounds,
            ResizeDirection::Right,
            ResizeDistribution::Spread,
        );

        // Check that ratios still sum to ~1.0
        let sum: f32 = root.ratios().iter().sum();
        assert!((sum - 1.0).abs() < 0.01);

        // Window B should have a larger ratio than the others
        let ratios = root.ratios();
        assert!(ratios[1] > ratios[0]);
        assert!(ratios[1] > ratios[2]);
    }

    #[test]
    fn test_resize_child_vertical_symmetric() {
        let root = new_container_with_bounds(Bounds::new(0, 0, 500, 1000));
        root.set_ratios(vec![0.25, 0.5, 0.25]); // Start with equal distribution around middle

        root.add_window(new_window());
        root.add_window(new_window());
        root.add_window(new_window());

        // Change direction to vertical for this test
        let vertical_container = new_container_with_direction(Direction::Vertical);
        vertical_container.set_bounds(Bounds::new(0, 0, 500, 1000));
        vertical_container.add_window(new_window());
        let window_b = vertical_container.add_window(new_window());
        vertical_container.add_window(new_window());

        // Make middle window larger using symmetric distribution
        let new_bounds = Bounds::new(0, 200, 500, 600); // Takes up 60% of container
        vertical_container.resize_child(
            &ContainerChildRef::Window(window_b.clone()),
            &new_bounds,
            ResizeDirection::Bottom,
            ResizeDistribution::Symmetric,
        );

        // Check that ratios still sum to ~1.0
        let sum: f32 = vertical_container.ratios().iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_resize_child_single_child() {
        let root = new_container();
        let window = root.add_window(new_window());

        // Resizing with only one child should not change ratios
        let original_ratios = root.ratios().clone();
        root.resize_child(
            &ContainerChildRef::Window(window),
            &Bounds::new(0, 0, 300, 300),
            ResizeDirection::Right,
            ResizeDistribution::Spread,
        );

        assert_eq!(*root.ratios(), original_ratios);
    }

    #[test]
    fn test_resize_child_nonexistent() {
        let root = new_container();
        root.add_window(new_window());
        let window_b = new_window(); // Not added to root

        let original_ratios = root.ratios().clone();
        root.resize_child(
            &ContainerChildRef::Window(window_b),
            &Bounds::new(0, 0, 300, 300),
            ResizeDirection::Right,
            ResizeDistribution::Spread,
        );

        // Should not change ratios if child doesn't exist
        assert_eq!(*root.ratios(), original_ratios);
    }

    // === Resize Between Tests ===

    #[test]
    fn test_resize_between_horizontal() {
        let root = new_container_with_bounds(Bounds::new(0, 0, 1000, 500));
        root.add_window(new_window());
        root.add_window(new_window());
        root.add_window(new_window());

        // Move split between first and second window to 30% position
        let success = root.resize_between(1, 300);

        assert!(success);
        let ratios = root.ratios();

        // Left side (first child) should have ~30% ratio
        assert!((ratios[0] - 0.3).abs() < 0.01);

        // Right side (remaining children) should have ~70% ratio combined
        let right_ratio: f32 = ratios[1..].iter().sum();
        assert!((right_ratio - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_resize_between_vertical() {
        let vertical_container = new_container_with_direction(Direction::Vertical);
        vertical_container.set_bounds(Bounds::new(0, 0, 500, 1000));
        vertical_container.add_window(new_window());
        vertical_container.add_window(new_window());

        // Move split between windows to 60% position (y=600)
        let success = vertical_container.resize_between(1, 600);

        assert!(success);
        let ratios = vertical_container.ratios();

        // First child should have ~60% ratio
        assert!((ratios[0] - 0.6).abs() < 0.01);
        // Second child should have ~40% ratio
        assert!((ratios[1] - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_resize_between_invalid_index() {
        let root = new_container();
        root.add_window(new_window());
        root.add_window(new_window());

        // Invalid indices should fail
        assert!(!root.resize_between(0, 100)); // Can't split before first child
        assert!(!root.resize_between(2, 100)); // Index out of bounds
        assert!(!root.resize_between(3, 100)); // Index way out of bounds
    }

    #[test]
    fn test_resize_between_single_child() {
        let root = new_container();
        root.add_window(new_window());

        // Can't resize between children when there's only one
        assert!(!root.resize_between(1, 100));
    }

    #[test]
    fn test_resize_between_minimum_ratios() {
        let root = new_container_with_bounds(Bounds::new(0, 0, 1000, 500));
        root.add_window(new_window());
        root.add_window(new_window());

        // Try to make left side extremely small (should be clamped to minimum)
        let success = root.resize_between(1, 50); // 5% position

        assert!(success);
        let ratios = root.ratios();

        // Left side should be clamped to minimum ratio (0.1)
        assert!((ratios[0] - 0.1).abs() < 0.01);
        // Right side should get the remainder
        assert!((ratios[1] - 0.9).abs() < 0.01);
    }

    // === Calculate Bounds Tests ===

    #[test]
    fn test_calculate_bounds_empty_container() {
        let root = new_container_with_bounds(Bounds::new(100, 100, 800, 600));
        root.calculate_bounds(); // Should not crash
    }

    #[test]
    fn test_calculate_bounds_single_window() {
        let root = new_container_with_bounds(Bounds::new(100, 100, 800, 600));
        let window = root.add_window(new_window());
        root.calculate_bounds();

        // Single window should get the full container bounds
        assert_eq!(window.bounds(), Bounds::new(100, 100, 800, 600));
    }

    #[test]
    fn test_calculate_bounds_horizontal_split() {
        let root = new_container_with_bounds(Bounds::new(0, 0, 1000, 500));
        let window_a = root.add_window(new_window());
        let window_b = root.add_window(new_window());

        // Set specific ratios
        root.set_ratios(vec![0.6, 0.4]);
        root.calculate_bounds();

        // First window should get 60% width
        assert_eq!(window_a.bounds(), Bounds::new(0, 0, 600, 500));
        // Second window should get 40% width
        assert_eq!(window_b.bounds(), Bounds::new(600, 0, 400, 500));
    }

    #[test]
    fn test_calculate_bounds_vertical_split() {
        let vertical_container = new_container_with_direction(Direction::Vertical);
        vertical_container.set_bounds(Bounds::new(0, 0, 500, 1000));
        let window_a = vertical_container.add_window(new_window());
        let window_b = vertical_container.add_window(new_window());

        // Set specific ratios
        vertical_container.set_ratios(vec![0.3, 0.7]);
        vertical_container.calculate_bounds();

        // First window should get 30% height
        assert_eq!(window_a.bounds(), Bounds::new(0, 0, 500, 300));
        // Second window should get 70% height
        assert_eq!(window_b.bounds(), Bounds::new(0, 300, 500, 700));
    }

    #[test]
    fn test_calculate_bounds_nested_containers() {
        let root = new_container_with_bounds(Bounds::new(0, 0, 1000, 500));
        let window_a = root.add_window(new_window());
        let nested_container = root.split_window(&window_a, new_window(), InsertOrder::After);
        let window_b = assert_is_window(&nested_container.children()[1]);

        root.calculate_bounds();

        // Root should have one child (the nested container)
        assert_eq!(root.children().len(), 1);

        // Nested container should occupy full root bounds
        assert_eq!(nested_container.bounds(), Bounds::new(0, 0, 1000, 500));

        // Windows in nested container should split the space
        // Since nested container has opposite direction (vertical), they should stack vertically
        assert_eq!(window_a.bounds().size.width, 1000);
        assert_eq!(window_b.bounds().size.width, 1000);
        assert_eq!(
            window_a.bounds().size.height + window_b.bounds().size.height,
            500
        );
    }

    #[test]
    fn test_calculate_bounds_rounding_errors() {
        let root = new_container_with_bounds(Bounds::new(0, 0, 333, 500)); // Odd width
        let window_a = root.add_window(new_window());
        let window_b = root.add_window(new_window());
        let window_c = root.add_window(new_window());

        // Equal ratios should handle rounding
        root.equalize_ratios();
        root.calculate_bounds();

        // All windows should have reasonable sizes
        assert!(window_a.bounds().size.width > 0);
        assert!(window_b.bounds().size.width > 0);
        assert!(window_c.bounds().size.width > 0);

        // Total width should equal container width
        let total_width = window_a.bounds().size.width
            + window_b.bounds().size.width
            + window_c.bounds().size.width;
        assert_eq!(total_width, 333);
    }

    // === Split Self Tests ===

    #[test]
    fn test_split_self_before() {
        let root = new_container();
        let existing_window = root.add_window(new_window());
        let new_window = new_window();

        let split_container = root.split_self(new_window.clone(), InsertOrder::Before);

        // Root should now have one child (the split container)
        assert_eq!(root.children().len(), 1);
        assert_is_container(&root.children()[0]);

        // Split container should have two children
        assert_eq!(split_container.children().len(), 2);
        assert_window(&split_container.children()[0], &new_window);

        // The second child should be a container with the existing window
        let nested_container = assert_is_container(&split_container.children()[1]);
        assert_eq!(nested_container.children().len(), 1);
        assert_window(&nested_container.children()[0], &existing_window);
    }

    #[test]
    fn test_split_self_after() {
        let root = new_container();
        let existing_window = root.add_window(new_window());
        let new_window = new_window();

        let split_container = root.split_self(new_window.clone(), InsertOrder::After);

        // Root should now have one child (the split container)
        assert_eq!(root.children().len(), 1);
        assert_is_container(&root.children()[0]);

        // Split container should have two children
        assert_eq!(split_container.children().len(), 2);

        // The first child should be a container with the existing window
        let nested_container = assert_is_container(&split_container.children()[0]);
        assert_eq!(nested_container.children().len(), 1);
        assert_window(&nested_container.children()[0], &existing_window);

        // The second child should be the new window
        assert_window(&split_container.children()[1], &new_window);
    }

    #[test]
    fn test_split_self_multiple_windows() {
        let root = new_container();
        let window_a = root.add_window(new_window());
        let window_b = root.add_window(new_window());
        let window_c = root.add_window(new_window());
        let new_window = new_window();

        let split_container = root.split_self(new_window.clone(), InsertOrder::After);

        // Root should now have one child (the split container)
        assert_eq!(root.children().len(), 1);

        // Split container should have two children
        assert_eq!(split_container.children().len(), 2);

        // First child should be a container with all existing windows
        let nested_container = assert_is_container(&split_container.children()[0]);
        assert_eq!(nested_container.children().len(), 3);
        assert_window(&nested_container.children()[0], &window_a);
        assert_window(&nested_container.children()[1], &window_b);
        assert_window(&nested_container.children()[2], &window_c);

        // Second child should be the new window
        assert_window(&split_container.children()[1], &new_window);
    }

    // === Edge Cases and Error Handling ===

    #[test]
    fn test_replace_child_nonexistent() {
        let root = new_container();
        let window_a = root.add_window(new_window());
        let window_b = new_window(); // Not in container
        let window_c = new_window();

        // Should not crash when trying to replace non-existent child
        root.replace_child(
            &ContainerChildRef::Window(window_b),
            ContainerChildRef::Window(window_c),
        );

        // Original structure should be unchanged
        assert_eq!(root.children().len(), 1);
        assert_window(&root.children()[0], &window_a);
    }

    #[test]
    fn test_remove_child_nonexistent() {
        let root = new_container();
        let window_a = root.add_window(new_window());
        let window_b = new_window(); // Not in container

        // Should not crash when trying to remove non-existent child
        root.remove_child(&ContainerChildRef::Window(window_b));

        // Original structure should be unchanged
        assert_eq!(root.children().len(), 1);
        assert_window(&root.children()[0], &window_a);
    }

    #[test]
    fn test_index_of_child_nonexistent() {
        let root = new_container();
        root.add_window(new_window());
        let window_b = new_window(); // Not in container

        assert_eq!(
            root.index_of_child(&ContainerChildRef::Window(window_b)),
            None
        );
    }

    #[test]
    fn test_self_ref_consistency() {
        let root = new_container();
        let self_ref = root.self_ref();

        // Self reference should be valid and point to the same container
        assert!(self_ref.upgrade().is_some());
        let upgraded = self_ref.upgrade().unwrap();
        assert_eq!(upgraded.id(), root.id());
    }

    #[test]
    fn test_container_ids_unique() {
        let container_a = new_container();
        let container_b = new_container();
        let container_c = new_container();

        // All containers should have unique IDs
        assert_ne!(container_a.id(), container_b.id());
        assert_ne!(container_b.id(), container_c.id());
        assert_ne!(container_a.id(), container_c.id());
    }

    #[test]
    fn test_ratios_empty_container() {
        let root = new_container();
        assert_eq!(root.ratios().len(), 0);
    }

    #[test]
    fn test_bounds_operations() {
        let bounds = Bounds::new(100, 200, 300, 400);
        let root = new_container_with_bounds(bounds.clone());

        assert_eq!(root.bounds(), bounds);

        let new_bounds = Bounds::new(150, 250, 350, 450);
        root.set_bounds(new_bounds.clone());
        assert_eq!(root.bounds(), new_bounds);
    }

    // === Complex Scenarios ===

    #[test]
    fn test_complex_nested_structure() {
        let root = new_container_with_bounds(Bounds::new(0, 0, 1000, 1000));

        // Create a complex nested structure
        let window_a = root.add_window(new_window());
        let window_b = root.add_window(new_window());

        // Split window_a with window_c
        let window_c = new_window();
        let container_ac = root.split_window(&window_a, window_c.clone(), InsertOrder::After);

        // Split window_c with window_d
        let window_d = new_window();
        let container_cd =
            container_ac.split_window(&window_c, window_d.clone(), InsertOrder::After);

        // Verify structure
        assert_eq!(root.children().len(), 2);
        assert_is_container(&root.children()[0]); // container_ac
        assert_window(&root.children()[1], &window_b);

        assert_eq!(container_ac.children().len(), 2);
        assert_window(&container_ac.children()[0], &window_a);
        assert_is_container(&container_ac.children()[1]); // container_cd

        assert_eq!(container_cd.children().len(), 2);
        assert_window(&container_cd.children()[0], &window_c);
        assert_window(&container_cd.children()[1], &window_d);

        // Test bounds calculation
        root.calculate_bounds();

        // All windows should have valid bounds
        assert!(window_a.bounds().size.width > 0);
        assert!(window_a.bounds().size.height > 0);
        assert!(window_b.bounds().size.width > 0);
        assert!(window_b.bounds().size.height > 0);
        assert!(window_c.bounds().size.width > 0);
        assert!(window_c.bounds().size.height > 0);
        assert!(window_d.bounds().size.width > 0);
        assert!(window_d.bounds().size.height > 0);
    }

    #[test]
    fn test_swap_complex_scenario() {
        let root_a = new_container();
        let root_b = new_container();

        let window_a1 = root_a.add_window(new_window());
        let window_a2 = root_a.add_window(new_window());
        let window_b1 = root_b.add_window(new_window());
        let window_b2 = root_b.add_window(new_window());

        Container::swap(
            &ContainerChildRef::Window(window_a1.clone()),
            &ContainerChildRef::Window(window_b2.clone()),
        );

        // Verify the swap
        assert_eq!(root_a.children().len(), 2);
        assert_window(&root_a.children()[0], &window_b2);
        assert_window(&root_a.children()[1], &window_a2);

        assert_eq!(root_b.children().len(), 2);
        assert_window(&root_b.children()[0], &window_b1);
        assert_window(&root_b.children()[1], &window_a1);

        // Verify parents
        assert_eq!(window_a1.parent(), root_b);
        assert_eq!(window_a2.parent(), root_a);
        assert_eq!(window_b1.parent(), root_b);
        assert_eq!(window_b2.parent(), root_a);
    }
}
