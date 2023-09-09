use crate::platform::{create_platform, PlatformError, PlatformTrait, PlatformWindowTrait};

mod platform;

#[derive(Debug)]
pub enum UltraWMFatalError {
    Error(String),
    PlatformError(PlatformError),
}

pub type UltraWMResult<T> = Result<T, UltraWMFatalError>;

impl From<PlatformError> for UltraWMFatalError {
    fn from(error: PlatformError) -> Self {
        UltraWMFatalError::PlatformError(error)
    }
}

impl From<String> for UltraWMFatalError {
    fn from(error: String) -> Self {
        UltraWMFatalError::Error(error)
    }
}

pub fn start() -> UltraWMResult<()> {
    let platform = create_platform();
    let windows = platform
        .list_all_windows()
        .map_err(|e| format!("Could not list windows: {:?}", e))?;

    println!("Found {} windows", windows.len());

    for window in windows.iter() {
        println!(
            "[{}] {} - ({}, {}) ({}x{})",
            window.id()?,
            window.title()?,
            window.position()?.x,
            window.position()?.y,
            window.size()?.width,
            window.size()?.height
        );
        // let size = window.size()?;
        // window.resize(size.width - 30, size.height - 30)?;
    }

    Ok(())
}
