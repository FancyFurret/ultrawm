use crate::platform::MouseButtons;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Keybind {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub super_key: bool,
    pub lmb: bool,
    pub rmb: bool,
    pub mmb: bool,
}

impl Keybind {
    /// Parse a keybind string like "ctrl+shift+lmb" (case-insensitive, order-insensitive)
    pub fn parse(s: &str) -> Self {
        let mut keybind = Keybind::default();
        for part in s.split('+') {
            match part.trim().to_ascii_lowercase().as_str() {
                "ctrl" => keybind.ctrl = true,
                "shift" => keybind.shift = true,
                "alt" => keybind.alt = true,
                "super" | "win" | "cmd" => keybind.super_key = true,
                "lmb" => keybind.lmb = true,
                "rmb" => keybind.rmb = true,
                "mmb" => keybind.mmb = true,
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

impl ToString for Keybind {
    fn to_string(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("ctrl");
        }
        if self.shift {
            parts.push("shift");
        }
        if self.alt {
            parts.push("alt");
        }
        if self.super_key {
            parts.push("super");
        }
        if self.lmb {
            parts.push("lmb");
        }
        if self.rmb {
            parts.push("rmb");
        }
        if self.mmb {
            parts.push("mmb");
        }
        parts.join("+")
    }
}

pub trait KeybindListExt {
    fn matches_mouse(&self, buttons: &MouseButtons) -> bool;
}

impl KeybindListExt for [Keybind] {
    fn matches_mouse(&self, buttons: &MouseButtons) -> bool {
        let mut mouse_keybind = Keybind::default();
        if buttons.left {
            mouse_keybind.lmb = true;
        }
        if buttons.right {
            mouse_keybind.rmb = true;
        }
        if buttons.middle {
            mouse_keybind.mmb = true;
        }
        self.iter().any(|b| b == &mouse_keybind)
    }
}

impl KeybindListExt for Vec<Keybind> {
    fn matches_mouse(&self, buttons: &MouseButtons) -> bool {
        self.as_slice().matches_mouse(buttons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_mouse_buttons() {
        assert_eq!(
            Keybind::parse("lmb"),
            Keybind {
                lmb: true,
                ..Default::default()
            }
        );
        assert_eq!(
            Keybind::parse("rmb"),
            Keybind {
                rmb: true,
                ..Default::default()
            }
        );
        assert_eq!(
            Keybind::parse("mmb"),
            Keybind {
                mmb: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_modifiers() {
        assert_eq!(
            Keybind::parse("ctrl"),
            Keybind {
                ctrl: true,
                ..Default::default()
            }
        );
        assert_eq!(
            Keybind::parse("shift"),
            Keybind {
                shift: true,
                ..Default::default()
            }
        );
        assert_eq!(
            Keybind::parse("alt"),
            Keybind {
                alt: true,
                ..Default::default()
            }
        );
        assert_eq!(
            Keybind::parse("win"),
            Keybind {
                super_key: true,
                ..Default::default()
            }
        );
        assert_eq!(
            Keybind::parse("cmd"),
            Keybind {
                super_key: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_combinations() {
        assert_eq!(
            Keybind::parse("ctrl+shift+lmb"),
            Keybind {
                ctrl: true,
                shift: true,
                lmb: true,
                ..Default::default()
            }
        );
        assert_eq!(
            Keybind::parse("lmb+ctrl+alt"),
            Keybind {
                ctrl: true,
                alt: true,
                lmb: true,
                ..Default::default()
            }
        );
        assert_eq!(
            Keybind::parse("alt+super+mmb"),
            Keybind {
                alt: true,
                super_key: true,
                mmb: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_case_and_whitespace() {
        assert_eq!(
            Keybind::parse("CTRL+LMB"),
            Keybind {
                ctrl: true,
                lmb: true,
                ..Default::default()
            }
        );
        assert_eq!(
            Keybind::parse("  shift + RMB  "),
            Keybind {
                shift: true,
                rmb: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_unknown_ignored() {
        assert_eq!(
            Keybind::parse("foo+bar+lmb"),
            Keybind {
                lmb: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(Keybind::parse(""), Keybind::default());
    }
}
