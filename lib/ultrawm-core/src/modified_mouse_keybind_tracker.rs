use crate::config::ModifiedMouseKeybind;
use crate::platform::{
    inteceptor::{InterceptionRequest, Interceptor},
    Keys, MouseButton, MouseButtons, PlatformEvent, Position,
};
use log::error;

/// Tracks modifier keys and handles event interception
pub struct ModifiedMouseKeybindTracker {
    current_keys: Keys,
    current_buttons: MouseButtons,
    pressed_buttons: Vec<MouseButton>,
    interception_request: Option<InterceptionRequest>,
    active: bool,
    moved: bool,
    started: bool,
    keybind: ModifiedMouseKeybind,
}

pub enum KeybindEvent {
    Start(Position),
    Drag(Position),
    End(Position),
}

impl ModifiedMouseKeybindTracker {
    pub fn new(keybind: ModifiedMouseKeybind) -> Self {
        Self {
            current_keys: Keys::new(),
            current_buttons: MouseButtons::new(),
            pressed_buttons: Vec::new(),
            interception_request: None,
            active: false,
            moved: false,
            started: false,
            keybind,
        }
    }

    pub fn handle_event(&mut self, event: &PlatformEvent) -> Option<KeybindEvent> {
        match event {
            PlatformEvent::KeyDown(key) => {
                self.current_keys.add(key);
                self.update_interception_state();
                None
            }
            PlatformEvent::KeyUp(key) => {
                self.current_keys.remove(key);
                self.update_interception_state();
                None
            }
            PlatformEvent::MouseDown(pos, button) => {
                self.current_buttons.add(button);
                self.update_interception_state();
                self.update_keybind_state(pos)
            }
            PlatformEvent::MouseUp(pos, button) => {
                if self.active && !self.moved {
                    self.pressed_buttons.push(button.clone());
                }

                self.current_buttons.remove(button);
                self.update_interception_state();
                self.update_keybind_state(pos)
            }
            PlatformEvent::MouseMoved(pos) => {
                if !self.active {
                    None
                } else {
                    self.moved = true;
                    if !self.started {
                        self.started = true;
                        Interceptor::set_handled();
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
        let should_intercept = self
            .keybind
            .modifiers_match(&self.current_keys, &self.current_buttons);

        if should_intercept && self.interception_request.is_none() {
            match Interceptor::request_interception() {
                Ok(request) => {
                    self.interception_request = Some(request);
                }
                Err(e) => {
                    error!("Failed to request mouse interception: {e}");
                }
            }
        } else if !should_intercept && self.interception_request.is_some() {
            self.interception_request = None;
            self.pressed_buttons.clear();
        }
    }

    fn update_keybind_state(&mut self, position: &Position) -> Option<KeybindEvent> {
        let active = self
            .keybind
            .matches(&self.current_keys, &self.current_buttons);
        if active && !self.active {
            self.active = true;
            // Don't send Start event yet - wait for mouse movement
            None
        } else if !active && self.active {
            self.active = false;
            self.started = false; // Reset for next time
            Some(KeybindEvent::End(position.clone()))
        } else {
            None
        }
    }
}
