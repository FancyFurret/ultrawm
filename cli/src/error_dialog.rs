use native_dialog::{DialogBuilder, MessageLevel};
use ultrawm_core::UltraWMFatalError;

/// Show an error dialog for a fatal error
pub fn show_error(error: &UltraWMFatalError) {
    let title = "UltraWM Error";
    let message = match error {
        UltraWMFatalError::Error(msg) => msg.clone(),
        UltraWMFatalError::PlatformError(e) => format!("{}", e),
        UltraWMFatalError::WMError(e) => format!("{}", e),
    };

    let _ = DialogBuilder::message().set_level(MessageLevel::Error).set_title(title).set_text(&message).alert().show();
}