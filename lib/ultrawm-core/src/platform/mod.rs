pub use common::*;
pub use event_bridge::*;
pub use traits::*;

mod common;
mod event_bridge;
mod thread_lock;
mod traits;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_os = "macos")] {
        mod macos;
        pub type PlatformInit = macos::MacOSPlatformInit;
        pub type Platform = macos::MacOSPlatform;
        pub type PlatformWindow = macos::MacOSPlatformWindow;
        pub type PlatformTilePreview = macos::MacOSTilePreview;
    }
}
