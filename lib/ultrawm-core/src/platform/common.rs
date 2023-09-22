use crate::platform::PlatformWindow;
use std::backtrace::Backtrace;
use std::fmt::Debug;

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
