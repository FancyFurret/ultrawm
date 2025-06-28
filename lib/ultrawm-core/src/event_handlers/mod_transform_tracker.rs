use crate::config::ModTransformBindings;
use crate::mod_mouse_keybind_tracker::{KeybindEvent, ModMouseKeybindTracker};
use crate::platform::{Bounds, Position, WMEvent, WindowId};
use crate::window::WindowRef;
use crate::wm::WindowManager;
use crate::Config;
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq)]
pub enum ModTransformDragEvent {
    Start(WindowId, Position, ModTransformType),
    Drag(WindowId, Position, ModTransformType),
    End(WindowId, Position, ModTransformType),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModTransformType {
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
    drag_type: ModTransformType,
}

pub struct ModTransformTracker {
    bindings: ModTransformBindings,
    tile_binding: ModMouseKeybindTracker,
    resize_binding: ModMouseKeybindTracker,
    resize_symmetric_binding: ModMouseKeybindTracker,
    slide_binding: ModMouseKeybindTracker,
    tile_drag: Option<DragContext>,
    resize_drag: Option<DragContext>,
    resize_symmetric_drag: Option<DragContext>,
    slide_drag: Option<DragContext>,
}

impl ModTransformTracker {
    pub fn new() -> Self {
        let config = Config::current();
        Self {
            bindings: config.mod_transform_bindings.clone(),
            tile_binding: ModMouseKeybindTracker::new(config.mod_transform_bindings.tile.clone()),
            resize_binding: ModMouseKeybindTracker::new(
                config.mod_transform_bindings.resize.clone(),
            ),
            resize_symmetric_binding: ModMouseKeybindTracker::new(
                config.mod_transform_bindings.resize_symmetric.clone(),
            ),
            slide_binding: ModMouseKeybindTracker::new(config.mod_transform_bindings.slide.clone()),
            tile_drag: None,
            resize_drag: None,
            resize_symmetric_drag: None,
            slide_drag: None,
        }
    }

    pub fn active(&self) -> bool {
        self.tile_binding.mod_held()
            || self.resize_binding.mod_held()
            || self.resize_symmetric_binding.mod_held()
            || self.slide_binding.mod_held()
    }

    pub fn handle_event(
        &mut self,
        event: &WMEvent,
        wm: &WindowManager,
    ) -> Vec<ModTransformDragEvent> {
        // Call all bindings so they can track their state properly, collect and filter results
        let mut events: Vec<ModTransformDragEvent> = vec![
            Self::handle_binding(
                event,
                &mut self.tile_binding,
                ModTransformType::Tile,
                wm,
                &mut self.tile_drag,
            ),
            Self::handle_binding(
                event,
                &mut self.resize_binding,
                ModTransformType::Resize,
                wm,
                &mut self.resize_drag,
            ),
            Self::handle_binding(
                event,
                &mut self.resize_symmetric_binding,
                ModTransformType::ResizeSymmetric,
                wm,
                &mut self.resize_symmetric_drag,
            ),
            Self::handle_binding(
                event,
                &mut self.slide_binding,
                ModTransformType::Slide,
                wm,
                &mut self.slide_drag,
            ),
        ]
        .into_iter()
        .filter_map(|x| x)
        .collect();

        // Sort events so End events come before Start events
        events.sort_by_key(|event| match event {
            ModTransformDragEvent::End(_, _, _) => 0,
            ModTransformDragEvent::Drag(_, _, _) => 1,
            ModTransformDragEvent::Start(_, _, _) => 2,
        });

        events
    }

    fn handle_binding(
        event: &WMEvent,
        binding: &mut ModMouseKeybindTracker,
        drag_type: ModTransformType,
        wm: &WindowManager,
        current_drag: &mut Option<DragContext>,
    ) -> Option<ModTransformDragEvent> {
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

                Some(ModTransformDragEvent::Start(window.id(), pos, drag_type))
            }
            Some(KeybindEvent::Drag(pos)) => {
                if let Some(drag) = current_drag {
                    Some(ModTransformDragEvent::Drag(
                        drag.window.id(),
                        pos,
                        drag_type,
                    ))
                } else {
                    None
                }
            }
            Some(KeybindEvent::End(pos)) => {
                if let Some(drag) = current_drag.take() {
                    Some(ModTransformDragEvent::End(
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
