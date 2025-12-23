use crate::menu_helpers::get_command_accelerator;
use log::{trace, warn};
use muda::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tray_icon::menu::MenuEvent as TrayMenuEvent;
use ultrawm_core::Config;

type CallbackFn = Box<dyn Fn() + Send + Sync>;
type ConfigGetterFn = Box<dyn Fn(&Config) -> bool + Send + Sync>;

/// Unified menu event handler that routes events to the appropriate callbacks
/// This handles both tray menu events (tray_icon) and context menu events (muda)
/// since tray_icon uses muda internally
static MENU_CALLBACKS: Mutex<Option<Arc<Mutex<HashMap<String, CallbackFn>>>>> = Mutex::new(None);

/// Initialize the unified menu event handler
/// This must be called before creating any menus
pub fn init_unified_handler() {
    // Set up handler for tray_icon menus (which use muda internally)
    TrayMenuEvent::set_event_handler(Some(move |event: TrayMenuEvent| {
        trace!("Menu event received: {:?}", event);

        // Convert MenuId to string for lookup
        let id_str = event.id.0.as_str();

        // Try to find callback in our unified registry
        if let Ok(callbacks_opt) = MENU_CALLBACKS.lock() {
            if let Some(callbacks) = callbacks_opt.as_ref() {
                if let Ok(callbacks_map) = callbacks.lock() {
                    if let Some(callback) = callbacks_map.get(id_str) {
                        trace!("Found callback for menu item: {}", id_str);
                        callback();
                        return;
                    }
                }
            }
        }

        // Fallback: check if it's a command ID (for context menus using muda directly)
        if let Some(cmd_id) = id_str.strip_prefix("cmd:") {
            trace!("Context menu: triggering command '{}'", cmd_id);
            // Get context from stored context menu request
            let context = {
                use ultrawm_core::CommandContext;
                if let Some(current_menu) = crate::context_menu::get_current_context_menu() {
                    if let Some(window_id) = current_menu.target_window {
                        Some(CommandContext::with_window(window_id))
                    } else {
                        Some(CommandContext::with_position(current_menu.position))
                    }
                } else {
                    None
                }
            };
            ultrawm_core::trigger_command_with_context(cmd_id, context);
        } else {
            trace!("No handler found for menu item: {}", id_str);
        }
    }));
}

/// Register a callback for a menu item by ID
pub fn register_callback(id: String, callback: CallbackFn) {
    if let Ok(mut callbacks_opt) = MENU_CALLBACKS.lock() {
        if callbacks_opt.is_none() {
            *callbacks_opt = Some(Arc::new(Mutex::new(HashMap::new())));
        }
        if let Some(callbacks) = callbacks_opt.as_ref() {
            if let Ok(mut callbacks_map) = callbacks.lock() {
                callbacks_map.insert(id, callback);
            }
        }
    }
}

pub struct MenuBuilder {
    menu: Menu,
    has_window: bool,
    // For tray menus: track check items for sync_with_config
    check_items: Arc<Mutex<HashMap<String, (CheckMenuItem, ConfigGetterFn)>>>,
}

impl MenuBuilder {
    pub fn new() -> Self {
        Self {
            menu: Menu::new(),
            has_window: false,
            check_items: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_window(mut self, has_window: bool) -> Self {
        self.has_window = has_window;
        self
    }

    pub fn add_label(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let item = MenuItem::new(text, false, None);
        self.menu.append(&item)?;
        Ok(())
    }

    pub fn add_separator(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let separator = PredefinedMenuItem::separator();
        self.menu.append(&separator)?;
        Ok(())
    }

    pub fn add_command(
        &mut self,
        cmd: &'static ultrawm_core::CommandDef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Only add command if it doesn't require a window, or if we have a window
        if cmd.requires_window && !self.has_window {
            return Ok(());
        }

        let accelerator = get_command_accelerator(cmd);
        let item = MenuItem::with_id(
            format!("cmd:{}", cmd.id),
            cmd.display_name,
            true,
            accelerator,
        );
        self.menu.append(&item)?;
        Ok(())
    }

    pub fn add_item<F>(&mut self, text: &str, callback: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let item = MenuItem::new(text, true, None);
        let id = item.id().clone();
        let id_str = id.0.as_str().to_string();
        self.menu.append(&item)?;

        register_callback(id_str, Box::new(callback));

        Ok(())
    }

    pub fn add_config_check_item<G, S>(
        &mut self,
        text: &str,
        config_getter: G,
        config_setter: S,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        G: Fn(&Config) -> bool + Send + Sync + 'static + Clone,
        S: Fn(&mut Config, bool) + Send + Sync + 'static + Clone,
    {
        let initial_value = config_getter(&Config::current());
        let item = CheckMenuItem::new(text, true, initial_value, None);
        let id = item.id().clone();
        let id_str = id.0.as_str().to_string();
        self.menu.append(&item)?;

        // Store the check item and its config getter
        if let Ok(mut check_items_map) = self.check_items.lock() {
            check_items_map.insert(id_str.clone(), (item, Box::new(config_getter.clone())));
        }

        // Register toggle callback
        register_callback(
            id_str,
            Box::new(move || {
                let mut config = Config::current().clone();
                let new_value = !config_getter(&config);
                config_setter(&mut config, new_value);
                ultrawm_core::load_config(config)
                    .unwrap_or_else(|e| warn!("Failed to set config value: {:?}", e));
            }),
        );

        Ok(())
    }

    pub fn build(self) -> Menu {
        self.menu
    }

    pub fn get_check_items(&self) -> Arc<Mutex<HashMap<String, (CheckMenuItem, ConfigGetterFn)>>> {
        self.check_items.clone()
    }
}

pub type ContextMenuBuilder = MenuBuilder;
