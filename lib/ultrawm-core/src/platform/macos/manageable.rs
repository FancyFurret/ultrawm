use crate::platform::macos::ffi::{get_window_id, AXUIElementExt};
use crate::platform::{PlatformError, PlatformResult};
use application_services::AXError;

pub fn app_is_manageable(app: &AXUIElementExt) -> ObserveResult {
    app.pid().map_err(|_| "App has no pid")?;
    app.title().map_err(|_| "App has no title")?;

    Ok(())
}

pub fn window_is_manageable(window: &AXUIElementExt) -> ObserveResult {
    get_window_id(&window.element).ok_or("Window has no id")?;
    window.title().map_err(|_| "Window has no title")?;

    let role = window.role().map_err(|_| "Window has no role")?;
    if role == "AXPopover" {
        Err("Window role is AXPopover")?
    }

    let subrole = window.subrole().map_err(|_| "Window has no subrole")?;
    if subrole == "AXUnknown" {
        Err("Window subrole is AXUnknown")?
    }

    if subrole == "AXDialog" {
        Err("Window subrole is AXDialog")?
    }

    Ok(())
}

#[derive(Debug)]
pub enum ObserveError {
    NotManageable(String),
    PlatformError(PlatformError),
}

pub type ObserveResult = Result<(), ObserveError>;
pub trait ObserveResultExt {
    fn handle_observe_error(self) -> PlatformResult<()>;
}

impl ObserveResultExt for ObserveResult {
    fn handle_observe_error(self) -> PlatformResult<()> {
        match self {
            Ok(_) => Ok(()),
            Err(ObserveError::NotManageable(_e)) => Ok(()),
            Err(ObserveError::PlatformError(e)) => Err(e.into()),
        }
    }
}

impl From<PlatformError> for ObserveError {
    fn from(error: PlatformError) -> Self {
        ObserveError::PlatformError(error)
    }
}

impl From<&str> for ObserveError {
    fn from(error: &str) -> Self {
        ObserveError::NotManageable(error.to_string())
    }
}

impl From<()> for ObserveError {
    fn from(_: ()) -> Self {
        ObserveError::PlatformError(PlatformError::Unknown.into())
    }
}

impl From<AXError> for ObserveError {
    fn from(error: AXError) -> Self {
        ObserveError::PlatformError(error.into())
    }
}

impl Into<PlatformError> for ObserveError {
    fn into(self) -> PlatformError {
        match self {
            ObserveError::NotManageable(e) => PlatformError::Error(e).into(),
            ObserveError::PlatformError(e) => e,
        }
    }
}
