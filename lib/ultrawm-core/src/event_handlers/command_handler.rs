use crate::commands::{build_commands, Command, CommandContext, CommandId};
use crate::config::Config;
use crate::event_handlers::EventHandler;
use crate::event_loop_wm::WMOperationResult;
use crate::platform::WMEvent;
use crate::wm::WindowManager;

pub struct CommandHandler {
    commands: Vec<Command>,
}

impl CommandHandler {
    pub async fn new() -> Self {
        Self {
            commands: build_commands(&Config::current().commands.keybinds),
        }
    }

    pub fn execute_command(
        &self,
        command_id: &CommandId,
        wm: &mut WindowManager,
        context: Option<&CommandContext>,
    ) -> WMOperationResult<bool> {
        for command in &self.commands {
            if &command.id == command_id {
                (command.handler)(wm, context)?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl EventHandler for CommandHandler {
    fn handle_event(&mut self, event: &WMEvent, wm: &mut WindowManager) -> WMOperationResult<bool> {
        match event {
            WMEvent::KeyDown(_) | WMEvent::KeyUp(_) => {
                for command in &mut self.commands {
                    if command.tracker.was_just_pressed() {
                        (command.handler)(wm, None)?;
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            WMEvent::CommandTriggered(command_id, context) => {
                self.execute_command(command_id, wm, context.as_ref())
            }
            WMEvent::ConfigChanged => {
                self.commands = build_commands(&Config::current().commands.keybinds);
                Ok(false)
            }
            _ => Ok(false),
        }
    }
}
