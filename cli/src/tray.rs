use log::warn;
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, TreeParsing};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use ultrawm_core::Config;

type CallbackFn = Box<dyn Fn() + Send + Sync>;
type ConfigGetterFn = Box<dyn Fn(&Config) -> bool + Send + Sync>;

pub struct UltraWMTray {
    _tray_icon: TrayIcon,
    check_items: Arc<Mutex<HashMap<MenuId, (CheckMenuItem, ConfigGetterFn)>>>,
}

impl UltraWMTray {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let icon_data = load_svg_icon()?;
        let icon = Icon::from_rgba(icon_data, 32, 32)?;

        let tray_menu = Menu::new();
        let callbacks: Arc<Mutex<HashMap<MenuId, CallbackFn>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let check_items: Arc<Mutex<HashMap<MenuId, (CheckMenuItem, ConfigGetterFn)>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let mut tray_builder = TrayBuilder {
            menu: &tray_menu,
            callbacks: callbacks.clone(),
            check_items: check_items.clone(),
        };

        // Items
        let version_item =
            MenuItem::new(format!("UltraWM {}", ultrawm_core::version()), false, None);
        tray_menu.append(&version_item)?;

        let separator = PredefinedMenuItem::separator();
        tray_menu.append(&separator)?;

        tray_builder.register_item("Reload Config", || {
            let path = Config::current().config_path.clone();

            if let Some(path) = path {
                let config = Config::load(path.to_str());
                if let Ok(config) = config {
                    ultrawm_core::load_config(config)
                        .unwrap_or_else(|e| warn!("Failed to load config: {:?}", e));
                } else {
                    warn!("Failed to load config");
                }
            } else {
                warn!("No config file found, loading default config...");
                let config = Config::default().clone();
                ultrawm_core::load_config(config)
                    .unwrap_or_else(|e| warn!("Failed to load config: {:?}", e));
            }
        })?;

        tray_builder.register_item("Open Config", || {
            let path = Config::current()
                .config_path
                .clone()
                .or_else(|| Config::default_config_path());

            if let Some(path) = path {
                open::that(path).unwrap_or_else(|e| warn!("Failed to open config file: {:?}", e));
            }
        })?;

        tray_menu.append(&separator)?;

        tray_builder.register_config_check_item(
            "Persistence",
            |c| c.persistence,
            |c, v| c.persistence = v,
        )?;

        tray_builder.register_config_check_item(
            "Live Window Resize",
            |c| c.live_window_resize,
            |c, v| c.live_window_resize = v,
        )?;

        tray_builder.register_config_check_item(
            "Resize Handles",
            |c| c.resize_handles,
            |c, v| c.resize_handles = v,
        )?;

        tray_builder.register_config_check_item(
            "Float New Windows",
            |c| c.float_new_windows,
            |c, v| c.float_new_windows = v,
        )?;

        tray_menu.append(&separator)?;

        tray_builder.register_item("Quit", || {
            ultrawm_core::shutdown();
        })?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("UltraWM")
            .with_icon(icon)
            .build()?;

        // Set up the event handler
        let callbacks_for_handler = callbacks.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if let Ok(callbacks_map) = callbacks_for_handler.lock() {
                if let Some(callback) = callbacks_map.get(&event.id) {
                    callback();
                }
            }
        }));

        let tray = Self {
            _tray_icon: tray_icon,
            check_items,
        };

        tray.sync_with_config(&Config::current());

        Ok(tray)
    }

    pub fn sync_with_config(&self, config: &Config) {
        // Update check items to reflect current config
        if let Ok(items) = self.check_items.lock() {
            for (_, (check_item, getter)) in items.iter() {
                let current_value = getter(config);
                check_item.set_checked(current_value);
            }
        }
    }
}

struct TrayBuilder<'a> {
    menu: &'a Menu,
    callbacks: Arc<Mutex<HashMap<MenuId, CallbackFn>>>,
    check_items: Arc<Mutex<HashMap<MenuId, (CheckMenuItem, ConfigGetterFn)>>>,
}

impl<'a> TrayBuilder<'a> {
    fn register_item<F>(
        &mut self,
        text: &str,
        callback: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let item = MenuItem::new(text, true, None);
        let id = item.id().clone();
        self.menu.append(&item)?;

        if let Ok(mut callbacks_map) = self.callbacks.lock() {
            callbacks_map.insert(id, Box::new(callback));
        }

        Ok(())
    }

    fn register_config_check_item<G, S>(
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

        // Store a simple toggle callback
        if let Ok(mut callbacks_map) = self.callbacks.lock() {
            callbacks_map.insert(
                id,
                Box::new(move || {
                    let mut config = Config::current().clone();
                    let new_value = !config_getter(&config);
                    config_setter(&mut config, new_value);
                    ultrawm_core::load_config(config)
                        .unwrap_or_else(|e| warn!("Failed to set config value: {:?}", e));
                }),
            );
        }

        Ok(())
    }
}

fn load_svg_icon() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // language=SVG
    let svg_data = r#"
        <svg width="32" height="32" viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
          <defs>
            <mask id="cutout">
              <rect x="0" y="0" width="32" height="32" fill="white"/>
              <rect x="6" y="6" width="8" height="20" fill="black"/>
              <rect x="17" y="6" width="9" height="9" fill="black"/>
              <rect x="17" y="17" width="9" height="9" fill="black"/>
            </mask>
          </defs>
          
          <rect x="2" y="2" width="28" height="28" rx="5" ry="5" fill="white" mask="url(#cutout)"/>
        </svg>"#;

    let options = Options::default();
    let rtree = resvg::usvg::Tree::from_data(svg_data.as_bytes(), &options)?;

    let size = rtree.view_box.rect.size();
    let (width, height) = (size.width() as u32, size.height() as u32);

    let mut pixmap = Pixmap::new(width, height).ok_or("Failed to create pixmap")?;

    let tree = resvg::Tree::from_usvg(&rtree);
    tree.render(Transform::default(), &mut pixmap.as_mut());

    Ok(pixmap.data().to_vec())
}
