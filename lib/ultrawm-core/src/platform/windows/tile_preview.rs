use crate::platform::{Bounds, PlatformResult, PlatformTilePreviewImpl};

pub struct WindowsTilePreview {

}

impl PlatformTilePreviewImpl for WindowsTilePreview {
    fn new() -> PlatformResult<Self> {
        todo!()
    }

    fn show(&mut self) -> PlatformResult<()> {
        todo!()
    }

    fn hide(&mut self) -> PlatformResult<()> {
        todo!()
    }

    fn move_to(&mut self, bounds: &Bounds) -> PlatformResult<()> {
        todo!()
    }
}