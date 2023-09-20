use crate::platform::{EventDispatcher, PlatformWindow};
use std::backtrace::Backtrace;
use std::fmt::Debug;

pub trait PlatformInterface
where
    Self: Sized,
{
    /// Returns a list of all windows on the system. Should only return application windows, system
    /// windows that cannot managed should not be returned.
    fn list_all_windows() -> PlatformResult<Vec<PlatformWindow>>;

    /// This function should block. Events should be sent via the provided dispatcher.
    /// Only one event loop will be requested at a time. Window events should only be sent for
    /// windows that can be managed.
    fn run_event_loop(dispatcher: EventDispatcher) -> PlatformResult<()>;
}

/// Should be lightweight, and freely copyable
pub trait PlatformWindowInterface: Clone {
    fn id(&self) -> WindowId;
    fn pid(&self) -> ProcessId;
    fn title(&self) -> PlatformResult<String>;
    fn position(&self) -> PlatformResult<Position>;
    fn size(&self) -> PlatformResult<Size>;
    fn visible(&self) -> PlatformResult<bool>;

    fn move_to(&self, x: u32, y: u32) -> PlatformResult<()>;
    fn resize(&self, width: u32, height: u32) -> PlatformResult<()>;
}

#[derive(Debug)]
pub struct PlatformError {
    pub error_type: PlatformErrorType,
    pub backtrace: Backtrace,
}

#[derive(Debug)]
pub enum PlatformErrorType {
    Unknown,
    Error(String),
}

impl From<PlatformErrorType> for PlatformError {
    fn from(error_type: PlatformErrorType) -> Self {
        Self {
            error_type,
            backtrace: Backtrace::capture(),
        }
    }
}

impl From<&str> for PlatformError {
    fn from(error: &str) -> Self {
        PlatformErrorType::Error(error.to_string()).into()
    }
}

impl From<String> for PlatformError {
    fn from(error: String) -> Self {
        PlatformErrorType::Error(error).into()
    }
}

impl From<()> for PlatformError {
    fn from(_: ()) -> Self {
        PlatformErrorType::Unknown.into()
    }
}

pub type PlatformResult<T> = Result<T, PlatformError>;

#[derive(Debug)]
pub enum PlatformEvent {
    WindowCreated(PlatformWindow),
    WindowDestroyed(WindowId),
    WindowFocused(PlatformWindow),
    WindowMoved(PlatformWindow),
    WindowResized(PlatformWindow),
    WindowShown(PlatformWindow),
    WindowHidden(PlatformWindow),
}

impl PlatformEvent {
    pub fn window(&self) -> Option<&PlatformWindow> {
        match self {
            PlatformEvent::WindowCreated(window)
            | PlatformEvent::WindowFocused(window)
            | PlatformEvent::WindowMoved(window)
            | PlatformEvent::WindowResized(window)
            | PlatformEvent::WindowShown(window)
            | PlatformEvent::WindowHidden(window) => Some(window),
            _ => None,
        }
    }
}

pub type ProcessId = u32;
pub type WindowId = u32;

#[derive(Debug)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct Position {
    pub x: u32,
    pub y: u32,
}
