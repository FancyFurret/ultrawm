use crate::event_handlers::command_registry::{register, CommandDef};
use log::info;

pub static AI_ORGANIZE_ALL_WINDOWS: CommandDef = CommandDef {
    display_name: "AI Organize All Windows",
    id: "ai_organize_all_windows",
    default_keybind: "cmd+shift+o",
    requires_window: false,
    handler: |_wm, _ctx| {
        info!("AI: Organize all windows (not yet implemented)");
        Ok(())
    },
};

pub static AI_ORGANIZE_CURRENT_WINDOW: CommandDef = CommandDef {
    display_name: "AI Organize Current Window",
    id: "ai_organize_current_window",
    default_keybind: "cmd+shift+i",
    requires_window: true,
    handler: |_wm, ctx| {
        if let Some(ctx) = ctx {
            if let Some(window_id) = ctx.target_window {
                info!("AI: Organize window {} (not yet implemented)", window_id);
            } else {
                info!("AI: Organize current window - no target window provided");
            }
        } else {
            info!("AI: Organize current window (not yet implemented)");
        }
        Ok(())
    },
};

pub fn register_commands() {
    register(&AI_ORGANIZE_ALL_WINDOWS);
    register(&AI_ORGANIZE_CURRENT_WINDOW);
}
