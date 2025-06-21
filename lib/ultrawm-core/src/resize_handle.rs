use crate::platform::{Bounds, Position};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HandleOrientation {
    /// The handle is vertical, allowing horizontal movement (left/right)
    Vertical,
    /// The handle is horizontal, allowing vertical movement (up/down)
    Horizontal,
}

#[derive(Debug, Clone)]
pub struct ResizeHandle {
    /// Center position of the handle in screen coordinates.
    pub center: Position,
    /// Length of the handle along its orientation axis (pixels)
    pub length: u32,
    /// Orientation of the handle (determines cursor icon & movement axis)
    pub orientation: HandleOrientation,
    /// Minimum coordinate along the drag axis that the handle is allowed to move to.
    pub min: i32,
    /// Maximum coordinate along the drag axis that the handle is allowed to move to.
    pub max: i32,
    /// ID of the container that owns this handle
    pub id: usize,
    /// Index of the child after this handle in the container's children list
    pub index: usize,
}

impl ResizeHandle {
    pub fn new(
        center: Position,
        length: u32,
        orientation: HandleOrientation,
        min: i32,
        max: i32,
        id: usize,
        index: usize,
    ) -> Self {
        Self {
            center,
            length,
            orientation,
            min,
            max,
            id,
            index,
        }
    }

    /// Clamps the provided coordinate along the drag axis (x for vertical handles, y for horizontal) into the min / max range.
    pub fn clamp_coordinate(&self, coord: i32) -> i32 {
        coord.clamp(self.min, self.max)
    }

    // bounds calculation helper for overlay preview
    pub fn preview_bounds(&self, thickness: u32) -> Bounds {
        match self.orientation {
            HandleOrientation::Vertical => Bounds::new(
                self.center.x - thickness as i32 / 2,
                self.center.y - self.length as i32 / 2,
                thickness,
                self.length,
            ),
            HandleOrientation::Horizontal => Bounds::new(
                self.center.x - self.length as i32 / 2,
                self.center.y - thickness as i32 / 2,
                self.length,
                thickness,
            ),
        }
    }
}
