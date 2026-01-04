use crate::overlay::OverlayWindowConfig;
use crate::platform::PlatformOverlayImpl;
use crate::platform::{
    Bounds, Display, EventDispatcher, PlatformEventsImpl, PlatformImpl, PlatformResult,
    PlatformWindow, PlatformWindowImpl, Position, ProcessId, Size, WindowId,
};
use skia_safe::Image;
use std::sync::{Arc, Mutex};
use winit::window::Window;

pub struct MockPlatformEvents;
unsafe impl PlatformEventsImpl for MockPlatformEvents {
    unsafe fn initialize(_dispatcher: EventDispatcher) -> PlatformResult<()> {
        Ok(())
    }
    unsafe fn finalize() -> PlatformResult<()> {
        Ok(())
    }
}

pub struct MockPlatform;
impl PlatformImpl for MockPlatform {
    fn list_visible_windows() -> PlatformResult<Vec<PlatformWindow>> {
        Ok(vec![])
    }

    fn list_all_displays() -> PlatformResult<Vec<Display>> {
        Ok(vec![])
    }

    fn get_mouse_position() -> PlatformResult<Position> {
        Ok(Position { x: 0, y: 0 })
    }

    fn set_cursor(_cursor_type: crate::platform::CursorType) -> PlatformResult<()> {
        Ok(())
    }

    fn reset_cursor() -> PlatformResult<()> {
        Ok(())
    }

    fn start_window_bounds_batch(_window_count: u32) -> PlatformResult<()> {
        Ok(())
    }

    fn end_window_bounds_batch() -> PlatformResult<()> {
        Ok(())
    }

    fn simulate_mouse_click(
        _position: Position,
        _button: crate::platform::MouseButton,
    ) -> PlatformResult<()> {
        Ok(())
    }
}

pub struct MockPlatformOverlay;
impl PlatformOverlayImpl for MockPlatformOverlay {
    fn get_window_id(_window: &Window) -> PlatformResult<WindowId> {
        Ok(1)
    }
    fn set_window_bounds(_window_id: WindowId, _bounds: Bounds) -> PlatformResult<()> {
        Ok(())
    }
    fn set_window_opacity(_window_id: WindowId, _opacity: f32) -> PlatformResult<()> {
        Ok(())
    }
    fn render_to_window(_image: &Image, _window_id: WindowId) -> PlatformResult<()> {
        Ok(())
    }

    fn initialize_overlay_window(
        _window: &Window,
        _config: &OverlayWindowConfig,
    ) -> PlatformResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MockPlatformWindow {
    pub id: WindowId,
    pub pid: ProcessId,
    pub title: String,
    pub position: Position,
    pub size: Size,
    pub visible: bool,
    set_bounds_calls: Arc<Mutex<Vec<Bounds>>>,
}
impl MockPlatformWindow {
    pub fn new(position: Position, size: Size, title: String) -> Self {
        Self {
            id: 0,
            pid: 0,
            title,
            position,
            size,
            visible: false,
            set_bounds_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_set_bounds_calls(&self) -> Vec<Bounds> {
        self.set_bounds_calls.lock().unwrap().clone()
    }

    pub fn clear_set_bounds_calls(&self) {
        self.set_bounds_calls.lock().unwrap().clear();
    }
}
impl PlatformWindowImpl for MockPlatformWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn pid(&self) -> ProcessId {
        self.pid
    }
    fn title(&self) -> String {
        self.title.clone()
    }
    fn position(&self) -> Position {
        self.position.clone()
    }
    fn size(&self) -> Size {
        self.size.clone()
    }
    fn visible(&self) -> bool {
        self.visible
    }
    fn set_bounds(&self, bounds: &Bounds) -> PlatformResult<()> {
        self.set_bounds_calls.lock().unwrap().push(bounds.clone());
        Ok(())
    }
    fn focus(&self) -> PlatformResult<()> {
        Ok(())
    }
    fn set_always_on_top(&self, _always_on_top: bool) -> PlatformResult<()> {
        Ok(())
    }
    fn close(&self) -> PlatformResult<()> {
        Ok(())
    }
    fn minimize(&self) -> PlatformResult<()> {
        Ok(())
    }
    fn valid(&self) -> bool {
        // Mock windows are always valid
        true
    }
}
