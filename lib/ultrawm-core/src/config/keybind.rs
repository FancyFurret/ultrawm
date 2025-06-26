use crate::config::InputCombo;
use crate::platform::{Keys, MouseButton, MouseButtons};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::marker::PhantomData;
use winit::keyboard::KeyCode;

#[derive(Debug, Clone, Default)]
pub struct Keybind<T: KeybindVariant> {
    combos: Vec<InputCombo>,
    _phantom: PhantomData<T>,
}
impl<T: KeybindVariant> Keybind<T> {
    pub fn matches_buttons(&self, buttons: &MouseButtons) -> bool {
        self.combos.iter().any(|b| b.buttons().matches(buttons))
    }

    pub fn matches_keys(&self, keys: &Keys) -> bool {
        self.combos.iter().any(|b| b.keys().matches(keys))
    }

    pub fn matches(&self, keys: &Keys, buttons: &MouseButtons) -> bool {
        self.combos
            .iter()
            .any(|b| b.keys().matches(keys) && b.buttons().matches(buttons))
    }

    pub fn modifiers_match(&self, keys: &Keys, buttons: &MouseButtons) -> bool {
        self.combos.iter().any(|combo| {
            combo.modifiers().iter().all(|modifier| match modifier {
                Modifier::Key(key) => keys.contains(key),
                Modifier::MouseButton(button) => buttons.contains(button),
            })
        })
    }
}

impl<T: KeybindVariant> Into<Keybind<T>> for Vec<&str> {
    fn into(self) -> Keybind<T> {
        let combos = self.into_iter().map(|s| InputCombo::parse(s)).collect();
        Keybind {
            combos,
            _phantom: PhantomData,
        }
    }
}

pub type MouseKeybind = Keybind<MouseKeybindVariant>;
pub type ModifiedMouseKeybind = Keybind<ModifiedMouseKeybindVariant>;

pub trait KeybindVariant: 'static {
    fn validate<E: serde::de::Error>(combo: &InputCombo) -> Result<(), E>
    where
        Self: Sized;
}

#[derive(Debug, Clone)]
pub struct MouseKeybindVariant;
impl KeybindVariant for MouseKeybindVariant {
    fn validate<E: serde::de::Error>(combo: &InputCombo) -> Result<(), E> {
        if !combo.buttons().any() {
            return Err(E::custom(
                "This keybind must contain at least one mouse button",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ModifiedMouseKeybindVariant;
impl KeybindVariant for ModifiedMouseKeybindVariant {
    fn validate<E: serde::de::Error>(combo: &InputCombo) -> Result<(), E> {
        if !combo.keys().any() {
            return Err(E::custom("This keybind must contain at least one key"));
        }

        if combo.modifiers().len() == 0 {
            return Err(E::custom("This keybind must contain at least one modifier"));
        }

        Ok(())
    }
}

pub enum Modifier {
    Key(KeyCode),
    MouseButton(MouseButton),
}

impl<T: KeybindVariant> Serialize for Keybind<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.combos.serialize(serializer)
    }
}

impl<'de, T: KeybindVariant> Deserialize<'de> for Keybind<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let combos: Vec<InputCombo> = Vec::deserialize(deserializer)?;

        // Validate each combo using the variant's validate method
        for combo in &combos {
            T::validate(combo)?;
        }

        Ok(Keybind {
            combos,
            _phantom: PhantomData,
        })
    }
}
