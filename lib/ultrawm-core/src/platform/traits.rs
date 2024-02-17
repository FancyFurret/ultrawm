use crate::platform::{
    Bounds, Display, EventDispatcher, PlatformResult, Position, ProcessId, Size, WindowId,
};

/// # Safety
/// These functions should only be called from the main thread. They are not thread safe.
pub unsafe trait PlatformInitImpl
where
    Self: Sized,
{
    /// Initializes the platform. This should be called once at the start of the program.
    unsafe fn initialize() -> PlatformResult<()>;

    /// This function should block. Events should be sent via the provided dispatcher.
    /// Only one event loop should be requested at a time. Window events should only be sent for
    /// windows that can be managed.
    unsafe fn run_event_loop(dispatcher: EventDispatcher) -> PlatformResult<()>;
}

pub trait PlatformImpl
where
    Self: Send + Sync,
{
    /// Returns a list of all windows on the system. Should only return application windows, system
    /// windows that cannot be managed should not be returned.
    fn list_visible_windows() -> PlatformResult<Vec<crate::platform::PlatformWindow>>;

    /// Returns a list of all monitors connected to the system.
    fn list_all_displays() -> PlatformResult<Vec<Display>>;

    /// Returns the current mouse position.
    fn get_mouse_position() -> PlatformResult<Position>;
}

pub trait PlatformTilePreviewImpl
where
    Self: Sized + Send + Sync,
{
    /// Creates a new tile preview. Should not be shown until `show` is called.
    fn new() -> PlatformResult<Self>;
    fn show(&mut self) -> PlatformResult<()>;
    fn hide(&mut self) -> PlatformResult<()>;
    fn move_to(&mut self, bounds: &Bounds) -> PlatformResult<()>;
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

    fn set_bounds(&mut self, bounds: &Bounds) -> PlatformResult<()>;
}

/// Optional, this trait only needs to be implemented if main thread utils like MainThreadLock
/// are needed by the platform implementation.
pub trait PlatformMainThreadImpl
where
    Self: Sized + Send + Sync,
{
    fn is_main_thread() -> bool;

    /// Runs the provided function on the main thread. Should only be called once the platform's event
    /// loop has started.
    fn run_on_main_thread<F, R>(f: F) -> PlatformResult<R>
    where
        F: FnOnce() -> R + Send,
        R: Send + 'static;
}
