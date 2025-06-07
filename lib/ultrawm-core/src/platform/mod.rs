pub use common::*;
pub use event_bridge::*;
pub use traits::*;

mod common;
mod event_bridge;
mod traits;

// TODO: Use features instead?
#[cfg(target_os = "macos")]
mod thread_lock;

use cfg_if::cfg_if;

pub mod animation;

cfg_if! {
    if #[cfg(test)] {
        pub mod mock;
        pub type PlatformInit = mock::MockPlatformInit;
        pub type Platform = mock::MockPlatform;
        pub type PlatformWindow = mock::MockPlatformWindow;
        pub type PlatformTilePreview = mock::MockPlatformTilePreview;
        pub type PlatformMainThread = mock::MockMainThread;
    }
    else if #[cfg(target_os = "macos")] {
        mod macos;
        pub type PlatformInit = macos::MacOSPlatformInit;
        pub type Platform = macos::MacOSPlatform;
        pub type PlatformWindow = macos::MacOSPlatformWindow; // TODO: Remove Platform from name?
        pub type PlatformTilePreview = macos::MacOSTilePreview;
        pub type PlatformMainThread = macos::MacOSMainThread;
    }
    else if #[cfg(target_os = "windows")] {
        pub mod windows;
        pub type PlatformInit = windows::WindowsPlatformInit;
        pub type Platform = windows::WindowsPlatform;
        pub type PlatformWindow = windows::WindowsPlatformWindow;
        pub type PlatformTilePreview = windows::WindowsTilePreview;
    }
    else {
        compile_error!("Unsupported platform");
    }
}
