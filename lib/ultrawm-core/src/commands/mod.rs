mod registry;

pub use registry::{
    build_commands, get_defaults, register, Command, CommandContext, CommandDef, CommandId,
};

use crate::ai::layout::{handle_organize_all_windows, handle_organize_single_window};
use crate::event_loop_wm::{WMOperationError, WMOperationResult};
use crate::platform::WindowId;
use crate::wm::WMError;
use log::info;

/// Helper to extract window_id from command context
fn get_window_id_from_context(ctx: Option<&CommandContext>) -> WMOperationResult<WindowId> {
    ctx.and_then(|c| c.target_window).ok_or_else(|| {
        info!("No target window provided for command");
        WMOperationError::Error(WMError::WindowNotFound(0))
    })
}

pub static AI_ORGANIZE_ALL_WINDOWS: CommandDef = CommandDef {
    display_name: "Auto Organize All Windows",
    id: "ai_organize_all_windows",
    default_keybind: "cmd+shift+o",
    requires_window: false,
    handler: |wm, _ctx| {
        info!("AI: Organizing all windows...");
        handle_organize_all_windows(wm)
    },
};

pub static AI_ORGANIZE_CURRENT_WINDOW: CommandDef = CommandDef {
    display_name: "Auto Organize Current Window",
    id: "ai_organize_current_window",
    default_keybind: "cmd+shift+i",
    requires_window: true,
    handler: |wm, ctx| {
        let window_id = get_window_id_from_context(ctx)?;
        handle_organize_single_window(wm, window_id)
    },
};

pub static FLOAT_WINDOW: CommandDef = CommandDef {
    display_name: "Float Window",
    id: "float_window",
    default_keybind: "",
    requires_window: true,
    handler: |wm, ctx| {
        let window_id = get_window_id_from_context(ctx)?;
        wm.float_window(window_id)?;
        Ok(())
    },
};

pub static CLOSE_WINDOW: CommandDef = CommandDef {
    display_name: "Close Window",
    id: "close_window",
    default_keybind: "",
    requires_window: true,
    handler: |wm, ctx| {
        let window_id = get_window_id_from_context(ctx)?;
        let window = wm.get_window(window_id)?;
        window.close().map_err(WMError::from)?;
        wm.remove_window(window_id)?;
        Ok(())
    },
};

pub static MINIMIZE_WINDOW: CommandDef = CommandDef {
    display_name: "Minimize Window",
    id: "minimize_window",
    default_keybind: "",
    requires_window: true,
    handler: |wm, ctx| {
        let window_id = get_window_id_from_context(ctx)?;
        let window = wm.get_window(window_id)?;
        window.minimize().map_err(WMError::from)?;
        Ok(())
    },
};

pub fn register_commands() {
    register(&AI_ORGANIZE_ALL_WINDOWS);
    register(&AI_ORGANIZE_CURRENT_WINDOW);
    register(&FLOAT_WINDOW);
    register(&CLOSE_WINDOW);
    register(&MINIMIZE_WINDOW);
}
