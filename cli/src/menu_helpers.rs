use crate::accelerator::keybind_to_accelerator;
use muda::accelerator::Accelerator;
use ultrawm_core::{CommandDef, Config};

pub fn get_command_accelerator(cmd: &'static CommandDef) -> Option<Accelerator> {
    let config = Config::current();
    let keybind = config
        .commands
        .keybinds
        .get(cmd.id)
        .cloned()
        .unwrap_or_else(|| vec![cmd.default_keybind].into());

    keybind_to_accelerator(&keybind)
}
