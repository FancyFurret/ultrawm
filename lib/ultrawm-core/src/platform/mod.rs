pub use common::*;
pub use event_bridge::*;

mod common;
mod event_bridge;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_os = "macos")] {
        mod macos;
        pub type Platform = macos::MacOSPlatform;
        pub type PlatformWindow = macos::MacOSPlatformWindow;
    }
}
