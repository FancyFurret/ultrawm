use crate::platform::PlatformWindow;
use serde::{Deserialize, Serialize};
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
    /// A new window has been opened. *If needed*, can also be sent when a window is shown after
    /// being hidden.
    WindowOpened(PlatformWindow),
    WindowClosed(WindowId),
    WindowShown(WindowId),
    WindowHidden(WindowId),
    WindowFocused(WindowId),
    /// The window has begun to be moved or resized. Preferably only sent once per window
    /// transformation, but may be sent multiple times. Extra events will be ignored.
    WindowTransformStarted(WindowId),
    MouseDown(Position, MouseButton),
    MouseUp(Position, MouseButton),
    MouseMoved(Position),
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub type DisplayId = u32;
pub type ProcessId = u32;
pub type WindowId = u64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Bounds {
    pub position: Position,
    pub size: Size,
}

impl Bounds {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            position: Position::new(x, y),
            size: Size::new(width, height),
        }
    }

    pub fn from_position(position: Position, size: Size) -> Self {
        Self { position, size }
    }

    pub fn center(&self) -> Position {
        Position::new(
            self.position.x + self.size.width as i32 / 2,
            self.position.y + self.size.height as i32 / 2,
        )
    }

    pub fn contains(&self, position: &Position) -> bool {
        position.x >= self.position.x
            && position.x < self.position.x + self.size.width as i32
            && position.y >= self.position.y
            && position.y < self.position.y + self.size.height as i32
    }

    pub fn intersects(&self, other: &Bounds) -> bool {
        self.position.x < other.position.x + other.size.width as i32
            && self.position.x + self.size.width as i32 > other.position.x
            && self.position.y < other.position.y + other.size.height as i32
            && self.position.y + self.size.height as i32 > other.position.y
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug)]
pub struct Display {
    pub id: DisplayId,
    pub name: String,
    pub bounds: Bounds,
    pub work_area: Bounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorType {
    Normal,
    ResizeNorth,
    ResizeSouth,
    ResizeEast,
    ResizeWest,
    ResizeNorthEast,
    ResizeNorthWest,
    ResizeSouthEast,
    ResizeSouthWest,
    Move,
    IBeam,
    Wait,
    NotAllowed,
}
