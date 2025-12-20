use crate::config::Modifier;
use crate::platform::{Keys, MouseButton, MouseButtons};
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::fmt::Display;
use winit::keyboard::KeyCode;

#[derive(Debug, Clone, Default)]
pub struct InputCombo {
    keys: Keys,
    buttons: MouseButtons,
}

impl InputCombo {
    pub fn parse(s: &str) -> Self {
        let mut keybind = InputCombo::default();
        for part in s.split('+') {
            let part_lower = part.trim().to_ascii_lowercase();
            match part_lower.as_str() {
                "ctrl" => keybind.keys.add(&KeyCode::ControlLeft),
                "shift" => keybind.keys.add(&KeyCode::ShiftLeft),
                "alt" => keybind.keys.add(&KeyCode::AltLeft),
                "super" | "win" | "cmd" => keybind.keys.add(&KeyCode::SuperLeft),
                "lmb" => keybind.buttons.add(&MouseButton::Left),
                "rmb" => keybind.buttons.add(&MouseButton::Right),
                "mmb" => keybind.buttons.add(&MouseButton::Middle),
                "bmb" | "button4" | "back" => keybind.buttons.add(&MouseButton::Button4),
                "fmb" | "button5" | "forward" => keybind.buttons.add(&MouseButton::Button5),
                "" => {}
                _ => {
                    if let Some(keycode) = Self::parse_key(&part_lower) {
                        keybind.keys.add(&keycode);
                    }
                }
            }
        }
        keybind
    }

    fn parse_key(s: &str) -> Option<KeyCode> {
        use winit::keyboard::KeyCode::*;
        match s {
            "a" => Some(KeyA),
            "b" => Some(KeyB),
            "c" => Some(KeyC),
            "d" => Some(KeyD),
            "e" => Some(KeyE),
            "f" => Some(KeyF),
            "g" => Some(KeyG),
            "h" => Some(KeyH),
            "i" => Some(KeyI),
            "j" => Some(KeyJ),
            "k" => Some(KeyK),
            "l" => Some(KeyL),
            "m" => Some(KeyM),
            "n" => Some(KeyN),
            "o" => Some(KeyO),
            "p" => Some(KeyP),
            "q" => Some(KeyQ),
            "r" => Some(KeyR),
            "s" => Some(KeyS),
            "t" => Some(KeyT),
            "u" => Some(KeyU),
            "v" => Some(KeyV),
            "w" => Some(KeyW),
            "x" => Some(KeyX),
            "y" => Some(KeyY),
            "z" => Some(KeyZ),
            "0" => Some(Digit0),
            "1" => Some(Digit1),
            "2" => Some(Digit2),
            "3" => Some(Digit3),
            "4" => Some(Digit4),
            "5" => Some(Digit5),
            "6" => Some(Digit6),
            "7" => Some(Digit7),
            "8" => Some(Digit8),
            "9" => Some(Digit9),
            "space" => Some(Space),
            "enter" | "return" => Some(Enter),
            "tab" => Some(Tab),
            "escape" | "esc" => Some(Escape),
            "backspace" => Some(Backspace),
            "delete" => Some(Delete),
            "up" | "arrowup" => Some(ArrowUp),
            "down" | "arrowdown" => Some(ArrowDown),
            "left" | "arrowleft" => Some(ArrowLeft),
            "right" | "arrowright" => Some(ArrowRight),
            "f1" => Some(F1),
            "f2" => Some(F2),
            "f3" => Some(F3),
            "f4" => Some(F4),
            "f5" => Some(F5),
            "f6" => Some(F6),
            "f7" => Some(F7),
            "f8" => Some(F8),
            "f9" => Some(F9),
            "f10" => Some(F10),
            "f11" => Some(F11),
            "f12" => Some(F12),
            _ => None,
        }
    }

    pub fn keys(&self) -> &Keys {
        &self.keys
    }

    pub fn buttons(&self) -> &MouseButtons {
        &self.buttons
    }

