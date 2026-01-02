use std::path::PathBuf;

/// Get the base directory for UltraWM data files
fn data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join("UltraWM"))
}

/// Get the base directory for UltraWM config files
fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("UltraWM"))
}

/// Get the path to the log file
pub fn log_file_path() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("ultrawm.log"))
}

/// Get the path to the default config file
pub fn default_config_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("config.yaml"))
}

/// Get the path to the layout file
pub fn layout_file_path() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("layout.yaml"))
}

/// Ensure the data directory exists
pub fn ensure_data_dir() -> Option<PathBuf> {
    data_dir().and_then(|dir| {
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    })
}

/// Ensure the config directory exists
pub fn ensure_config_dir() -> Option<PathBuf> {
    config_dir().and_then(|dir| {
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    })
}
