use crate::overlay::content::OverlayContent;
use crate::overlay::manager::OverlayManager;
use crate::overlay::OverlayId;
use crate::overlay::OverlayWindowCommand;
use crate::platform::Bounds;
use std::sync::Arc;

/// Handle to an overlay window - provides ergonomic API
pub struct Overlay {
    id: OverlayId,
    manager: Arc<OverlayManager>,
}

impl Overlay {
    /// Create a new overlay handle (use OverlayManager::add() instead)
    pub(crate) fn new(id: OverlayId, manager: Arc<OverlayManager>) -> Self {
        Self { id, manager }
    }

    /// Show the overlay
    pub fn show(&self) {
        self.manager
            .send_command(self.id, OverlayWindowCommand::Show);
    }

    /// Hide the overlay
    pub fn hide(&self) {
        self.manager
            .send_command(self.id, OverlayWindowCommand::Hide);
    }

    /// Move the overlay to new bounds
    pub fn move_to(&self, bounds: &Bounds) {
        self.manager
            .send_command(self.id, OverlayWindowCommand::MoveTo(bounds.clone()));
    }

    /// Update the overlay content
    pub fn update_content<F>(&self, f: F)
    where
        F: FnOnce(&mut dyn OverlayContent) + Send + 'static,
    {
        self.manager.update_content(self.id, f);
    }

    /// Get the overlay ID
    pub fn id(&self) -> OverlayId {
        self.id
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        self.manager.remove_overlay(self.id);
    }
}
