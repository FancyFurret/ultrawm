use log::{error, info, trace, warn};
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
        match Config::load(config_path) {
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

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    // Initialize tray icon
    let _tray = match UltraWMTray::new(shutdown.clone()) {
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
        shutdown_clone.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    // Start the window manager with the loaded config
    ultrawm_core::start_with_config(shutdown, config)?;

    info!("UltraWM stopped");
    Ok(())
}
