use crate::event_loop_main::run_on_main_thread_blocking;
use crate::platform::macos::ffi::{get_window_id, AXUIElementExt};
use crate::platform::macos::platform::MacOSPlatform;
use crate::platform::traits::PlatformWindowImpl;
use crate::platform::{Bounds, PlatformError, PlatformResult, Position, ProcessId, Size, WindowId};
use application_services::accessibility_ui::AXUIElement;
use application_services::AXUIElementRef;
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_graphics::geometry::{CGPoint, CGSize};
use log::error;
use osakit::{Language, Script, Value};

#[derive(Debug, Clone)]
pub struct MacOSPlatformWindow {
    id: u32,
    pid: u32,
    pub element: AXUIElementExt,
}

thread_local! {
    static OSA_SCRIPT: Option<Script> = {
        let mut script = Script::new_from_source(
            Language::AppleScript,
            r#"
                on focus_window(process_name)
                    tell application "System Events"
                        set frontmost of process process_name to true
                    end tell
                end focus_window"#,
        );

        if let Err(_) = script.compile() {
            error!("Failed to compile OSA script");
            return None;
        }

        Some(script)
    };
}

impl MacOSPlatformWindow {
    pub fn new(element: AXUIElementExt) -> PlatformResult<Self> {
        let id = get_window_id(&element.element).ok_or("Could not get window id")?;
        let pid = element
            .element
            .get_pid()
            .map_err(|_| "Could not get window pid")?;

        element
            .position()
            .map_err(|_| "Window element is invalid (cannot get position)")?;
        element
            .size()
            .map_err(|_| "Window element is invalid (cannot get size)")?;

        Ok(Self {
            id,
            pid: pid as u32,
            element,
        })
    }
}

unsafe impl Send for MacOSPlatformWindow {}
unsafe impl Sync for MacOSPlatformWindow {}

impl PlatformWindowImpl for MacOSPlatformWindow {
    fn id(&self) -> WindowId {
        self.id as WindowId
    }

    fn pid(&self) -> ProcessId {
        self.pid
    }

    fn title(&self) -> String {
        self.element
            .title()
            .unwrap_or("Unknown".to_string())
            .to_string()
    }

    fn position(&self) -> Position {
        let position = self
            .element
            .position()
            .unwrap_or_else(|_| CGPoint::new(0.0, 0.0));
        let y_offset = MacOSPlatform::get_cgevent_y_offset();
        Position {
            x: position.x as i32,
            y: position.y as i32 + y_offset,
        }
    }

    fn size(&self) -> Size {
        let size = self
            .element
            .size()
            .unwrap_or_else(|_| CGSize::new(0.0, 0.0));
        Size {
            width: size.width as u32,
            height: size.height as u32,
        }
    }

    fn visible(&self) -> bool {
        !self.element.minimized().unwrap_or(false)
    }

    fn set_bounds(&self, bounds: &Bounds) -> PlatformResult<()> {
        // Set size BEFORE position to avoid intermediate states where the window
        // temporarily exceeds screen bounds. This is important when shrinking a window
        // that also moves (e.g., the bottom window in a vertical stack when the
        // resize handle moves down). Setting position first would temporarily place
        // the window in an invalid state, causing some apps to reject the resize.
        self.element.set_size(CGSize::new(
            bounds.size.width as f64,
            bounds.size.height as f64,
        ))?;
        let y_offset = MacOSPlatform::get_cgevent_y_offset();
        self.element.set_position(CGPoint::new(
            bounds.position.x as f64,
            (bounds.position.y - y_offset) as f64,
        ))?;
        Ok(())
    }

    /// Doesn't seem like there is any easy way to do this in macOS.
    /// Yabai resorts to it's scripting addition. For now we'll just use
    /// AppleScript. This may not work if you have multiple windows of one
    /// application open, though.
    fn focus(&self) -> PlatformResult<()> {
        let process_name = {
            let app_element = unsafe {
                let mut app_ref: AXUIElementRef = std::ptr::null();
                let result = application_services::AXUIElementCopyAttributeValue(
                    self.element.element.as_concrete_TypeRef(),
                    CFString::new("AXParent").as_concrete_TypeRef(),
                    &mut app_ref as *mut _ as *mut *const std::ffi::c_void,
                );

                if result == application_services::kAXErrorSuccess {
                    AXUIElement::wrap_under_create_rule(app_ref)
                } else {
                    return Err("Could not get application element".into());
                }
            };

            // Get the application name
            let app_ext = AXUIElementExt::from(app_element);
            app_ext.title().unwrap_or_else(|_| "unknown".to_string())
        };

        self.osa_focus(process_name)?;
        Ok(())
    }

    fn set_always_on_top(&self, _always_on_top: bool) -> PlatformResult<()> {
        // TODO: This would require disabling SIP, injecting into Dock.app, and calling private APIs
        // See Yabai as reference
        Ok(())
    }

    fn close(&self) -> PlatformResult<()> {
        let close_button = self
            .element
            .get_close_button()
            .map_err(|e| format!("Failed to get close button: {:?}", e))?;

        const PRESS_ACTION: &str = "AXPress";
        close_button
            .perform_action(PRESS_ACTION)
            .map_err(|e| format!("Failed to close window: {:?}", e))?;

        Ok(())
    }

    fn minimize(&self) -> PlatformResult<()> {
        self.element
            .set_minimized(true)
            .map_err(|e| format!("Failed to minimize window: {:?}", e))?;
        Ok(())
    }

    fn valid(&self) -> bool {
        self.element.position().is_ok() && self.element.size().is_ok()
    }
}

impl MacOSPlatformWindow {
    fn osa_focus(&self, process_name: String) -> PlatformResult<()> {
        run_on_main_thread_blocking(|| {
            OSA_SCRIPT.with(|script| {
                if let Some(script) = script.as_ref() {
                    script
                        .execute_function("focus_window", vec![Value::String(process_name)])
                        .map_err(|e| {
                            PlatformError::Error(format!("Failed to execute AppleScript: {:?}", e))
                        })?;
                };

                Ok(())
            })
        })
    }
}
