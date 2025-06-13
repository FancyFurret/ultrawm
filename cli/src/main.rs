use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use ultrawm_core::UltraWMResult;

fn main() -> UltraWMResult<()> {
    println!("Starting UltraWM");

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    // Set up Ctrl+C handler
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, shutting down...");
        shutdown_clone.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    // Start the window manager
    ultrawm_core::start(shutdown)?;

    println!("UltraWM stopped");
    Ok(())
}
