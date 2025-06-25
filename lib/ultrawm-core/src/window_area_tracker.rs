use crate::config::WindowAreaBindings;
use crate::keybind::KeybindListExt;
use crate::platform::{Bounds, Keys, MouseButtons, PlatformEvent, Position, WindowId};
use crate::window::WindowRef;
use crate::wm::WindowManager;
use crate::Config;
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq)]
pub enum WindowAreaDragEvent {
    Start(WindowId, Position, WindowAreaDragType),
    Drag(WindowId, Position, WindowAreaDragType),
    End(WindowId, Position, WindowAreaDragType),
}

#[derive(Debug, Clone, PartialEq)]
pub enum WindowAreaDragType {
    Resize,
    ResizeSymmetric,
    Slide,
    Tile,
}

#[derive(Debug, Clone)]
struct DragContext {
    window: WindowRef,
    start_position: Position,
    start_bounds: Bounds,
    drag_type: WindowAreaDragType,
}

pub struct WindowAreaTracker {
    current_keys: Keys,
    current_buttons: MouseButtons,
    bindings: WindowAreaBindings,
    current_drag: Option<DragContext>,
}

impl WindowAreaTracker {
    pub fn new() -> Self {
        let config = Config::current();
        Self {
            current_keys: Keys::new(),
            current_buttons: MouseButtons::new(),
            bindings: config.window_area_bindings.clone(),
            current_drag: None,
        }
    }

    pub fn handle_event(
        &mut self,
        event: &PlatformEvent,
        wm: &WindowManager,
    ) -> Option<WindowAreaDragEvent> {
        match event {
            PlatformEvent::KeyDown(key) => {
                self.current_keys.add(key);
                self.cancel_if_no_binding();
            }
            PlatformEvent::KeyUp(key) => {
                self.current_keys.remove(key);
                self.cancel_if_no_binding();
            }
            PlatformEvent::MouseDown(pos, button) => {
                self.current_buttons.add(button);
                if self.current_drag.is_none() {
                    if let Some((window, drag_type)) = self.get_drag_type(wm, pos) {
                        let start_bounds = window.bounds().clone();
                        self.current_drag = Some(DragContext {
                            window: window.clone(),
                            start_position: pos.clone(),
                            start_bounds,
                            drag_type: drag_type.clone(),
                        });
                        return Some(WindowAreaDragEvent::Start(
                            window.id(),
                            pos.clone(),
                            drag_type,
                        ));
                    }
                }
            }
            PlatformEvent::MouseMoved(pos) => {
                if let Some(drag) = &self.current_drag {
                    return Some(WindowAreaDragEvent::Drag(
                        drag.window.id(),
                        pos.clone(),
                        drag.drag_type.clone(),
                    ));
                }
            }
            PlatformEvent::MouseUp(pos, button) => {
                self.current_buttons.remove(button);
                if let Some(drag) = self.current_drag.take() {
                    return Some(WindowAreaDragEvent::End(
                        drag.window.id(),
                        pos.clone(),
                        drag.drag_type,
                    ));
                }
            }
            _ => {}
        }
        None
    }

    fn get_drag_type(
        &self,
        wm: &WindowManager,
        pos: &Position,
    ) -> Option<(WindowRef, WindowAreaDragType)> {
        let window = wm.all_windows().find(|w| w.bounds().contains(pos))?.clone();

        if self
            .bindings
            .resize
            .matches(&self.current_keys, &self.current_buttons)
        {
            Some((window, WindowAreaDragType::Resize))
        } else if self
            .bindings
            .resize_symmetric
            .matches(&self.current_keys, &self.current_buttons)
        {
            Some((window, WindowAreaDragType::ResizeSymmetric))
        } else if self
            .bindings
            .slide
            .matches(&self.current_keys, &self.current_buttons)
        {
            Some((window, WindowAreaDragType::Slide))
        } else if self
            .bindings
            .tile
            .matches(&self.current_keys, &self.current_buttons)
        {
            Some((window, WindowAreaDragType::Tile))
        } else {
            None
        }
    }

    fn cancel_if_no_binding(&mut self) {
        if self.current_drag.is_some()
            && !self
                .bindings
                .resize
                .matches(&self.current_keys, &self.current_buttons)
            && !self
                .bindings
                .resize_symmetric
                .matches(&self.current_keys, &self.current_buttons)
            && !self
                .bindings
                .slide
                .matches(&self.current_keys, &self.current_buttons)
            && !self
                .bindings
                .tile
                .matches(&self.current_keys, &self.current_buttons)
        {
            self.current_drag = None;
        }
    }

    pub fn get_drag_start(&self, id: WindowId) -> Option<(Position, Bounds)> {
        self.current_drag.as_ref().and_then(|drag| {
            if drag.window.id() == id {
                Some((drag.start_position.clone(), drag.start_bounds.clone()))
            } else {
                None
            }
        })
    }
}
