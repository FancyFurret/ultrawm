use std::collections::HashSet;

use crate::config::ModMouseKeybind;
use crate::platform::{
    input_state::InputState,
    inteceptor::{InterceptionRequest, Interceptor},
    MouseButton, Position, WMEvent,
};
use log::error;
use winit::keyboard::KeyCode;

/// Tracks modifier keys and handles event interception
pub struct ModMouseKeybindTracker {
    interception_request: Option<InterceptionRequest>,
    active: bool,
    moved: bool,
    started: bool,
    keybind: ModMouseKeybind,
    included_buttons: HashSet<MouseButton>,
}

pub enum KeybindEvent {
    Start(Position),
    Drag(Position),
    End(Position),
    Cancel(),
}

impl ModMouseKeybindTracker {
    pub fn new(keybind: ModMouseKeybind) -> Self {
        Self {
            interception_request: None,
            active: false,
            moved: false,
            started: false,
            included_buttons: keybind
                .combos()
                .iter()
                .flat_map(|combo| combo.buttons().iter())
                .cloned()
                .collect(),
            keybind,
        }
    }

    pub fn handle_event(&mut self, event: &WMEvent) -> Option<KeybindEvent> {
        match event {
            WMEvent::KeyDown(_) | WMEvent::KeyUp(_) => {
                self.update_interception_state();

                let escape_pressed = InputState::key_pressed(&KeyCode::Escape);
                if escape_pressed && self.active {
                    self.active = false;
                    self.started = false;
                    return Some(KeybindEvent::Cancel());
                }
                None
            }
            WMEvent::MouseDown(pos, _) => {
                self.update_interception_state();
                self.update_keybind_state(event, pos)
            }
            WMEvent::MouseUp(pos, _) => {
                self.update_interception_state();
                self.update_keybind_state(event, pos)
            }
            WMEvent::MouseMoved(pos) => {
                if !self.active {
                    None
                } else {
                    self.moved = true;
                    if !self.started {
                        self.started = true;
                        Interceptor::set_handled(&self.included_buttons);
                        Some(KeybindEvent::Start(pos.clone()))
                    } else {
                        Some(KeybindEvent::Drag(pos.clone()))
                    }
                }
            }
            _ => None,
        }
    }

    fn update_interception_state(&mut self) {
        let escape_pressed = InputState::key_pressed(&KeyCode::Escape);
        let should_intercept = self.keybind.modifiers_match(
            &InputState::pressed_keys(),
            &InputState::pressed_mouse_buttons(),
        ) && !escape_pressed;

        if should_intercept && self.interception_request.is_none() {
            match Interceptor::request_interception(self.included_buttons.clone()) {
                Ok(request) => {
                    self.interception_request = Some(request);
                }
                Err(e) => {
                    error!("Failed to request mouse interception: {e}");
                }
            }
        } else if !should_intercept && self.interception_request.is_some() {
            self.interception_request = None;
        }
    }

    fn update_keybind_state(
        &mut self,
        event: &WMEvent,
        position: &Position,
    ) -> Option<KeybindEvent> {
        let active = InputState::binding_matches(&self.keybind);
        let any_mouse_down = InputState::mouse_button_pressed(&MouseButton::Left)
            || InputState::mouse_button_pressed(&MouseButton::Middle)
            || InputState::mouse_button_pressed(&MouseButton::Right);

        if active && !self.active && matches!(event, WMEvent::MouseDown(_, _)) {
            // If our binding now matches due to a mouse down, activate
            self.active = true;
            // Don't send Start event yet - wait for mouse movement
            None
        } else if self.active && !active && matches!(event, WMEvent::MouseDown(_, _)) {
            // If another button was pressed, cancel
            self.active = false;
            self.started = false;
            Some(KeybindEvent::Cancel())
        } else if self.active && !active && !any_mouse_down {
            // If all buttons have been released, end
            self.active = false;
            self.started = false;
            Some(KeybindEvent::End(position.clone()))
        } else {
            None
        }
    }

    pub fn mod_held(&self) -> bool {
        self.interception_request.is_some()
    }
}
