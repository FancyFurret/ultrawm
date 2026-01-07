use crate::platform::{Bounds, Position};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HandleOrientation {
    /// The handle is vertical, allowing horizontal movement (left/right)
    Vertical,
    /// The handle is horizontal, allowing vertical movement (up/down)
    Horizontal,
}

pub enum ResizeMode {
    Evenly,
    Before,
    After,
    BeforeSymmetric,
    AfterSymmetric,
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
    /// ID of the element before (left/top of) this handle
    pub before_id: u64,
    /// ID of the element after (right/bottom of) this handle
    pub after_id: u64,
}

impl ResizeHandle {
    pub fn new(
        center: Position,
        length: u32,
        orientation: HandleOrientation,
        min: i32,
        max: i32,
        before_id: u64,
        after_id: u64,
    ) -> Self {
        Self {
            center,
            length,
            orientation,
            min,
            max,
            before_id,
            after_id,
        }
    }

    /// Clamps the provided coordinate along the drag axis (x for vertical handles, y for horizontal) into the min / max range.
    pub fn clamp_coordinate(&self, coord: i32) -> i32 {
        coord.clamp(self.min, self.max)
    }

    /// Returns the bounds for a visual preview of this handle.
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

    /// Check if a position is within the handle's hit area.
    pub fn contains(&self, position: &Position, thickness: i32) -> bool {
        match self.orientation {
            HandleOrientation::Vertical => {
                let dx = (position.x - self.center.x).abs();
                let dy = (position.y - self.center.y).abs();
                dx <= thickness / 2 && dy <= self.length as i32 / 2
            }
            HandleOrientation::Horizontal => {
                let dx = (position.x - self.center.x).abs();
                let dy = (position.y - self.center.y).abs();
                dy <= thickness / 2 && dx <= self.length as i32 / 2
            }
        }
    }
}
