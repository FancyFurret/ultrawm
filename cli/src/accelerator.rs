use muda::accelerator::{Accelerator, Code, Modifiers};
use winit::keyboard::KeyCode;

pub fn keybind_to_accelerator(keybind: &ultrawm_core::config::KeyboardKeybind) -> Option<Accelerator> {
    let combo = keybind.combos().first()?;

    let mut modifiers = Modifiers::empty();
    let mut key_code = None;

    for key in combo.keys().iter() {
        match key {
            KeyCode::ControlLeft | KeyCode::ControlRight => {
                modifiers |= Modifiers::CONTROL;
            }
            KeyCode::ShiftLeft | KeyCode::ShiftRight => {
                modifiers |= Modifiers::SHIFT;
            }
            KeyCode::AltLeft | KeyCode::AltRight => {
                modifiers |= Modifiers::ALT;
            }
            KeyCode::SuperLeft | KeyCode::SuperRight => {
                modifiers |= Modifiers::SUPER;
            }
            _ => {
                if key_code.is_none() {
                    key_code = winit_to_accelerator_code(key);
                }
            }
        }
    }

    key_code.map(|code| Accelerator::new(Some(modifiers), code))
}

fn winit_to_accelerator_code(key: &KeyCode) -> Option<Code> {
    match key {
        KeyCode::KeyA => Some(Code::KeyA),
        KeyCode::KeyB => Some(Code::KeyB),
        KeyCode::KeyC => Some(Code::KeyC),
        KeyCode::KeyD => Some(Code::KeyD),
        KeyCode::KeyE => Some(Code::KeyE),
        KeyCode::KeyF => Some(Code::KeyF),
        KeyCode::KeyG => Some(Code::KeyG),
        KeyCode::KeyH => Some(Code::KeyH),
        KeyCode::KeyI => Some(Code::KeyI),
        KeyCode::KeyJ => Some(Code::KeyJ),
        KeyCode::KeyK => Some(Code::KeyK),
        KeyCode::KeyL => Some(Code::KeyL),
        KeyCode::KeyM => Some(Code::KeyM),
        KeyCode::KeyN => Some(Code::KeyN),
        KeyCode::KeyO => Some(Code::KeyO),
        KeyCode::KeyP => Some(Code::KeyP),
        KeyCode::KeyQ => Some(Code::KeyQ),
        KeyCode::KeyR => Some(Code::KeyR),
        KeyCode::KeyS => Some(Code::KeyS),
        KeyCode::KeyT => Some(Code::KeyT),
        KeyCode::KeyU => Some(Code::KeyU),
        KeyCode::KeyV => Some(Code::KeyV),
        KeyCode::KeyW => Some(Code::KeyW),
        KeyCode::KeyX => Some(Code::KeyX),
        KeyCode::KeyY => Some(Code::KeyY),
        KeyCode::KeyZ => Some(Code::KeyZ),
        KeyCode::Digit0 => Some(Code::Digit0),
        KeyCode::Digit1 => Some(Code::Digit1),
        KeyCode::Digit2 => Some(Code::Digit2),
        KeyCode::Digit3 => Some(Code::Digit3),
        KeyCode::Digit4 => Some(Code::Digit4),
        KeyCode::Digit5 => Some(Code::Digit5),
        KeyCode::Digit6 => Some(Code::Digit6),
        KeyCode::Digit7 => Some(Code::Digit7),
        KeyCode::Digit8 => Some(Code::Digit8),
        KeyCode::Digit9 => Some(Code::Digit9),
        KeyCode::Space => Some(Code::Space),
        KeyCode::Enter => Some(Code::Enter),
        KeyCode::Tab => Some(Code::Tab),
        KeyCode::Escape => Some(Code::Escape),
        KeyCode::Backspace => Some(Code::Backspace),
        KeyCode::Delete => Some(Code::Delete),
        KeyCode::ArrowUp => Some(Code::ArrowUp),
        KeyCode::ArrowDown => Some(Code::ArrowDown),
        KeyCode::ArrowLeft => Some(Code::ArrowLeft),
        KeyCode::ArrowRight => Some(Code::ArrowRight),
        KeyCode::F1 => Some(Code::F1),
        KeyCode::F2 => Some(Code::F2),
        KeyCode::F3 => Some(Code::F3),
        KeyCode::F4 => Some(Code::F4),
        KeyCode::F5 => Some(Code::F5),
        KeyCode::F6 => Some(Code::F6),
        KeyCode::F7 => Some(Code::F7),
        KeyCode::F8 => Some(Code::F8),
        KeyCode::F9 => Some(Code::F9),
        KeyCode::F10 => Some(Code::F10),
        KeyCode::F11 => Some(Code::F11),
        KeyCode::F12 => Some(Code::F12),
        _ => None,
    }
}

