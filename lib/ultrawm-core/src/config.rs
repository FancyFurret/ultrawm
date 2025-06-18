use log::{trace, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Whether to save your layout and load it on startup
    pub persistence: bool,
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
    pub fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("UltraWM").join("config.yaml"))
    }

    pub fn load(config_path: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let path = match config_path {
            Some(p) => PathBuf::from(p),
            None => {
                Self::default_config_path().ok_or("Could not determine default config directory")?
            }
        };

        if !path.exists() {
            Self::create_default_config_file(&path)?;
            trace!("Created default config file at: {}", path.display());
        }

        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read config file '{}': {}", path.display(), e))?;

        let config: Config = serde_yaml::from_str(&contents)
            .map_err(|e| format!("Failed to parse config file '{}': {}", path.display(), e))?;

        // Save the config back to ensure all fields are present (fills in any missing fields with defaults)
        if let Err(e) = config.save_to_file(&path) {
            warn!("Failed to update config file with missing fields: {e}");
        }

        Ok(config)
    }

    fn create_default_config_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let default_config = Config::default();
        default_config.save_to_file(path)?;
        Ok(())
    }

    pub fn set_config(config: Config) {
        if let Ok(mut global_config) = CURRENT_CONFIG.write() {
            *global_config = config;
        }
    }

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

    pub fn persistence() -> bool {
        Self::current().persistence
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

    /// Save the current config to a file
    pub fn save_to_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let header =
            "# UltraWM Configuration File\n# This file contains your UltraWM settings.\n\n";
        let serialized_config = serde_yaml::to_string(self)?;
        let config_content = format!("{}{}", header, serialized_config);

        fs::write(path, config_content)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            persistence: true,
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
