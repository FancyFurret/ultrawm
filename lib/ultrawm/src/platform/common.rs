// #[cfg(target_os = "macos")]
// use crate::platform::macos::WindowPlatformData;

// #[derive(Debug)]
// pub struct Window {
//     pub id: u32,
//     pub pid: u32,
//     pub title: String,
//     pub x: u32,
//     pub y: u32,
//     pub width: u32,
//     pub height: u32,
//     pub visible: bool,
//     pub platform_data: WindowPlatformData,
// }

pub struct Size {
    pub width: u32,
    pub height: u32,
}

pub struct Position {
    pub x: u32,
    pub y: u32,
}
