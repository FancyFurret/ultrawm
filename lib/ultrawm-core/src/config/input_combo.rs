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
    /// Parse a keybind string like "ctrl+shift+lmb" (case-insensitive, order-insensitive)
    pub fn parse(s: &str) -> Self {
        let mut keybind = InputCombo::default();
        for part in s.split('+') {
            match part.trim().to_ascii_lowercase().as_str() {
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
                _ => {}
            }
        }
        keybind
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
        if self.keys.contains(&KeyCode::ControlLeft) {
            parts.push("ctrl");
        }
        if self.keys.contains(&KeyCode::ShiftLeft) {
            parts.push("shift");
        }
        if self.keys.contains(&KeyCode::AltLeft) {
            parts.push("alt");
        }
        if self.keys.contains(&KeyCode::SuperLeft) {
            parts.push("super");
        }
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

impl Into<InputCombo> for &str {
    fn into(self) -> InputCombo {
        InputCombo::parse(self)
    }
}
