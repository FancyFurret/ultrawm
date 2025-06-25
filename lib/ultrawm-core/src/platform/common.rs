use crate::platform::PlatformWindow;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;
use winit::keyboard::KeyCode;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("Unknown platform error")]
    Unknown,

    #[error("{0}")]
    Error(String),
}

impl From<&str> for PlatformError {
    fn from(error: &str) -> Self {
        PlatformError::Error(error.to_string()).into()
    }
}

impl From<String> for PlatformError {
    fn from(error: String) -> Self {
        PlatformError::Error(error).into()
    }
}

impl From<()> for PlatformError {
    fn from(_: ()) -> Self {
        PlatformError::Unknown.into()
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
    KeyDown(KeyCode),
    KeyUp(KeyCode),
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Default)]
pub struct MouseButtons {
    buttons: HashSet<MouseButton>,
}

impl MouseButtons {
    pub fn new() -> Self {
        Self {
            buttons: HashSet::new(),
        }
    }

    pub fn any(&self) -> bool {
        self.buttons.len() > 0
    }

    pub fn contains(&self, button: &MouseButton) -> bool {
        self.buttons.contains(button)
    }

    pub fn contains_all(&self, other: &MouseButtons) -> bool {
        other.buttons.iter().all(|button| self.contains(button))
            && self.buttons.len() == other.buttons.len()
    }

    pub fn add(&mut self, button: &MouseButton) {
        self.buttons.insert(button.clone());
    }

    pub fn remove(&mut self, button: &MouseButton) {
        self.buttons.remove(button);
    }

    pub fn update_button(&mut self, button: &MouseButton, pressed: bool) {
        if pressed {
            self.buttons.insert(button.clone());
        } else {
            self.buttons.remove(button);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Keys {
    keys: HashSet<KeyCode>,
}

impl Keys {
    pub fn new() -> Self {
        Self {
            keys: HashSet::new(),
        }
    }

    pub fn contains(&self, key: &KeyCode) -> bool {
        self.keys.contains(key)
    }

    pub fn contains_all(&self, other: &Keys) -> bool {
        other.keys.iter().all(|key| self.contains(key)) && self.keys.len() == other.keys.len()
    }

    pub fn add(&mut self, key: &KeyCode) {
        self.keys.insert(key.clone());
    }

    pub fn remove(&mut self, key: &KeyCode) {
        self.keys.remove(key);
    }

    pub fn update_key(&mut self, key: &KeyCode, pressed: bool) {
        if pressed {
            self.keys.insert(key.clone());
        } else {
            self.keys.remove(key);
        }
    }
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

    pub fn offset_top(&mut self, offset: i32) {
        self.position.y += offset;
        self.size.height = (self.size.height as i32 - offset) as u32;
    }

    pub fn offset_bottom(&mut self, offset: i32) {
        self.size.height = (self.size.height as i32 + offset) as u32;
    }

    pub fn offset_left(&mut self, offset: i32) {
        self.position.x += offset;
        self.size.width = (self.size.width as i32 - offset) as u32;
    }

    pub fn offset_right(&mut self, offset: i32) {
        self.size.width = (self.size.width as i32 + offset) as u32;
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
