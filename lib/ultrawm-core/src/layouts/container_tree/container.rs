use crate::config::ConfigRef;
use crate::layouts::Direction;
use crate::platform::Bounds;
use crate::window::Window;

#[derive(Debug)]
pub enum ContainerChild {
    Container(Container),
    Window(Window),
}

impl ContainerChild {
    pub fn set_bounds(&mut self, bounds: Bounds) {
        match self {
            ContainerChild::Container(container) => container.set_bounds(bounds),
            ContainerChild::Window(window) => window.set_bounds(bounds),
        }
    }
}

#[derive(Debug)]
pub struct Container {
    config: ConfigRef,
    bounds: Bounds,
    direction: Direction,
    children: Vec<ContainerChild>,
    window: Option<Window>,
}

impl Container {
    pub fn new(config: ConfigRef, bounds: Bounds, direction: Direction) -> Self {
        Self {
            config,
            bounds,
            direction,
            children: Vec::new(),
            window: None,
        }
    }

    pub fn add_window(&mut self, window: Window) {
        self.children.push(ContainerChild::Window(window));
        self.balance();
    }

    pub fn balance(&mut self) {
        let num_children = self.children.len() as u32;
        if num_children == 0 {
            return;
        }

        let container_size = match self.direction {
            Direction::Horizontal => self.bounds.size.width,
            Direction::Vertical => self.bounds.size.height,
        };

        let total_gap = self.config.window_gap * (num_children - 1);
        let child_size = (container_size - total_gap) / num_children;
        let mut current_position = match self.direction {
            Direction::Horizontal => self.bounds.position.x,
            Direction::Vertical => self.bounds.position.y,
        };

        for child in &mut self.children {
            let new_bounds = match self.direction {
                Direction::Horizontal => Bounds::new(
                    current_position,
                    self.bounds.position.y,
                    child_size,
                    self.bounds.size.height,
                ),
                Direction::Vertical => Bounds::new(
                    self.bounds.position.x,
                    current_position,
                    self.bounds.size.width,
                    child_size,
                ),
            };
            child.set_bounds(new_bounds);

            current_position += child_size as i32 + self.config.window_gap as i32;
        }
    }

    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    pub fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;

        // TODO: Shouldnt need to balance?
        self.balance();
    }

    pub fn children(&self) -> &Vec<ContainerChild> {
        &self.children
    }
}
