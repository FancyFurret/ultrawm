use crate::layouts::container_tree::container::{ContainerChildRef, ContainerRef, WindowRef};

pub struct ContainerTreeIterator {
    stack: Vec<ContainerRef>,
    current_windows: Vec<WindowRef>,
}

impl ContainerTreeIterator {
    pub fn new(root: ContainerRef) -> Self {
        Self {
            stack: vec![root],
            current_windows: Vec::new(),
        }
    }
}
impl Iterator for ContainerTreeIterator {
    type Item = WindowRef;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(container) = self.stack.pop() {
            for child in container.children().iter() {
                match child {
                    ContainerChildRef::Container(container) => self.stack.push(container.clone()),
                    ContainerChildRef::Window(window) => self.current_windows.push(window.clone()),
                }
            }
        }

        if let Some(window) = self.current_windows.pop() {
            return Some(window.clone());
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::container_tree::tests::{new_container, new_window};

    #[test]
    fn test_iterator() {
        let root = new_container();
        for _ in 0..10 {
            root.add_window(new_window().into());
        }
        let mut iterator = ContainerTreeIterator::new(root.clone());
        for _ in 0..10 {
            assert!(iterator.next().is_some());
        }
        assert!(iterator.next().is_none());
    }
}
