use crate::menu_helpers::get_command_accelerator;
use log::{trace, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tray_icon::menu::{CheckMenuItem, MenuId as TrayMenuId, MenuItem as TrayMenuItem};
use tray_icon::menu::{Menu as TrayMenu, MenuEvent as TrayMenuEvent, PredefinedMenuItem};
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
            ultrawm_core::trigger_command(cmd_id);
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

/// Builder for tray menus
pub struct TrayMenuBuilder<'a> {
    menu: &'a TrayMenu,
    check_items: Arc<Mutex<HashMap<TrayMenuId, (CheckMenuItem, ConfigGetterFn)>>>,
}

impl<'a> TrayMenuBuilder<'a> {
    pub fn new(menu: &'a TrayMenu) -> Self {
        Self {
            menu,
            check_items: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_label(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let item = TrayMenuItem::new(text, false, None);
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
        let accelerator = get_command_accelerator(cmd);
        let item = TrayMenuItem::new(cmd.display_name, true, accelerator);
        let id = item.id().clone();
        let id_str = id.0.as_str().to_string();
        self.menu.append(&item)?;

        // Register callback that triggers the command
        register_callback(
            id_str,
            Box::new(move || {
                ultrawm_core::trigger_command(cmd.id);
            }),
        );

        Ok(())
    }

    pub fn add_item<F>(&mut self, text: &str, callback: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let item = TrayMenuItem::new(text, true, None);
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
        self.menu.append(&item)?;

        // Store the check item and its config getter
        if let Ok(mut check_items_map) = self.check_items.lock() {
            check_items_map.insert(id.clone(), (item, Box::new(config_getter.clone())));
        }

        // Register toggle callback
        let id_str = id.0.as_str().to_string();
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

    pub fn get_check_items(
        &self,
    ) -> Arc<Mutex<HashMap<TrayMenuId, (CheckMenuItem, ConfigGetterFn)>>> {
        self.check_items.clone()
    }
}

/// Builder for context menus (muda)
pub struct ContextMenuBuilder {
    menu: muda::Menu,
}

impl ContextMenuBuilder {
    pub fn new() -> Self {
        Self {
            menu: muda::Menu::new(),
        }
    }

    pub fn add_command(
        &mut self,
        cmd: &'static ultrawm_core::CommandDef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let accelerator = get_command_accelerator(cmd);
        let item = muda::MenuItem::with_id(
            format!("cmd:{}", cmd.id),
            cmd.display_name,
            true,
            accelerator,
        );
        self.menu.append(&item)?;
        Ok(())
    }

    pub fn build(self) -> muda::Menu {
        self.menu
    }
}
