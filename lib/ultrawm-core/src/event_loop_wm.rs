use crate::window_move_handler::WindowMoveHandler;
use crate::window_resize_handler::WindowResizeHandler;
use crate::{
    event_loop_main::EventLoopMain,
    platform::{EventBridge, PlatformEvent},
    wm::WindowManager,
    UltraWMResult,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct EventLoopWM {}

impl EventLoopWM {
    pub async fn run(mut bridge: EventBridge, shutdown: Arc<AtomicBool>) -> UltraWMResult<()> {
        println!("Handling events...");

        let mut wm = WindowManager::new()?;

        let mut move_handler = WindowMoveHandler::new().await?;
        let mut resize_handler = WindowResizeHandler::new().await?;

        while !shutdown.load(Ordering::SeqCst) {
            let event = bridge
                .next_event()
                .await
                .ok_or("Could not get next event")?;

            match &event {
                PlatformEvent::WindowOpened(window) => {
                    wm.track_window(window.clone()).unwrap_or_else(|_| {
                        println!("Could not track window");
                    });
                }
                PlatformEvent::WindowShown(_) => {
                    // TODO: If the window was hidden, then bring it back to where it was
                }
                PlatformEvent::WindowClosed(id) | PlatformEvent::WindowHidden(id) => {
                    // TODO: Check if manageable
                    wm.remove_window(*id).unwrap_or_else(|_| {
                        // println!("Could not remove window");
                    });
                }
                _ => {}
            }

            move_handler.handle(&event, &mut wm)?;
            resize_handler.handle(&event, &move_handler, &mut wm)?;
        }

        wm.cleanup()?;
        EventLoopMain::shutdown();
        Ok(())
    }
}
