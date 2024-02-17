#![allow(non_upper_case_globals)]
#![allow(unused_imports)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[cfg(target_os = "macos")]
extern crate core_foundation;

#[cfg(target_os = "macos")]
use {
    core_foundation::array::CFArrayRef, core_foundation::array::CFIndex,
    core_foundation::base::CFAllocatorRef, core_foundation::base::CFTypeID,
    core_foundation::base::CFTypeRef, core_foundation::dictionary::CFDictionaryRef,
    core_foundation::runloop::CFRunLoopSourceRef, core_foundation::string::CFStringRef,
};

#[cfg(target_os = "macos")]
pub mod accessibility_ui;
#[cfg(target_os = "macos")]
pub mod accessibility_value;

#[cfg(target_os = "macos")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
