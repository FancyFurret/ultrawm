mod config;
mod content;
mod handle;
mod manager;

pub mod overlays;

pub use config::{
    OverlayWindowBackgroundStyle, OverlayWindowBorderStyle, OverlayWindowCommand,
    OverlayWindowConfig,
};
pub use content::OverlayContent;
pub use handle::Overlay;
pub use manager::OverlayManager;

use std::sync::{Arc, OnceLock};

pub type OverlayId = u64;

static OVERLAY_MANAGER: OnceLock<Arc<OverlayManager>> = OnceLock::new();

pub fn init() -> Arc<OverlayManager> {
    OVERLAY_MANAGER
        .get_or_init(|| Arc::new(OverlayManager::new()))
        .clone()
}

pub fn manager() -> Arc<OverlayManager> {
    OVERLAY_MANAGER
        .get()
        .expect("Overlay manager not initialized. Call overlay::init() first.")
        .clone()
}
