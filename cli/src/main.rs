use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use ultrawm_core::{config::Config, UltraWMResult};

mod cli;
mod tray;

use cli::parse_args;
use tray::UltraWMTray;

fn main() -> UltraWMResult<()> {
    let args = parse_args();

    println!("Starting UltraWM");

    // Handle config loading
    let mut config = if args.use_defaults {
        Default::default()
    } else {
        let config_path = args.config_path.as_ref().map(|p| p.to_str().unwrap());
        match Config::load(config_path) {
            Ok(config) => {
                if let Some(path) = &args.config_path {
                    println!("Configuration loaded from: {}", path.display());
                }
                config
            }
            Err(e) => {
                eprintln!("Failed to load config: {}", e);
                if args.validate {
                    return Err(format!("Config loading failed: {}", e).into());
                } else {
                    eprintln!("Falling back to default configuration");
                    Default::default()
                }
            }
        }
    };

    // Handle dry-run mode
    if args.validate {
        let config_path = args.config_path.as_ref().map(|p| p.to_str().unwrap());
        match Config::load(config_path) {
            Ok(_) => println!("Config file is valid"),
            Err(e) => {
                return Err(format!("Config validation failed: {}", e).into());
            }
        }
        return Ok(());
    }

    if args.reset_layout {
        match ultrawm_core::reset_layout() {
            Ok(_) => println!("Successfully reset layout"),
            Err(_) => {
                return Err("Could not reset layout".into());
            }
        }
        return Ok(());
    }

    if args.no_persistence {
        println!("Starting with no persistence");
        config.persistence = false;
    }

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    // Initialize tray icon
    let _tray = match UltraWMTray::new(shutdown.clone()) {
        Ok(tray) => Some(tray),
        Err(e) => {
            eprintln!("Failed to initialize tray icon: {}", e);
            eprintln!("Continuing without tray icon...");
            None
        }
    };

    // Set up Ctrl+C handler
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, shutting down...");
        shutdown_clone.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    // Start the window manager with the loaded config
    ultrawm_core::start_with_config(shutdown, config)?;

    println!("UltraWM stopped");
    Ok(())
}
