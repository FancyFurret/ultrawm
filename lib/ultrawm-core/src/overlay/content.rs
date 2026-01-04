use crate::overlay::OverlayWindowConfig;
use crate::platform::{Bounds, PlatformResult};
use skia_safe::Canvas;

pub trait OverlayContent: Send + 'static {
    fn config(&self) -> OverlayWindowConfig;

    fn draw(&mut self, canvas: &Canvas, bounds: &Bounds) -> PlatformResult<()>;

    fn on_show(&mut self) {}

    fn on_hide(&mut self) {}

    fn on_bounds_changed(&mut self, _bounds: &Bounds) {}
}
