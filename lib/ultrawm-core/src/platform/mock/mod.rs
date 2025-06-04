use crate::platform::{
    Bounds, Display, EventDispatcher, PlatformImpl, PlatformInitImpl, PlatformMainThreadImpl,
    PlatformResult, PlatformTilePreviewImpl, PlatformWindow, PlatformWindowImpl, Position,
    ProcessId, Size, WindowId,
};

pub struct MockPlatformInit;
unsafe impl PlatformInitImpl for MockPlatformInit {
    unsafe fn initialize() -> PlatformResult<()> {
        return Ok(());
    }
    unsafe fn run_event_loop(dispatcher: EventDispatcher) -> PlatformResult<()> {
        return Ok(());
    }
}

pub struct MockPlatform;
impl PlatformImpl for MockPlatform {
    fn list_visible_windows() -> PlatformResult<Vec<PlatformWindow>> {
        return Ok(vec![]);
    }

    fn list_all_displays() -> PlatformResult<Vec<Display>> {
        return Ok(vec![]);
    }

    fn get_mouse_position() -> PlatformResult<Position> {
        return Ok(Position { x: 0, y: 0 });
    }
}

pub struct MockMainThread;
impl PlatformMainThreadImpl for MockMainThread {
    fn is_main_thread() -> bool {
        return true;
    }

    fn run_on_main_thread<F, R>(f: F) -> PlatformResult<R>
    where
        F: FnOnce() -> R + Send,
        R: Send + 'static,
    {
        return Ok(f());
    }
}

pub struct MockPlatformTilePreview;
impl PlatformTilePreviewImpl for MockPlatformTilePreview {
    fn new() -> PlatformResult<Self> {
        return Ok(Self {});
    }
    fn show(&mut self) -> PlatformResult<()> {
        return Ok(());
    }
    fn hide(&mut self) -> PlatformResult<()> {
        return Ok(());
    }
    fn move_to(&mut self, _bounds: &Bounds) -> PlatformResult<()> {
        return Ok(());
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
}
impl MockPlatformWindow {
    pub fn new(position: Position, size: Size, title: String) -> Self {
        return Self {
            id: 0,
            pid: 0,
            title,
            position,
            size,
            visible: false,
        };
    }
}
impl PlatformWindowImpl for MockPlatformWindow {
    fn id(&self) -> WindowId {
        return self.id;
    }
    fn pid(&self) -> ProcessId {
        return self.pid;
    }
    fn title(&self) -> String {
        return self.title.clone();
    }
    fn position(&self) -> Position {
        return self.position.clone();
    }
    fn size(&self) -> Size {
        return self.size.clone();
    }
    fn visible(&self) -> bool {
        return self.visible;
    }
    fn set_bounds(&self, bounds: &Bounds) -> PlatformResult<()> {
        return Ok(());
    }
}