    pub fn modifiers(&self) -> Vec<Modifier> {
        let mut modifiers = Vec::new();
        for key in self.keys.iter() {
            if matches!(
                key,
                KeyCode::ControlLeft
                    | KeyCode::ControlRight
                    | KeyCode::ShiftLeft
                    | KeyCode::ShiftRight
                    | KeyCode::AltLeft
                    | KeyCode::AltRight
                    | KeyCode::SuperLeft
                    | KeyCode::SuperRight
            ) {
                modifiers.push(Modifier::Key(key.clone()));
            }
        }
        for button in self.buttons.iter() {
            if matches!(button, MouseButton::Button4 | MouseButton::Button5) {
                modifiers.push(Modifier::MouseButton(button.clone()));
            }
        }
        modifiers
    }
}

impl Serialize for InputCombo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for InputCombo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeybindVisitor;
        impl<'de> Visitor<'de> for KeybindVisitor {
            type Value = InputCombo;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a keybind string like 'ctrl+shift+lmb'")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(InputCombo::parse(v))
            }
        }
        deserializer.deserialize_str(KeybindVisitor)
    }
}

impl Display for InputCombo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        use winit::keyboard::KeyCode::*;

        // Modifiers first (in standard order)
        if self.keys.contains(&ControlLeft) {
            parts.push("ctrl");
        }
        if self.keys.contains(&ShiftLeft) {
            parts.push("shift");
        }
        if self.keys.contains(&AltLeft) {
            parts.push("alt");
        }
        if self.keys.contains(&SuperLeft) {
            parts.push("cmd");
        }

        // Regular keys
        for key in self.keys.iter() {
            if !matches!(key, ControlLeft | ShiftLeft | AltLeft | SuperLeft) {
                if let Some(key_str) = key_to_string(key) {
                    parts.push(key_str);
                }
            }
        }

        // Mouse buttons
        if self.buttons.contains(&MouseButton::Left) {
            parts.push("lmb");
        }
        if self.buttons.contains(&MouseButton::Right) {
            parts.push("rmb");
        }
        if self.buttons.contains(&MouseButton::Middle) {
            parts.push("mmb");
        }
        if self.buttons.contains(&MouseButton::Button4) {
            parts.push("back");
        }
        if self.buttons.contains(&MouseButton::Button5) {
            parts.push("forward");
        }
        write!(f, "{}", parts.join("+"))
    }
}

fn key_to_string(key: &KeyCode) -> Option<&'static str> {
    use winit::keyboard::KeyCode::*;
    match key {
        KeyA => Some("a"),
        KeyB => Some("b"),
        KeyC => Some("c"),
        KeyD => Some("d"),
        KeyE => Some("e"),
        KeyF => Some("f"),
        KeyG => Some("g"),
        KeyH => Some("h"),
        KeyI => Some("i"),
        KeyJ => Some("j"),
        KeyK => Some("k"),
        KeyL => Some("l"),
        KeyM => Some("m"),
        KeyN => Some("n"),
        KeyO => Some("o"),
        KeyP => Some("p"),
        KeyQ => Some("q"),
        KeyR => Some("r"),
        KeyS => Some("s"),
        KeyT => Some("t"),
        KeyU => Some("u"),
        KeyV => Some("v"),
        KeyW => Some("w"),
        KeyX => Some("x"),
        KeyY => Some("y"),
        KeyZ => Some("z"),
        Digit0 => Some("0"),
        Digit1 => Some("1"),
        Digit2 => Some("2"),
        Digit3 => Some("3"),
        Digit4 => Some("4"),
        Digit5 => Some("5"),
        Digit6 => Some("6"),
        Digit7 => Some("7"),
        Digit8 => Some("8"),
        Digit9 => Some("9"),
        Space => Some("space"),
        Enter => Some("enter"),
        Tab => Some("tab"),
        Escape => Some("escape"),
        Backspace => Some("backspace"),
        Delete => Some("delete"),
        ArrowUp => Some("up"),
        ArrowDown => Some("down"),
        ArrowLeft => Some("left"),
        ArrowRight => Some("right"),
        F1 => Some("f1"),
        F2 => Some("f2"),
        F3 => Some("f3"),
        F4 => Some("f4"),
        F5 => Some("f5"),
        F6 => Some("f6"),
        F7 => Some("f7"),
        F8 => Some("f8"),
        F9 => Some("f9"),
        F10 => Some("f10"),
        F11 => Some("f11"),
        F12 => Some("f12"),
        _ => None,
    }
}

impl Into<InputCombo> for &str {
    fn into(self) -> InputCombo {
        InputCombo::parse(self)
    }
}
