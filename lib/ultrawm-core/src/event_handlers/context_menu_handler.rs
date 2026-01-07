use crate::config::Config;
use crate::event_handlers::mod_mouse_keybind_tracker::{KeybindEvent, ModMouseKeybindTracker};
use crate::event_handlers::EventHandler;
use crate::event_loop_main::run_on_main_thread;
use crate::event_loop_wm::WMOperationResult;
use crate::menu::{show_menu_at_position, MenuBuilder};
use crate::platform::{ContextMenuRequest, Position, WMEvent};
use crate::wm::WindowManager;
use crate::{
    CommandContext, AI_ORGANIZE_ALL_WINDOWS, AI_ORGANIZE_CURRENT_WINDOW, CLOSE_WINDOW,
    FLOAT_WINDOW, MINIMIZE_WINDOW,
};
use log::{debug, warn};

pub struct ContextMenuHandler {
    tracker: ModMouseKeybindTracker,
}

impl ContextMenuHandler {
    pub fn new() -> Self {
        let config = Config::current();
        Self {
            tracker: ModMouseKeybindTracker::new(
                config.mod_transform_bindings.context_menu.clone(),
            ),
        }
    }
}

impl EventHandler for ContextMenuHandler {
    fn handle_event(&mut self, event: &WMEvent, wm: &mut WindowManager) -> WMOperationResult<bool> {
        if let Some(keybind_event) = self.tracker.handle_event(event) {
            match keybind_event {
                KeybindEvent::Activate(pos) => {
                    // Click without drag - show context menu
                    let target_window = wm.find_window_at_position(&pos).map(|w| w.id());
                    debug!(
                        "Context menu activated at {:?}, window: {:?}",
                        pos, target_window
                    );

                    run_on_main_thread(move || {
                        show_context_menu(
                            ContextMenuRequest {
                                position: pos.clone(),
                                target_window,
                            },
                            pos.clone(),
                        )
                        .unwrap_or_else(|e| {
                            warn!("Failed to show context menu: {:?}", e);
                        });
                    });

                    return Ok(true);
                }
                _ => {}
            }
            return Ok(false);
        }

        if matches!(event, WMEvent::ConfigChanged) {
            let config = Config::current();
            self.tracker =
                ModMouseKeybindTracker::new(config.mod_transform_bindings.context_menu.clone());
        }

        Ok(false)
    }
}

fn show_context_menu(
    request: ContextMenuRequest,
    position: Position,
) -> Result<(), Box<dyn std::error::Error>> {
    let context = if let Some(window_id) = request.target_window {
        Some(CommandContext::with_window(window_id))
    } else {
        Some(CommandContext::with_position(request.position.clone()))
    };

    let mut menu_builder = MenuBuilder::new().with_context(context);

    menu_builder.add_label(&format!("UltraWM {}", crate::version()))?;
    menu_builder.add_separator()?;

    menu_builder.add_command(&AI_ORGANIZE_CURRENT_WINDOW)?;
    menu_builder.add_command(&AI_ORGANIZE_ALL_WINDOWS)?;

    if request.target_window.is_some() {
        menu_builder.add_separator()?;
        menu_builder.add_command(&FLOAT_WINDOW)?;
        menu_builder.add_command(&CLOSE_WINDOW)?;
        menu_builder.add_command(&MINIMIZE_WINDOW)?;
    }

    let menu = menu_builder.build();
    show_menu_at_position(&menu, &position);

    Ok(())
}
