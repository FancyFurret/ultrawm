use resvg::usvg::TreeParsing;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct UltraWMTray {
    _tray_icon: TrayIcon,
}

impl UltraWMTray {
    pub fn new(shutdown: Arc<AtomicBool>) -> Result<Self, Box<dyn std::error::Error>> {
        let icon_data = load_svg_icon()?;
        let icon = Icon::from_rgba(icon_data, 32, 32)?;

        let tray_menu = Menu::new();
        let version_item =
            MenuItem::new(format!("UltraWM {}", ultrawm_core::version()), false, None);
        let separator = PredefinedMenuItem::separator();
        let quit_item = MenuItem::new("Quit", true, None);

        tray_menu.append(&version_item)?;
        tray_menu.append(&separator)?;
        tray_menu.append(&quit_item)?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("UltraWM")
            .with_icon(icon)
            .build()?;

        let quit_id = quit_item.id().clone();

        let shutdown_clone = shutdown.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| match event.id {
            id if id == quit_id => {
                shutdown_clone.store(true, Ordering::SeqCst);
            }
            _ => {}
        }));

        Ok(Self {
            _tray_icon: tray_icon,
        })
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

    let options = resvg::usvg::Options::default();
    let rtree = resvg::usvg::Tree::from_data(svg_data.as_bytes(), &options)?;

    let size = rtree.view_box.rect.size();
    let (width, height) = (size.width() as u32, size.height() as u32);

    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width, height).ok_or("Failed to create pixmap")?;

    let tree = resvg::Tree::from_usvg(&rtree);
    tree.render(resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());

    Ok(pixmap.data().to_vec())
}
