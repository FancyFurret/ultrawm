use crate::config::WindowAreaBindings;
use crate::modified_mouse_keybind_tracker::{KeybindEvent, ModifiedMouseKeybindTracker};
use crate::platform::{Bounds, PlatformEvent, Position, WindowId};
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
    bindings: WindowAreaBindings,
    tile_binding: ModifiedMouseKeybindTracker,
    resize_binding: ModifiedMouseKeybindTracker,
    resize_symmetric_binding: ModifiedMouseKeybindTracker,
    slide_binding: ModifiedMouseKeybindTracker,
    tile_drag: Option<DragContext>,
    resize_drag: Option<DragContext>,
    resize_symmetric_drag: Option<DragContext>,
    slide_drag: Option<DragContext>,
}

impl WindowAreaTracker {
    pub fn new() -> Self {
        let config = Config::current();
        Self {
            bindings: config.window_area_bindings.clone(),
            tile_binding: ModifiedMouseKeybindTracker::new(
                config.window_area_bindings.tile.clone(),
            ),
            resize_binding: ModifiedMouseKeybindTracker::new(
                config.window_area_bindings.resize.clone(),
            ),
            resize_symmetric_binding: ModifiedMouseKeybindTracker::new(
                config.window_area_bindings.resize_symmetric.clone(),
            ),
            slide_binding: ModifiedMouseKeybindTracker::new(
                config.window_area_bindings.slide.clone(),
            ),
            tile_drag: None,
            resize_drag: None,
            resize_symmetric_drag: None,
            slide_drag: None,
        }
    }

    pub fn handle_event(
        &mut self,
        event: &PlatformEvent,
        wm: &WindowManager,
    ) -> Vec<WindowAreaDragEvent> {
        // Call all bindings so they can track their state properly, collect and filter results
        let mut events: Vec<WindowAreaDragEvent> = vec![
            Self::handle_binding(
                event,
                &mut self.tile_binding,
                WindowAreaDragType::Tile,
                wm,
                &mut self.tile_drag,
            ),
            Self::handle_binding(
                event,
                &mut self.resize_binding,
                WindowAreaDragType::Resize,
                wm,
                &mut self.resize_drag,
            ),
            Self::handle_binding(
                event,
                &mut self.resize_symmetric_binding,
                WindowAreaDragType::ResizeSymmetric,
                wm,
                &mut self.resize_symmetric_drag,
            ),
            Self::handle_binding(
                event,
                &mut self.slide_binding,
                WindowAreaDragType::Slide,
                wm,
                &mut self.slide_drag,
            ),
        ]
        .into_iter()
        .filter_map(|x| x)
        .collect();

        // Sort events so End events come before Start events
        events.sort_by_key(|event| match event {
            WindowAreaDragEvent::End(_, _, _) => 0,
            WindowAreaDragEvent::Drag(_, _, _) => 1,
            WindowAreaDragEvent::Start(_, _, _) => 2,
        });

        events
    }

    fn handle_binding(
        event: &PlatformEvent,
        binding: &mut ModifiedMouseKeybindTracker,
        drag_type: WindowAreaDragType,
        wm: &WindowManager,
        current_drag: &mut Option<DragContext>,
    ) -> Option<WindowAreaDragEvent> {
        match binding.handle_event(event) {
            Some(KeybindEvent::Start(pos)) => {
                let window = wm
                    .all_windows()
                    .find(|w| w.bounds().contains(&pos))?
                    .clone();
                let start_bounds = window.bounds().clone();

                *current_drag = Some(DragContext {
                    window: window.clone(),
                    start_position: pos.clone(),
                    start_bounds,
                    drag_type: drag_type.clone(),
                });

                Some(WindowAreaDragEvent::Start(window.id(), pos, drag_type))
            }
            Some(KeybindEvent::Drag(pos)) => {
                if let Some(drag) = current_drag {
                    Some(WindowAreaDragEvent::Drag(drag.window.id(), pos, drag_type))
                } else {
                    None
                }
            }
            Some(KeybindEvent::End(pos)) => {
                if let Some(drag) = current_drag.take() {
                    Some(WindowAreaDragEvent::End(
                        drag.window.id(),
                        pos,
                        drag.drag_type,
                    ))
                } else {
                    None
                }
            }
            None => None,
        }
    }
}
