use std::rc::Rc;

pub type ConfigRef = Rc<Config>;

#[derive(Debug)]
pub struct Config {
    /// The number of pixels between windows
    pub window_gap: u32,
    /// The number of pixels between the edge of the partition and the windows
    pub partition_gap: u32,
    /// Whether to float windows by default when they are created
    pub float_new_windows: bool,
    /// The number of frames per second for the tile preview animation
    pub tile_preview_fps: u32,
    /// Whether to animate the tile preview
    pub tile_preview_animate: bool,
    /// The duration of the tile preview animation in milliseconds
    pub tile_preview_animation_duration: u32,
    /// Whether to animate the tile preview fade (opacity)
    pub tile_preview_fade_animate: bool,
    /// Whether to animate the tile preview move (position/size)
    pub tile_preview_move_animate: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_gap: 20,
            partition_gap: 40,
            float_new_windows: true,
            tile_preview_fps: 500,
            tile_preview_animate: true,
            tile_preview_animation_duration: 150,
            tile_preview_fade_animate: true,
            tile_preview_move_animate: true,
        }
    }
}
