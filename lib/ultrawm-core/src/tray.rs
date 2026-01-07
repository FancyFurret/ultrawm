use crate::menu::system::{ConfigGetterFnArc, MenuBuilder};
use crate::{paths, Config};
use log::{info, warn};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::Options;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tray_icon::{menu::CheckMenuItem, Icon, TrayIcon, TrayIconBuilder};

pub struct UltraWMTray {
    _tray_icon: TrayIcon,
    check_items: Arc<Mutex<HashMap<String, (CheckMenuItem, ConfigGetterFnArc)>>>,
}

impl UltraWMTray {
    pub fn initialize() -> Result<Self, Box<dyn std::error::Error>> {
        let icon_data = load_svg_icon()?;
        let icon = Icon::from_rgba(icon_data, 32, 32)?;

        let mut menu_builder = MenuBuilder::new();

        // Version label
        menu_builder.add_label(&format!("UltraWM {}", crate::version()))?;
        menu_builder.add_separator()?;

        // Commands section
        menu_builder.add_label("Commands")?;
        menu_builder.add_command(&crate::AI_ORGANIZE_ALL_WINDOWS)?;
        menu_builder.add_command(&crate::AI_ORGANIZE_CURRENT_WINDOW)?;

        menu_builder.add_separator()?;
        menu_builder.add_label("Options")?;

        menu_builder.add_config_check_item(
            "Persistence",
            |c| c.persistence,
            |c, v| c.persistence = v,
        )?;

        menu_builder.add_config_check_item(
            "Live Window Resize",
            |c| c.live_window_resize,
            |c, v| c.live_window_resize = v,
        )?;

        menu_builder.add_config_check_item(
            "Resize Handles",
            |c| c.resize_handles,
            |c, v| c.resize_handles = v,
        )?;

        menu_builder.add_config_check_item(
            "Float New Windows",
            |c| c.float_new_windows,
            |c, v| c.float_new_windows = v,
        )?;

        menu_builder.add_config_check_item(
            "Focus on Hover",
            |c| c.focus_on_hover,
            |c, v| c.focus_on_hover = v,
        )?;

        menu_builder.add_config_check_item(
            "Focus on Drag",
            |c| c.focus_on_drag,
            |c, v| c.focus_on_drag = v,
        )?;

        menu_builder.add_separator()?;

        menu_builder.add_item("Reload Config", || {
            let path = Config::current().config_path.clone();

            if let Some(path) = path {
                let config = Config::load(path.to_str(), false);
                if let Ok(config) = config {
                    crate::load_config(config)
                        .unwrap_or_else(|e| warn!("Failed to load config: {:?}", e));
                } else {
                    warn!("Failed to load config");
                }
            } else {
                warn!("No config file found, loading default config...");
                let config = Config::default().clone();
                crate::load_config(config)
                    .unwrap_or_else(|e| warn!("Failed to load config: {:?}", e));
            }
        })?;

        menu_builder.add_item("Open Config", || {
            let path = Config::current()
                .config_path
                .clone()
                .or_else(|| paths::default_config_path());

            if let Some(path) = path {
                open::that(path).unwrap_or_else(|e| warn!("Failed to open config file: {:?}", e));
            }
        })?;

        menu_builder.add_separator()?;

        menu_builder.add_item("Open Log", || {
            if let Some(log_path) = paths::log_file_path() {
                open::that(&log_path).unwrap_or_else(|e| warn!("Failed to open log file: {:?}", e));
            } else {
                warn!("Log file path not available");
            }
        })?;

        menu_builder.add_separator()?;

        menu_builder.add_item("Quit", || {
            crate::shutdown();
        })?;

        let check_items = menu_builder.get_check_items();
        let menu = menu_builder.build();

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("UltraWM")
            .with_icon(icon)
            .build()?;

        let tray = Self {
            _tray_icon: tray_icon,
            check_items,
        };

        tray.sync_with_config(&Config::current());

        info!("Tray icon initialized");

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

fn load_svg_icon() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Use white icon on Windows, black on other platforms
    let fill_color = "white";

    // language=SVG
    let svg_data = format!(
        r#"
        <svg width="32" height="32" viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
          <path fill="{}" fill-rule="evenodd"
            d="
              M 2 8
              a 6 6 0 0 1 6 -6
              h 16
              a 6 6 0 0 1 6 6
              v 16
              a 6 6 0 0 1 -6 6
              h -16
              a 6 6 0 0 1 -6 -6
              z
              M 6 7
              a 1 1 0 0 1 1 -1
              h 6
              a 1 1 0 0 1 1 1
              v 18
              a 1 1 0 0 1 -1 1
              h -6
              a 1 1 0 0 1 -1 -1
              z
              M 18 7
              a 1 1 0 0 1 1 -1
              h 6
              a 1 1 0 0 1 1 1
              v 6
              a 1 1 0 0 1 -1 1
              h -6
              a 1 1 0 0 1 -1 -1
              z
              M 18 19
              a 1 1 0 0 1 1 -1
              h 6
              a 1 1 0 0 1 1 1
              v 6
              a 1 1 0 0 1 -1 1
              h -6
              a 1 1 0 0 1 -1 -1
              z
            "
          />
        </svg>"#,
        fill_color
    );

    let options = Options::default();
    let rtree = resvg::usvg::Tree::from_data(svg_data.as_bytes(), &options)?;

    let size = rtree.size();
    let (width, height) = (size.width() as u32, size.height() as u32);

    let mut pixmap = Pixmap::new(width, height).ok_or("Failed to create pixmap")?;

    resvg::render(&rtree, Transform::default(), &mut pixmap.as_mut());

    Ok(pixmap.data().to_vec())
}
