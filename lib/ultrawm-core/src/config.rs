use once_cell::sync::Lazy;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Debug, Clone)]
pub struct Config {
    /// The number of pixels between windows
    pub window_gap: u32,
    /// The number of pixels between the edge of the partition and the windows
    pub partition_gap: u32,
    /// Whether to float windows by default when they are created
    pub float_new_windows: bool,
    /// The number of frames per second for the tile preview animation
    pub tile_preview_fps: u32,
    /// The duration of the tile preview animation in milliseconds
    pub tile_preview_animation_ms: u32,
    /// Whether to animate the tile preview fade (opacity)
    pub tile_preview_fade_animate: bool,
    /// Whether to animate the tile preview move (position/size)
    pub tile_preview_move_animate: bool,
    /// Whether to enable drag handles
    pub drag_handles: bool,
    /// The width in pixels of the invisible drag handles between tiled windows
    pub drag_handle_width: u32,
    /// Color of the drag handle highlight (RGB)
    pub drag_handle_color: (u8, u8, u8),
    /// Opacity of drag handle highlight (0.0 - 1.0)
    pub drag_handle_opacity: f32,
    /// Resizes windows as handles are dragged
    pub live_window_resize: bool,
}

static CURRENT_CONFIG: Lazy<Arc<RwLock<Config>>> =
    Lazy::new(|| Arc::new(RwLock::new(Config::default())));

impl Config {
    pub fn current() -> std::sync::RwLockReadGuard<'static, Config> {
        CURRENT_CONFIG.read().unwrap()
    }

    pub fn update<F>(f: F)
    where
        F: FnOnce(&mut Config),
    {
        if let Ok(mut config) = CURRENT_CONFIG.write() {
            f(&mut config);
        }
    }

    pub fn reset() {
        if let Ok(mut config) = CURRENT_CONFIG.write() {
            *config = Config::default();
        }
    }

    pub fn window_gap() -> u32 {
        Self::current().window_gap
    }

    pub fn partition_gap() -> u32 {
        Self::current().partition_gap
    }

    pub fn float_new_windows() -> bool {
        Self::current().float_new_windows
    }

    pub fn tile_preview_fps() -> u32 {
        Self::current().tile_preview_fps
    }

    pub fn tile_preview_animation_ms() -> u32 {
        Self::current().tile_preview_animation_ms
    }

    pub fn tile_preview_fade_animate() -> bool {
        Self::current().tile_preview_fade_animate
    }

    pub fn tile_preview_move_animate() -> bool {
        Self::current().tile_preview_move_animate
    }

    pub fn drag_handles() -> bool {
        Self::current().drag_handles
    }

    pub fn drag_handle_width() -> u32 {
        Self::current().drag_handle_width
    }

    pub fn drag_handle_color() -> (u8, u8, u8) {
        Self::current().drag_handle_color
    }

    pub fn drag_handle_opacity() -> f32 {
        Self::current().drag_handle_opacity
    }

    pub fn live_window_resize() -> bool {
        Self::current().live_window_resize
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_gap: 20,
            partition_gap: 40,
            float_new_windows: true,
            tile_preview_fps: 240,
            tile_preview_animation_ms: 150,
            tile_preview_fade_animate: true,
            tile_preview_move_animate: true,
            drag_handles: true,
            drag_handle_width: 25,
            drag_handle_color: (40, 40, 40),
            drag_handle_opacity: 0.8,
            live_window_resize: true,
        }
    }
}
