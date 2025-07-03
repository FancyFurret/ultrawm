use crate::config::config_serializer::serialize_config;
use crate::config::{ModMouseKeybind, MouseKeybind};
use log::{trace, warn};
use once_cell::sync::Lazy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Config {
    /// The path the config file was loaded from
    #[serde(skip)]
    pub config_path: Option<PathBuf>,

    /// Save and restore your window layout when UltraWM starts
    pub persistence: bool,
    /// Space between windows in pixels (set to 0 for no gaps)
    pub window_gap: u32,
    /// Space between screen edges and windows in pixels
    pub partition_gap: u32,
    /// New windows start as floating instead of automatically tiling
    pub float_new_windows: bool,
    /// Automatically focus windows when your mouse hovers over them
    pub focus_on_hover: bool,
    /// The number of frames per second for the tile preview animation
    pub tile_preview_fps: u32,
    /// How long tile preview animations take in milliseconds
    pub tile_preview_animation_ms: u32,
    /// Enable fade in/out effects for tile previews
    pub tile_preview_fade_animate: bool,
    /// Enable movement animations for tile previews
    pub tile_preview_move_animate: bool,
    /// Enable animations when tiling windows
    pub window_tile_animate: bool,
    /// How long window tiling animations take in milliseconds
    pub window_tile_animation_ms: u32,
    /// The number of frames per second for window tiling animations
    pub window_tile_fps: u32,
    /// Show transparent resize handles between tiled windows for easy resizing
    pub resize_handles: bool,
    /// Width of the transparent resize handles in pixels
    pub resize_handle_width: u32,
    /// Color of the resize handles (red, green, blue from 0-255)
    pub resize_handle_color: (u8, u8, u8),
    /// Opacity of drag handle highlight (0.0 - 1.0)
    pub resize_handle_opacity: f32,
    /// Update window sizes in real-time while dragging handles
    pub live_window_resize: bool,
    /// Mouse controls for resize handles
    pub resize_handle_bindings: ResizeHandleBindings,
    /// Mouse controls for moving and resizing windows with a modifier key
    pub mod_transform_bindings: ModTransformBindings,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ResizeHandleBindings {
    /// Resize the window on the left or top side of the handle
    pub resize_before: MouseKeybind,
    /// Resize the window on the right or bottom side of the handle
    pub resize_after: MouseKeybind,
    /// Resize both sides equally
    pub resize_evenly: MouseKeybind,
    /// Equally resize the sides of the left/top window
    pub resize_before_symmetric: MouseKeybind,
    /// Equally resize the sides of the right/bottom window
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ModTransformBindings {
    /// Move the window into or around the tiled layout
    pub tile: ModMouseKeybind,
    /// Make the window move freely without tiling
    pub float: ModMouseKeybind,
    /// Move the window around without changing its tiled/floating state
    pub shift: ModMouseKeybind,
    /// Switch between tiled and floating modes
    pub toggle: ModMouseKeybind,
    /// Resize the window from the corner or edge you're dragging
    pub resize: ModMouseKeybind,
    /// Equally resize the sides of the window
    pub resize_symmetric: ModMouseKeybind,
}

impl Default for ModTransformBindings {
    fn default() -> Self {
        Self {
            tile: vec![].into(),
            float: vec![].into(),
            shift: vec!["ctrl+lmb", "bmb+lmb"].into(),
            toggle: vec!["ctrl+lmb+rmb", "bmb+lmb+rmb", "fmb+lmb"].into(),
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

    pub fn window_tile_animate() -> bool {
        Self::current().window_tile_animate
    }

    pub fn window_tile_animation_ms() -> u32 {
        Self::current().window_tile_animation_ms
    }

    pub fn window_tile_fps() -> u32 {
        Self::current().window_tile_fps
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

        serialize_config(self, path.to_str().unwrap())?;
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
            window_tile_animate: true,
            window_tile_animation_ms: 150,
            window_tile_fps: 240,
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
