use log::{error, info, trace, warn};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::env;
use std::path::PathBuf;
use ultrawm_core::{config::Config, UltraWMResult};

mod cli;
mod logger;
mod tray;

use cli::parse_args;
use tray::UltraWMTray;

fn main() -> UltraWMResult<()> {
    let args = parse_args();

    // Initialize logger
    logger::init_logger(args.verbose).expect("Failed to initialize logger");

    info!("Starting UltraWM");
    trace!("Command: {}", env::args().collect::<Vec<_>>().join(" "));

    // Handle config loading
    let mut config = if args.use_defaults {
        trace!("Using default configuration");
        Default::default()
    } else {
        let config_path = args.config_path.as_ref().map(|p| p.to_str().unwrap());
        match Config::load(config_path, true) {
            Ok(config) => {
                if let Some(path) = &args.config_path {
                    info!("Configuration loaded from: {}", path.display());
                }
                trace!("Starting with configuration: {config:?}");
                config
            }
            Err(e) => {
                error!("Failed to load config: {}", e);
                if args.validate {
                    return Err(format!("Config loading failed: {e:?}").into());
                } else {
                    warn!("Falling back to default configuration");
                    Default::default()
                }
            }
        }
    };

    // Handle dry-run mode
    if args.validate {
        info!("Config validation successful");
        return Ok(());
    }

    if args.reset_layout {
        match ultrawm_core::reset_layout() {
            Ok(_) => info!("Successfully reset layout"),
            Err(_) => {
                error!("Could not reset layout");
                return Err("Could not reset layout".into());
            }
        }
        return Ok(());
    }

    if args.no_persistence {
        info!("Starting with no persistence");
        config.persistence = false;
    }

    // Set up config file watching if we have a config path
    let _watcher = if let Some(path) = config.config_path.clone() {
        match setup_config_watcher(path) {
            Ok(watcher) => Some(watcher),
            Err(e) => {
                warn!("Failed to set up config file watcher: {:?}", e);
                None
            }
        }
    } else {
        None
    };

    // Initialize tray icon
    let _tray = match UltraWMTray::new() {
        Ok(tray) => {
            trace!("Tray icon initialized successfully");
            Some(tray)
        }
        Err(e) => {
            warn!("Failed to initialize tray icon: {}", e);
            warn!("Continuing without tray icon...");
            None
        }
    };

    // Set up Ctrl+C handler
    ctrlc::set_handler(move || {
        info!("Received Ctrl+C, shutting down...");
        ultrawm_core::shutdown();
    })
    .expect("Error setting Ctrl+C handler");

    // Start the window manager with the loaded config
    ultrawm_core::start_with_config(config)?;

    info!("UltraWM stopped");
    Ok(())
}

fn setup_config_watcher(config_path: PathBuf) -> UltraWMResult<RecommendedWatcher> {
    let config_path_clone = config_path.clone();
    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
            Ok(event) => match event.kind {
                EventKind::Modify(_) | EventKind::Create(_) => {
                    match Config::load(Some(config_path_clone.to_str().unwrap()), false) {
                        Ok(new_config) => {
                            ultrawm_core::load_config(new_config)
                                .unwrap_or_else(|e| warn!("Failed to load config: {:?}", e));
                        }
                        Err(e) => {
                            error!("Failed to reload config: {}", e);
                            warn!("Keeping previous configuration");
                        }
                    }
                }
                EventKind::Remove(_) => {
                    warn!("Config file was removed, keeping current configuration");
                }
                _ => {}
            },
            Err(e) => error!("File watcher error: {:?}", e),
        })
        .map_err(|e| format!("Failed to create file watcher: {:?}", e))?;

    watcher
        .watch(&config_path, RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch config file: {:?}", e))?;

    Ok(watcher)
}
