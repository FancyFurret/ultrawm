use crate::config::{Keybind, KeybindVariant};
use crate::platform::{Keys, MouseButton, MouseButtons, WMEvent};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use winit::keyboard::KeyCode;

static MOUSE_BUTTON_STATES: LazyLock<Mutex<HashMap<MouseButton, ButtonState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static KEY_STATES: LazyLock<Mutex<HashMap<KeyCode, ButtonState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonState {
    /// Button/key was just pressed (first frame)
    Down,
    /// Button/key is being held (subsequent frames)
    Held,
    /// Button/key was just released (first frame)
    Up,
    /// Button/key is not pressed
    Released,
}

impl ButtonState {
    pub fn is_down(&self) -> bool {
        matches!(self, ButtonState::Down)
    }

    pub fn is_held(&self) -> bool {
        matches!(self, ButtonState::Held)
    }

    pub fn is_up(&self) -> bool {
        matches!(self, ButtonState::Up)
    }

    pub fn is_pressed(&self) -> bool {
        matches!(self, ButtonState::Down | ButtonState::Held)
    }

    pub fn is_released(&self) -> bool {
        matches!(self, ButtonState::Up | ButtonState::Released)
    }
}

pub struct InputState;

impl InputState {
    pub fn initialize() {
        let _ = MOUSE_BUTTON_STATES.lock().map(|mut states| {
            states.clear();
        });
        let _ = KEY_STATES.lock().map(|mut states| {
            states.clear();
        });
    }

    pub fn handle_event(event: &WMEvent) {
        Self::update_states();

        match event {
            WMEvent::MouseDown(_pos, button) => {
                Self::set_mouse_button_state(button, true);
            }
            WMEvent::MouseUp(_pos, button) => {
                Self::set_mouse_button_state(button, false);
            }
            WMEvent::KeyDown(key) => {
                Self::set_key_state(key, true);
            }
            WMEvent::KeyUp(key) => {
                Self::set_key_state(key, false);
            }
            _ => {}
        }
    }

    fn set_mouse_button_state(button: &MouseButton, pressed: bool) {
        if let Ok(mut states) = MOUSE_BUTTON_STATES.lock() {
            if pressed {
                states.insert(button.clone(), ButtonState::Down);
            } else {
                states.insert(button.clone(), ButtonState::Up);
            }
        }
    }

    fn set_key_state(key: &KeyCode, pressed: bool) {
        if let Ok(mut states) = KEY_STATES.lock() {
            if pressed {
                states.insert(key.clone(), ButtonState::Down);
            } else {
                states.insert(key.clone(), ButtonState::Up);
            }
        }
    }

    pub fn mouse_button_down(button: &MouseButton) -> bool {
        MOUSE_BUTTON_STATES
            .lock()
            .map(|states| states.get(button).map(|s| s.is_down()).unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn mouse_button_held(button: &MouseButton) -> bool {
        MOUSE_BUTTON_STATES
            .lock()
            .map(|states| states.get(button).map(|s| s.is_held()).unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn mouse_button_up(button: &MouseButton) -> bool {
        MOUSE_BUTTON_STATES
            .lock()
            .map(|states| states.get(button).map(|s| s.is_up()).unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn mouse_button_pressed(button: &MouseButton) -> bool {
        MOUSE_BUTTON_STATES
            .lock()
            .map(|states| states.get(button).map(|s| s.is_pressed()).unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn key_down(key: &KeyCode) -> bool {
        KEY_STATES
            .lock()
            .map(|states| states.get(key).map(|s| s.is_down()).unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn key_held(key: &KeyCode) -> bool {
        KEY_STATES
            .lock()
            .map(|states| states.get(key).map(|s| s.is_held()).unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn key_up(key: &KeyCode) -> bool {
        KEY_STATES
            .lock()
            .map(|states| states.get(key).map(|s| s.is_up()).unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn key_pressed(key: &KeyCode) -> bool {
        KEY_STATES
            .lock()
            .map(|states| states.get(key).map(|s| s.is_pressed()).unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn binding_matches_mouse<T: KeybindVariant>(keybind: &Keybind<T>) -> bool {
        keybind.matches_buttons(&Self::pressed_mouse_buttons())
    }

    pub fn binding_matches_key<T: KeybindVariant>(keybind: &Keybind<T>) -> bool {
        keybind.matches_keys(&Self::pressed_keys())
    }

    pub fn binding_matches<T: KeybindVariant>(keybind: &Keybind<T>) -> bool {
        keybind.matches(&Self::pressed_keys(), &Self::pressed_mouse_buttons())
    }

    pub fn pressed_mouse_buttons() -> MouseButtons {
        MOUSE_BUTTON_STATES
            .lock()
            .map(|states| {
                states
                    .iter()
                    .filter(|(_, state)| state.is_pressed())
                    .map(|(button, _)| button.clone())
                    .collect()
            })
            .unwrap_or(MouseButtons::new())
    }

    pub fn pressed_keys() -> Keys {
        KEY_STATES
            .lock()
            .map(|states| {
                states
                    .iter()
                    .filter(|(_, state)| state.is_pressed())
                    .map(|(key, _)| key.clone())
                    .collect()
            })
            .unwrap_or(Keys::new())
    }

    fn update_states() {
        // Update mouse button states
        if let Ok(mut states) = MOUSE_BUTTON_STATES.lock() {
            for state in states.values_mut() {
                match state {
                    ButtonState::Down => *state = ButtonState::Held,
                    ButtonState::Up => *state = ButtonState::Released,
                    _ => {}
                }
            }
        }

        // Update key states
        if let Ok(mut states) = KEY_STATES.lock() {
            for state in states.values_mut() {
                match state {
                    ButtonState::Down => *state = ButtonState::Held,
                    ButtonState::Up => *state = ButtonState::Released,
                    _ => {}
                }
            }
        }
    }
}
