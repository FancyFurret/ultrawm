use crate::config::KeyboardKeybind;
use crate::platform::input_state::InputState;

/// Tracks keyboard keybinds and detects when they are pressed
pub struct KeyboardKeybindTracker {
    keybind: KeyboardKeybind,
    was_pressed: bool,
}

impl KeyboardKeybindTracker {
    pub fn new(keybind: KeyboardKeybind) -> Self {
        Self {
            keybind,
            was_pressed: false,
        }
    }

    /// Check if the keybind is currently pressed
    pub fn is_pressed(&self) -> bool {
        InputState::binding_matches(&self.keybind)
    }

    /// Check if the keybind was just pressed (transition from not pressed to pressed)
    pub fn was_just_pressed(&mut self) -> bool {
        let currently_pressed = self.is_pressed();
        let just_pressed = currently_pressed && !self.was_pressed;
        self.was_pressed = currently_pressed;
        just_pressed
    }

    pub fn update(&mut self) {
        self.was_pressed = self.is_pressed();
    }
}

