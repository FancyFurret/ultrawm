use crate::config::KeyboardKeybind;
use crate::event_handlers::keyboard_keybind_tracker::KeyboardKeybindTracker;
use crate::event_loop_wm::WMOperationResult;
use crate::platform::{Position, WindowId};
use crate::wm::WindowManager;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

pub type CommandFn = fn(&mut WindowManager, Option<&CommandContext>) -> WMOperationResult<()>;
pub type CommandId = String;

/// Context passed to command handlers when triggered from UI elements like context menus
#[derive(Debug, Clone)]
pub struct CommandContext {
    /// The target window for this command (e.g., the window right-clicked on)
    pub target_window: Option<WindowId>,
    /// The position where the command was triggered (e.g., context menu position)
    pub position: Option<Position>,
}

impl CommandContext {
    pub fn new() -> Self {
        Self {
            target_window: None,
            position: None,
        }
    }

    pub fn with_window(window_id: WindowId) -> Self {
        Self {
            target_window: Some(window_id),
            position: None,
        }
    }

    pub fn with_position(position: Position) -> Self {
        Self {
            target_window: None,
            position: Some(position),
        }
    }

    pub fn with_window_and_position(window_id: WindowId, position: Position) -> Self {
        Self {
            target_window: Some(window_id),
            position: Some(position),
        }
    }
}

/// Static command definition - contains everything about a command
pub struct CommandDef {
    pub display_name: &'static str,
    pub id: &'static str,
    pub default_keybind: &'static str,
    pub handler: CommandFn,
    pub requires_window: bool,
}

/// Global command registry
static REGISTRY: LazyLock<RwLock<Vec<&'static CommandDef>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// Register a command with the global registry
pub fn register(def: &'static CommandDef) {
    if let Ok(mut registry) = REGISTRY.write() {
        registry.push(def);
    }
}

/// Get all registered command names and their default keybinds
pub fn get_defaults() -> HashMap<String, String> {
    REGISTRY
        .read()
        .map(|registry| {
            registry
                .iter()
                .map(|def| (def.id.to_string(), def.default_keybind.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Internal command for the handler
pub(crate) struct Command {
    pub id: CommandId,
    pub tracker: KeyboardKeybindTracker,
    pub handler: CommandFn,
}

/// Build command handlers from the registry using config keybinds
pub(crate) fn build_commands(keybinds: &HashMap<String, KeyboardKeybind>) -> Vec<Command> {
    REGISTRY
        .read()
        .map(|registry| {
            registry
                .iter()
                .map(|def| {
                    let keybind = keybinds
                        .get(def.id)
                        .cloned()
                        .unwrap_or_else(|| vec![def.default_keybind].into());

                    Command {
                        id: def.id.to_string(),
                        tracker: KeyboardKeybindTracker::new(keybind),
                        handler: def.handler,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}
