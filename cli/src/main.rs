#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use log::{debug, error, info, trace, warn};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::env;
use std::path::PathBuf;
use ultrawm_core::{config::Config, register_commands, UltraWMResult};

mod cli;
mod error_dialog;
mod logger;

use cli::parse_args;

fn main() {
    // Parse args first to check for console flag
    let args = parse_args();

    // On Windows, allocate console if requested
    #[cfg(target_os = "windows")]
    if args.console {
        allocate_console();
    }

    // Initialize logger early (before error handling)
    if let Err(e) = logger::init_logger(args.quiet, args.verbose) {
        eprintln!("Failed to initialize logger: {}", e);
        return;
    }

    // Run main logic and handle fatal errors with dialog
    match run_main(args) {
        Ok(()) => {}
        Err(e) => {
            error!("Fatal error: {:?}", e);
            if ultrawm_core::check_panic().is_none() {
                error_dialog::show_error(&e);
            }
            std::process::exit(1);
        }
    }
}

fn run_main(args: cli::Args) -> UltraWMResult<()> {
    info!("Starting UltraWM");
    debug!("Command: {}", env::args().collect::<Vec<_>>().join(" "));

    // Register commands before config loading so defaults can be filled
    register_commands();

    // Handle config loading
    let mut config = if args.use_defaults {
        debug!("Using default configuration");
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

    // Set up panic handler to catch panics from background threads
    ultrawm_core::setup_panic_handler();

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

#[cfg(target_os = "windows")]
fn allocate_console() {
    use windows::Win32::System::Console::*;
    unsafe {
        // First try to attach to the parent console (if running from terminal)
        let attach_result = AttachConsole(u32::MAX);

        if attach_result.is_ok() {
            // Successfully attached to parent console
            // Redirect stdout, stderr, and stdin to the console
            let _ = std::io::stdout();
            let _ = std::io::stderr();
            let _ = std::io::stdin();
        } else {
            // No parent console, allocate a new one
            if AllocConsole().is_ok() {
                // Redirect stdout, stderr, and stdin to the new console
                let _ = std::io::stdout();
                let _ = std::io::stderr();
                let _ = std::io::stdin();
            }
        }
    }
}
