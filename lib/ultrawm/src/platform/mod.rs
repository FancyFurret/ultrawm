use crate::platform::common::{Position, Size};
use cfg_if::cfg_if;

pub mod common;

cfg_if! {
    if #[cfg(target_os = "macos")] {
        mod macos;
        pub type Platform = macos::MacOSPlatform;
        pub type PlatformWindow = macos::MacOSPlatformWindow;
    }
}

#[derive(Debug)]
pub enum PlatformError {
    Unknown,
    Error(&'static str),
}

pub type PlatformResult<T> = Result<T, PlatformError>;

pub trait PlatformTrait {
    fn new() -> Self
    where
        Self: Sized;

    /// Returns a list of all windows on the system.
    /// Should only return application windows, system windows that cannot managed should not be returned.
    fn list_all_windows(&self) -> PlatformResult<Vec<PlatformWindow>>;
}

pub trait PlatformWindowTrait: Clone {
    fn id(&self) -> PlatformResult<u32>;
    fn pid(&self) -> PlatformResult<u32>;
    fn title(&self) -> PlatformResult<String>;
    fn position(&self) -> PlatformResult<Position>;
    fn size(&self) -> PlatformResult<Size>;
    fn visible(&self) -> PlatformResult<bool>;

    fn move_to(&self, x: u32, y: u32) -> PlatformResult<()>;
    fn resize(&self, width: u32, height: u32) -> PlatformResult<()>;
}

pub fn create_platform() -> Platform {
    Platform::new()
}
