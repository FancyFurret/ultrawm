pub use container_ref::*;
pub use container_window::*;

use crate::config::ConfigRef;
use crate::layouts::{Direction, ResizeDirection};
use crate::platform::Bounds;
use std::cell::{Ref, RefCell, RefMut};
use std::rc::{Rc, Weak};

use super::Side;

mod container_ref;
mod container_window;

pub type ParentContainerRef = Weak<Container>;

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

/// Behaviour for distributing size changes when a child is resized.
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
    config: ConfigRef,
    bounds: RefCell<Bounds>,
    direction: Direction,
    parent: Option<RefCell<ParentContainerRef>>,
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
    pub fn new_root(config: ConfigRef, bounds: Bounds) -> ContainerRef {
        Self::new(config, bounds, Direction::Horizontal, None)
    }

    fn new(
        config: ConfigRef,
        bounds: Bounds,
        direction: Direction,
        parent: Option<ParentContainerRef>,
    ) -> ContainerRef {
        let self_rc = Rc::new(Self {
            config,
            bounds: RefCell::new(bounds),
            direction,
            parent: parent.map(RefCell::new),
            children: RefCell::new(Vec::new()),
            ratios: RefCell::new(Vec::new()),
            self_ref: RefCell::new(Weak::new()),
        });

        self_rc.self_ref.replace(Rc::downgrade(&self_rc));
        self_rc
    }

    pub fn bounds(&self) -> Bounds {
        self.bounds.borrow().clone()
    }

    fn set_bounds(&self, bounds: Bounds) {
        self.bounds.replace(bounds);

        // TODO: Shouldnt need to balance?
        self.balance();
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn parent(&self) -> Option<ContainerRef> {
        self.parent
            .as_ref()
            .map(|parent| parent.borrow().upgrade().unwrap())
    }

    fn set_parent(&self, parent: ParentContainerRef) {
        if let Some(parent_ref) = &self.parent {
            parent_ref.replace(parent.clone());
        }
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
        mut index: usize,
        window_ref: ContainerWindowRef,
    ) -> ContainerWindowRef {
        let child = ContainerChildRef::Window(window_ref.clone());

        // If the window is already in this container, remove it
        let current_index = self.children().iter().position(|c| c == &child);
        if let Some(current_index) = current_index {
            if current_index < index {
                index -= 1;
            }

            self.children_mut().remove(current_index);
        }

        // Insert the window into this container
        self.children_mut().insert(index, child.clone());

        // If the window has a different parent, remove it from its old parent
        if self.self_ref.as_ptr() != window_ref.parent().self_ref.as_ptr() {
            // Remove the window from its current parent
            let parent = window_ref.parent();
            window_ref.set_parent(self.self_ref());

            // Do this very last, since it can potentially remove self, if self is now the only child
            parent.remove_child(&child);
            parent.balance();
        }

        window_ref.parent().balance();
        window_ref
    }

    pub fn add_container(&self, container: ContainerRef) -> ContainerRef {
        let index = self.children().len();
        self.insert_container(index, container)
    }

    pub fn insert_container(&self, index: usize, container: ContainerRef) -> ContainerRef {
        let child = ContainerChildRef::Container(container.clone());
        self.children_mut().insert(index, child.clone());

        // If the container has a different parent, remove it from its old parent
        if let Some(parent) = container.parent() {
            if self.self_ref.as_ptr() != parent.self_ref.as_ptr() {
                // Remove the container from its current parent
                container.set_parent(self.self_ref());
                parent.remove_child(&child);
                parent.balance();
            }
        } else {
            container.set_parent(self.self_ref());
        }

        container
    }

    pub fn split_window(
        &self,
        window_to_split: &ContainerWindowRef,
        new_window: ContainerWindowRef,
        order: InsertOrder,
    ) -> ContainerRef {
        let new_container = Container::new(
            self.config.clone(),
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
        let split_container = Container::new(
            self.config.clone(),
            self.bounds().clone(),
            self.direction.opposite(),
            Some(self.self_ref()),
        );

        let new_container = Container::new(
            self.config.clone(),
            self.bounds().clone(),
            self.direction,
            Some(split_container.self_ref()),
        );

        for child in self.children().iter() {
            new_container.children_mut().push(child.clone());
            child.set_parent(new_container.self_ref());
        }

        self.children_mut().clear();
        let split_container = self.add_container(split_container);

        match order {
            InsertOrder::Before => {
                split_container.add_window(new_window.clone());
                split_container
                    .children_mut()
                    .push(ContainerChildRef::Container(new_container.clone()));
            }
            InsertOrder::After => {
                split_container
                    .children_mut()
                    .push(ContainerChildRef::Container(new_container.clone()));
                split_container.add_window(new_window.clone());
            }
        }

        split_container
    }

    pub fn replace_child(&self, old_child: &ContainerChildRef, new_child: ContainerChildRef) {
        // Ensure the new child has the correct parent
        let index = self.index_of_child(old_child);
        if index.is_none() {
            println!("Child not found");
            return;
        }

        let index = index.unwrap();
        new_child.set_parent(self.self_ref());
        self.children_mut()[index] = new_child.clone();
        self.balance();
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
        a_parent.balance();
        b_parent.balance();
    }

    pub fn remove_child(&self, child: &ContainerChildRef) {
        // Remove the child and its corresponding ratio weight
        if let Some(index) = self.index_of_child(child) {
            self.children_mut().remove(index);
            if index < self.ratios.borrow().len() {
                self.ratios.borrow_mut().remove(index);
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

        self.balance();
    }

    pub fn balance(&self) {
        // Ensure ratio vector matches children len
        self.ensure_ratio_len();

        let children = self.children();
        if children.is_empty() {
            return;
        }

        let ratios = self.ratios.borrow();
        let total_weight: f32 = ratios.iter().sum::<f32>().max(1.0);

        let container_size: u32 = match self.direction {
            Direction::Horizontal => self.bounds().size.width,
            Direction::Vertical => self.bounds().size.height,
        };

        let mut current_position: i32 = match self.direction {
            Direction::Horizontal => self.bounds().position.x,
            Direction::Vertical => self.bounds().position.y,
        };

        let mut remaining_size = container_size as i32;

        for (idx, (child, weight)) in children.iter().zip(ratios.iter()).enumerate() {
            let is_last = idx == children.len() - 1;
            let allocated_size: u32 = if is_last {
                remaining_size.max(0) as u32
            } else {
                let size = ((container_size as f32 * *weight) / total_weight).round() as u32;
                remaining_size -= size as i32;
                size
            };

            let new_bounds = match self.direction {
                Direction::Horizontal => Bounds::new(
                    current_position,
                    self.bounds().position.y,
                    allocated_size,
                    self.bounds().size.height,
                ),
                Direction::Vertical => Bounds::new(
                    self.bounds().position.x,
                    current_position,
                    self.bounds().size.width,
                    allocated_size,
                ),
            };
            child.set_bounds(new_bounds);

            current_position += allocated_size as i32;

            if let ContainerChildRef::Container(c) = child {
                c.balance();
            }
        }
    }

    fn ensure_ratio_len(&self) {
        let child_len = self.children().len();
        let mut ratios = self.ratios.borrow_mut();
        if ratios.len() != child_len {
            ratios.clear();
            ratios.resize(child_len, 1.0);
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
            None => return, // child not found in this container
        };

        // Can't adjust ratios meaningfully with only one child.
        if self.children().len() < 2 {
            return;
        }

        self.ensure_ratio_len();

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

        let mut ratios = self.ratios.borrow_mut();
        let distribute_delta = |ratios: &mut Vec<f32>, indices: &[usize], delta: f32| {
            if indices.is_empty() {
                return;
            }
            let per_delta = delta / indices.len() as f32;
            for &idx in indices {
                ratios[idx] += per_delta;
                if ratios[idx] < min_weight {
                    ratios[idx] = min_weight;
                }
            }
        };

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
                    ratios[index] = new_weight;
                    distribute_delta(&mut ratios, &all_indices, -delta);
                } else {
                    ratios[index] = new_weight;
                    distribute_delta(&mut ratios, &indices, -delta);
                }
            }
            ResizeDistribution::Symmetric => {
                let left_count = index;
                let right_count = ratios_len - index - 1;
                if left_count == 0 || right_count == 0 {
                    // Fallback to Spread logic: distribute among all siblings except the resized one
                    let all_indices: Vec<usize> = (0..ratios_len).filter(|&i| i != index).collect();
                    ratios[index] = new_weight;
                    distribute_delta(&mut ratios, &all_indices, -delta);
                } else {
                    let left_indices: Vec<usize> = (0..index).collect();
                    let right_indices: Vec<usize> = (index + 1..ratios_len).collect();
                    ratios[index] = new_weight;
                    distribute_delta(&mut ratios, &left_indices, -delta / 2.0);
                    distribute_delta(&mut ratios, &right_indices, -delta / 2.0);
                }
            }
        }

        // Re-normalize weights so their sum equals original total_weight.
        let new_total: f32 = ratios.iter().sum();
        if new_total != 0.0 {
            let factor = total_weight / new_total;
            for w in ratios.iter_mut() {
                *w *= factor;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::container_tree::tests::{
        assert_is_container, assert_is_window, assert_window, new_bounds, new_config,
        new_container, new_window,
    };

    pub(super) fn new_container_with_bounds(bounds: Bounds) -> ContainerRef {
        Container::new(new_config(), bounds.clone(), Direction::Horizontal, None)
    }

    pub(super) fn new_container_with_direction(direction: Direction) -> ContainerRef {
        Container::new(new_config(), new_bounds(), direction, None)
    }

    pub(super) fn new_container_with_parent(parent: ContainerRef) -> ContainerRef {
        Container::new(
            new_config(),
            new_bounds(),
            Direction::Horizontal,
            Some(parent.self_ref()),
        )
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
    fn test_insert_existing_windowollapsing_container() {
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
}
