use crate::config::{ModMouseKeybind, MouseKeybind};
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
    /// The path the config file was loaded from
    #[serde(skip)]
    pub config_path: Option<PathBuf>,

    /// Whether to save your layout and load it on startup
    pub persistence: bool,
    /// The number of pixels between windows
    pub window_gap: u32,
    /// The number of pixels between the edge of the partition and the windows
    pub partition_gap: u32,
    /// Whether to float windows by default when they are created
    pub float_new_windows: bool,
    /// Whether to focus windows when the mouse hovers over them
    pub focus_on_hover: bool,
    /// The number of frames per second for the tile preview animation
    pub tile_preview_fps: u32,
    /// The duration of the tile preview animation in milliseconds
    pub tile_preview_animation_ms: u32,
    /// Whether to animate the tile preview fade (opacity)
    pub tile_preview_fade_animate: bool,
    /// Whether to animate the tile preview move (position/size)
    pub tile_preview_move_animate: bool,
    /// Whether to enable drag handles
    pub resize_handles: bool,
    /// The width in pixels of the invisible drag handles between tiled windows
    pub resize_handle_width: u32,
    /// Color of the drag handle highlight (RGB)
    pub resize_handle_color: (u8, u8, u8),
    /// Opacity of drag handle highlight (0.0 - 1.0)
    pub resize_handle_opacity: f32,
    /// Resizes windows as handles are dragged
    pub live_window_resize: bool,
    /// Bindings handle resize actions
    pub resize_handle_bindings: ResizeHandleBindings,
    /// Bindings for window area actions (tile, slide, etc.)
    pub mod_transform_bindings: ModTransformBindings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ResizeHandleBindings {
    /// Bindings for resizing the left or top window (e.g., LMB)
    pub resize_before: MouseKeybind,
    /// Bindings for resizing the right or bottom window (e.g., RMB)
    pub resize_after: MouseKeybind,
    /// Bindings for resizing both sides evenly (e.g., MMB, LMB+RMB)
    pub resize_evenly: MouseKeybind,
    /// Bindings for symmetric resize of left/top (e.g., Shift+LMB)
    pub resize_before_symmetric: MouseKeybind,
    /// Bindings for symmetric resize of right/bottom (e.g., Shift+RMB)
    pub resize_after_symmetric: MouseKeybind,
}

impl Default for ResizeHandleBindings {
    fn default() -> Self {
        Self {
            resize_before: vec!["lmb"].into(),
            resize_after: vec!["rmb"].into(),
            resize_evenly: vec!["mmb"].into(),
            resize_before_symmetric: vec!["lmb+mmb"].into(),
            resize_after_symmetric: vec!["rmb+mmb"].into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModTransformBindings {
    pub tile: ModMouseKeybind,
    pub float: ModMouseKeybind,
    pub resize: ModMouseKeybind,
    pub resize_symmetric: ModMouseKeybind,
}

impl Default for ModTransformBindings {
    fn default() -> Self {
        Self {
            tile: vec!["ctrl+lmb", "bmb+lmb"].into(),
            float: vec!["ctrl+lmb+rmb", "bmb+lmb+rmb", "fmb+lmb"].into(),
            resize: vec!["ctrl+rmb", "bmb+rmb"].into(),
            resize_symmetric: vec!["ctrl+mmb", "bmb+mmb"].into(),
        }
    }
}

static CURRENT_CONFIG: Lazy<Arc<RwLock<Config>>> =
    Lazy::new(|| Arc::new(RwLock::new(Config::default())));

impl Config {
    pub fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("UltraWM").join("config.yaml"))
    }

    pub fn load(config_path: Option<&str>, save: bool) -> Result<Self, Box<dyn std::error::Error>> {
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

        let mut config: Config = serde_yaml::from_str(&contents)
            .map_err(|e| format!("Failed to parse config file '{}': {}", path.display(), e))?;

        config.config_path = Some(path.clone());

        // Save the config back to ensure all fields are present (fills in any missing fields with defaults)
        if save {
            if let Err(e) = config.save_to_file(&path.clone()) {
                warn!("Failed to update config file with missing fields: {e}");
            }
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

    pub fn focus_on_hover() -> bool {
        Self::current().focus_on_hover
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

    pub fn resize_handles() -> bool {
        Self::current().resize_handles
    }

    pub fn resize_handle_width() -> u32 {
        Self::current().resize_handle_width
    }

    pub fn resize_handle_color() -> (u8, u8, u8) {
        Self::current().resize_handle_color
    }

    pub fn resize_handle_opacity() -> f32 {
        Self::current().resize_handle_opacity
    }

    pub fn live_window_resize() -> bool {
        Self::current().live_window_resize
    }

    pub fn get_window_area_bindings(&self) -> &ModTransformBindings {
        &self.mod_transform_bindings
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
            config_path: None,
            persistence: true,
            window_gap: 20,
            partition_gap: 40,
            float_new_windows: true,
            focus_on_hover: true,
            tile_preview_fps: 240,
            tile_preview_animation_ms: 150,
            tile_preview_fade_animate: true,
            tile_preview_move_animate: true,
            resize_handles: true,
            resize_handle_width: 25,
            resize_handle_color: (40, 40, 40),
            resize_handle_opacity: 0.8,
            live_window_resize: true,
            resize_handle_bindings: ResizeHandleBindings::default(),
            mod_transform_bindings: ModTransformBindings::default(),
        }
    }
}
