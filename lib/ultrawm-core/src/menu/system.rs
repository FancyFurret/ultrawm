use crate::menu::accelerator::keybind_to_accelerator;
use crate::{CommandDef, Config};
use log::{debug, warn};
use muda::accelerator::Accelerator;
use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tray_icon::menu::MenuEvent as TrayMenuEvent;

fn get_command_accelerator(cmd: &'static CommandDef) -> Option<Accelerator> {
    let config = Config::current();
    let keybind = config
        .commands
        .keybinds
        .get(cmd.id)
        .cloned()
        .unwrap_or_else(|| vec![cmd.default_keybind].into());

    keybind_to_accelerator(&keybind)
}

type CallbackFn = Box<dyn Fn() + Send + Sync>;
type ConfigGetterFn = Box<dyn Fn(&Config) -> bool + Send + Sync>;
pub type ConfigGetterFnArc = Arc<ConfigGetterFn>;

static MENU_CALLBACKS: Mutex<Option<Arc<Mutex<HashMap<String, CallbackFn>>>>> = Mutex::new(None);

pub struct MenuSystem;

impl MenuSystem {
    pub fn initialize() -> Result<(), String> {
        TrayMenuEvent::set_event_handler(Some(move |event: TrayMenuEvent| {
            let id_str = event.id.0.as_str();
            MenuSystem::trigger_callback(id_str);
        }));

        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let id_str = event.id.0.as_str();
            MenuSystem::trigger_callback(id_str);
        }));

        Ok(())
    }

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

    pub fn trigger_callback(id: &str) {
        if let Ok(callbacks_opt) = MENU_CALLBACKS.lock() {
            if let Some(callbacks) = callbacks_opt.as_ref() {
                if let Ok(callbacks_map) = callbacks.lock() {
                    if let Some(callback) = callbacks_map.get(id) {
                        callback();
                        return;
                    }
                }
            }
        }

        debug!("No callback found for menu item: {}", id);
    }
}

pub struct MenuBuilder {
    menu: Menu,
    context: Option<crate::CommandContext>,
    check_items: Arc<Mutex<HashMap<String, (CheckMenuItem, ConfigGetterFnArc)>>>,
}

impl MenuBuilder {
    pub fn new() -> Self {
        Self {
            menu: Menu::new(),
            context: None,
            check_items: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_context(mut self, context: Option<crate::CommandContext>) -> Self {
        self.context = context;
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
        cmd: &'static crate::CommandDef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if cmd.requires_window && self.context.is_none() {
            return Ok(());
        }

        let accelerator = get_command_accelerator(cmd);
        let item_id = format!("cmd:{}", cmd.id);
        let item = MenuItem::with_id(item_id.clone(), cmd.display_name, true, accelerator);
        self.menu.append(&item)?;

        let cmd_id = cmd.id.to_string();
        let context = self.context.clone();
        MenuSystem::register_callback(
            item_id,
            Box::new(move || {
                crate::trigger_command_with_context(&cmd_id, context.clone());
            }),
        );

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

        MenuSystem::register_callback(id_str, Box::new(callback));

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

        let getter_arc = Arc::new(Box::new(config_getter.clone()) as ConfigGetterFn);
        if self.context.is_none() {
            if let Ok(mut check_items_map) = self.check_items.lock() {
                check_items_map.insert(id_str.clone(), (item, Arc::clone(&getter_arc)));
            }
        }

        let getter_clone = Arc::clone(&getter_arc);
        MenuSystem::register_callback(
            id_str,
            Box::new(move || {
                let mut config = Config::current().clone();
                let new_value = !getter_clone(&config);
                config_setter(&mut config, new_value);
                crate::load_config(config)
                    .unwrap_or_else(|e| warn!("Failed to set config value: {:?}", e));
            }),
        );

        Ok(())
    }

    pub fn build(self) -> Menu {
        self.menu
    }

    pub fn get_check_items(
        &self,
    ) -> Arc<Mutex<HashMap<String, (CheckMenuItem, ConfigGetterFnArc)>>> {
        self.check_items.clone()
    }
}
