mod registry;

pub use registry::{
    build_commands, get_defaults, register, Command, CommandContext, CommandDef, CommandId,
};

use crate::ai::layout::{handle_organize_all_windows, handle_organize_single_window};
use log::info;

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
        let window_id = match ctx {
            Some(ctx) => ctx.target_window,
            None => {
                info!("AI: Organize current window - no context provided");
                return Ok(());
            }
        };

        let window_id = match window_id {
            Some(id) => id,
            None => {
                info!("AI: Organize current window - no target window provided");
                return Ok(());
            }
        };

        handle_organize_single_window(wm, window_id)
    },
};

pub fn register_commands() {
    register(&AI_ORGANIZE_ALL_WINDOWS);
    register(&AI_ORGANIZE_CURRENT_WINDOW);
}
