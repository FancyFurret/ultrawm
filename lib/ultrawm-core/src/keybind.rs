use crate::platform::{Keys, MouseButton, MouseButtons};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::fmt::Display;
use winit::keyboard::KeyCode;

#[derive(Debug, Clone, Default)]
pub struct Keybind {
    pub keys: Keys,
    pub mouse: MouseButtons,
}

impl Keybind {
    /// Parse a keybind string like "ctrl+shift+lmb" (case-insensitive, order-insensitive)
    pub fn parse(s: &str) -> Self {
        let mut keybind = Keybind::default();
        for part in s.split('+') {
            match part.trim().to_ascii_lowercase().as_str() {
                "ctrl" => keybind.keys.add(&KeyCode::ControlLeft),
                "shift" => keybind.keys.add(&KeyCode::ShiftLeft),
                "alt" => keybind.keys.add(&KeyCode::AltLeft),
                "super" | "win" | "cmd" => keybind.keys.add(&KeyCode::SuperLeft),
                "lmb" => keybind.mouse.add(&MouseButton::Left),
                "rmb" => keybind.mouse.add(&MouseButton::Right),
                "mmb" => keybind.mouse.add(&MouseButton::Middle),
                "" => {}
                _ => {}
            }
        }
        keybind
    }
}

impl Serialize for Keybind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Keybind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeybindVisitor;
        impl<'de> Visitor<'de> for KeybindVisitor {
            type Value = Keybind;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a keybind string like 'ctrl+shift+lmb'")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Keybind::parse(v))
            }
        }
        deserializer.deserialize_str(KeybindVisitor)
    }
}

impl Display for Keybind {
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
        if self.mouse.contains(&MouseButton::Left) {
            parts.push("lmb");
        }
        if self.mouse.contains(&MouseButton::Right) {
            parts.push("rmb");
        }
        if self.mouse.contains(&MouseButton::Middle) {
            parts.push("mmb");
        }
        write!(f, "{}", parts.join("+"))
    }
}

pub trait KeybindListExt {
    fn matches_mouse(&self, buttons: &MouseButtons) -> bool;
    fn matches_keys(&self, keys: &Keys) -> bool;
    fn matches(&self, keys: &Keys, buttons: &MouseButtons) -> bool {
        self.matches_mouse(buttons) && self.matches_keys(keys)
    }
}

impl KeybindListExt for [Keybind] {
    fn matches_mouse(&self, buttons: &MouseButtons) -> bool {
        self.iter().any(|b| b.mouse.contains_all(buttons))
    }

    fn matches_keys(&self, keys: &Keys) -> bool {
        self.iter().any(|b| b.keys.contains_all(keys))
    }
}

impl KeybindListExt for Vec<Keybind> {
    fn matches_mouse(&self, buttons: &MouseButtons) -> bool {
        self.as_slice().matches_mouse(buttons)
    }

    fn matches_keys(&self, keys: &Keys) -> bool {
        self.as_slice().matches_keys(keys)
    }
}
