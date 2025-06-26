pub use common::*;
pub use event_bridge::*;
pub use traits::*;

mod common;
mod event_bridge;
pub mod inteceptor;
pub(crate) mod traits;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(test)] {
        pub mod mock;
        pub type PlatformEvents = mock::MockPlatformEvents;
        pub type Platform = mock::MockPlatform;
        pub type PlatformWindow = mock::MockPlatformWindow;
        pub type PlatformOverlay = mock::MockPlatformOverlay;
    }
    else if #[cfg(target_os = "macos")] {
        mod macos;
        pub type PlatformEvents = macos::MacOSPlatformEvents;
        pub type Platform = macos::MacOSPlatform;
        pub type PlatformWindow = macos::MacOSPlatformWindow;
        pub type PlatformOverlay = macos::MacOSPlatformOverlay;
    }
    else if #[cfg(target_os = "windows")] {
        pub mod windows;
        pub type PlatformEvents = windows::WindowsPlatformEvents;
        pub type Platform = windows::WindowsPlatform;
        pub type PlatformWindow = windows::WindowsPlatformWindow;
        pub type PlatformOverlay = windows::WindowsPlatformOverlay;
    }
    else {
        compile_error!("Unsupported platform");
    }
}
