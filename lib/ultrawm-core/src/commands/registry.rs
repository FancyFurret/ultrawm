use crate::config::KeyboardKeybind;
use crate::event_handlers::keyboard_keybind_tracker::KeyboardKeybindTracker;
use crate::event_loop_wm::WMOperationResult;
use crate::platform::{Position, WindowId};
use crate::wm::WindowManager;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

pub type CommandFn = fn(&mut WindowManager, Option<&CommandContext>) -> WMOperationResult<()>;
pub type CommandId = String;

#[derive(Debug, Clone)]
pub struct CommandContext {
    pub target_window: Option<WindowId>,
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

pub struct CommandDef {
    pub display_name: &'static str,
    pub id: &'static str,
    pub default_keybind: &'static str,
    pub handler: CommandFn,
    pub requires_window: bool,
}

static REGISTRY: LazyLock<RwLock<Vec<&'static CommandDef>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

pub fn register(def: &'static CommandDef) {
    if let Ok(mut registry) = REGISTRY.write() {
        registry.push(def);
    }
}

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

pub struct Command {
    pub id: CommandId,
    pub tracker: KeyboardKeybindTracker,
    pub handler: CommandFn,
}

pub fn build_commands(keybinds: &HashMap<String, KeyboardKeybind>) -> Vec<Command> {
    REGISTRY
        .read()
        .map(|registry| {
            registry
                .iter()
                .filter_map(|def| {
                    let keybind = keybinds
                        .get(def.id)
                        .cloned()
                        .unwrap_or_else(|| vec![def.default_keybind].into());

                    if keybind.combos().is_empty()
                        || keybind
                            .combos()
                            .iter()
                            .all(|combo| !combo.keys().any())
                    {
                        return None;
                    }

                    Some(Command {
                        id: def.id.to_string(),
                        tracker: KeyboardKeybindTracker::new(keybind),
                        handler: def.handler,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}
