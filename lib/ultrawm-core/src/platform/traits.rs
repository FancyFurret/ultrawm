use skia_safe::Image;
use winit::window::Window;

use crate::platform::PlatformWindow;
use crate::{
    overlay_window::OverlayWindowConfig,
    platform::{
        Bounds, CursorType, Display, EventDispatcher, MouseButton, PlatformResult, Position,
        ProcessId, Size, WindowId,
    },
};

pub unsafe trait PlatformEventsImpl
where
    Self: Sized,
{
    /// Initializes the platform. This should be called once at the start of the program.
    /// This function should only be called from the main thread. It is not thread safe.
    unsafe fn initialize(dispatcher: EventDispatcher) -> PlatformResult<()>;
    unsafe fn finalize() -> PlatformResult<()>;

    fn intercept_button(button: MouseButton, intercept: bool) -> PlatformResult<()>;
}

pub trait PlatformImpl
where
    Self: Send + Sync,
{
    /// Returns a list of all windows on the system. Should only return application windows, system
    /// windows that cannot be managed should not be returned.
    fn list_visible_windows() -> PlatformResult<Vec<PlatformWindow>>;

    /// Returns a list of all monitors connected to the system.
    fn list_all_displays() -> PlatformResult<Vec<Display>>;

    /// Returns the current mouse position.
    fn get_mouse_position() -> PlatformResult<Position>;

    /// Sets the cursor to the specified type.
    fn set_cursor(cursor_type: CursorType) -> PlatformResult<()>;

    /// Resets the cursor to the system default.
    fn reset_cursor() -> PlatformResult<()>;

    fn start_window_bounds_batch(window_count: u32) -> PlatformResult<()>;
    fn end_window_bounds_batch() -> PlatformResult<()>;

    /// Simulates a mouse click at the specified position
    fn simulate_mouse_click(position: Position, button: MouseButton) -> PlatformResult<()>;
}

pub trait PlatformOverlayImpl {
    fn get_window_id(window: &Window) -> PlatformResult<WindowId>;

    fn set_window_bounds(window_id: WindowId, bounds: Bounds) -> PlatformResult<()>;

    fn set_window_opacity(window_id: WindowId, opacity: f32) -> PlatformResult<()>;

    fn render_to_window(image: &Image, window_id: WindowId) -> PlatformResult<()>;

    fn initialize_overlay_window(
        window: &Window,
        config: &OverlayWindowConfig,
    ) -> PlatformResult<()>;
}

/// Should be lightweight, and freely copyable
pub trait PlatformWindowImpl
where
    Self: Sized + Send + Sync + Clone,
{
    fn id(&self) -> WindowId;
    fn pid(&self) -> ProcessId;
    fn title(&self) -> String;
    fn position(&self) -> Position;
    fn size(&self) -> Size;
    fn visible(&self) -> bool;

    fn set_bounds(&self, bounds: &Bounds) -> PlatformResult<()>;
}
