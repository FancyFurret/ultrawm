#![allow(non_upper_case_globals)]
#![allow(unused_imports)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

extern crate core_foundation;

use core_foundation::array::CFArrayRef;
use core_foundation::array::CFIndex;
use core_foundation::base::CFAllocatorRef;
use core_foundation::base::CFTypeID;
use core_foundation::base::CFTypeRef;
use core_foundation::dictionary::CFDictionaryRef;
use core_foundation::runloop::CFRunLoopSourceRef;
use core_foundation::string::CFStringRef;

pub mod accessibility_ui;
pub mod accessibility_value;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
