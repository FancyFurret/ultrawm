use crate::config::ModTransformBindings;
use crate::event_handlers::mod_mouse_keybind_tracker::{KeybindEvent, ModMouseKeybindTracker};
use crate::layouts::ResizeDirection;
use crate::platform::{Bounds, Position, WMEvent, WindowId};
use crate::window::WindowRef;
use crate::wm::WindowManager;
use crate::Config;
use std::fmt::Debug;

// Threshold percentage for determining slide vs resize - if mouse is within this percentage
// of the center of the window, it should slide instead of resize
const SLIDE_THRESHOLD_PERCENT: f32 = 0.3;

#[derive(Debug, Clone, PartialEq)]
pub enum ModTransformDragEvent {
    Start(WindowId, Position, ModTransformType),
    Drag(WindowId, Position, ModTransformType),
    End(WindowId, Position, ModTransformType),
    Cancel(WindowId, ModTransformType),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModTransformType {
    Tile,
    Float,
    Shift,
    Toggle,
    Slide,
    Resize(ResizeDirection),
    ResizeSymmetric(ResizeDirection),
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
    float_binding: ModMouseKeybindTracker,
    shift_binding: ModMouseKeybindTracker,
    toggle_binding: ModMouseKeybindTracker,
    resize_binding: ModMouseKeybindTracker,
    resize_symmetric_binding: ModMouseKeybindTracker,
    tile_drag: Option<DragContext>,
    float_drag: Option<DragContext>,
    shift_drag: Option<DragContext>,
    toggle_drag: Option<DragContext>,
    resize_drag: Option<DragContext>,
    resize_symmetric_drag: Option<DragContext>,
}

impl ModTransformTracker {
    /// Check if a position is in the middle area of the window bounds
    fn is_position_in_middle(bounds: &Bounds, pos: &Position) -> bool {
        let center_x = bounds.position.x + (bounds.size.width as i32) / 2;
        let center_y = bounds.position.y + (bounds.size.height as i32) / 2;

        let threshold_width = (bounds.size.width as f32 * SLIDE_THRESHOLD_PERCENT) as i32;
        let threshold_height = (bounds.size.height as f32 * SLIDE_THRESHOLD_PERCENT) as i32;

        let left_bound = center_x - threshold_width / 2;
        let right_bound = center_x + threshold_width / 2;
        let top_bound = center_y - threshold_height / 2;
        let bottom_bound = center_y + threshold_height / 2;

        pos.x >= left_bound && pos.x <= right_bound && pos.y >= top_bound && pos.y <= bottom_bound
    }

    fn resize_mode(bounds: &Bounds, pos: &Position) -> ResizeDirection {
        if let Some(dir) = Self::detect_corner_wedge(bounds, pos) {
            return dir;
        }
        Self::detect_edge(bounds, pos)
    }

    fn detect_corner_wedge(bounds: &Bounds, pos: &Position) -> Option<ResizeDirection> {
        let left = bounds.position.x;
        let right = bounds.position.x + bounds.size.width as i32;
        let top = bounds.position.y;
        let bottom = bounds.position.y + bounds.size.height as i32;
        let w = bounds.size.width as f32;
        let h = bounds.size.height as f32;
        let corner_w = (w * 0.25).round() as i32;
        let corner_h = (h * 0.25).round() as i32;

        // Top-left
        if pos.x >= left && pos.x < left + corner_w && pos.y >= top && pos.y < top + corner_h {
            return Some(ResizeDirection::TopLeft);
        }
        // Top-right
        if pos.x <= right && pos.x > right - corner_w && pos.y >= top && pos.y < top + corner_h {
            return Some(ResizeDirection::TopRight);
        }
        // Bottom-left
        if pos.x >= left && pos.x < left + corner_w && pos.y <= bottom && pos.y > bottom - corner_h
        {
            return Some(ResizeDirection::BottomLeft);
        }
        // Bottom-right
        if pos.x <= right
            && pos.x > right - corner_w
            && pos.y <= bottom
            && pos.y > bottom - corner_h
        {
            return Some(ResizeDirection::BottomRight);
        }
        None
    }

    fn detect_edge(bounds: &Bounds, pos: &Position) -> ResizeDirection {
        let left = bounds.position.x;
        let right = bounds.position.x + bounds.size.width as i32;
        let top = bounds.position.y;
        let bottom = bounds.position.y + bounds.size.height as i32;
        let left_dist = (pos.x - left).abs();
        let right_dist = (pos.x - right).abs();
        let top_dist = (pos.y - top).abs();
        let bottom_dist = (pos.y - bottom).abs();
        let mut min_dist = left_dist;
        let mut dir = ResizeDirection::Left;
        if right_dist < min_dist {
            min_dist = right_dist;
            dir = ResizeDirection::Right;
        }
        if top_dist < min_dist {
            min_dist = top_dist;
            dir = ResizeDirection::Top;
        }
        if bottom_dist < min_dist {
            dir = ResizeDirection::Bottom;
        }
        dir
    }

    pub fn new() -> Self {
        let config = Config::current();
        Self {
            bindings: config.mod_transform_bindings.clone(),
            tile_binding: ModMouseKeybindTracker::new(config.mod_transform_bindings.tile.clone()),
            float_binding: ModMouseKeybindTracker::new(config.mod_transform_bindings.float.clone()),
            shift_binding: ModMouseKeybindTracker::new(config.mod_transform_bindings.shift.clone()),
            toggle_binding: ModMouseKeybindTracker::new(
                config.mod_transform_bindings.toggle.clone(),
            ),
            resize_binding: ModMouseKeybindTracker::new(
                config.mod_transform_bindings.resize.clone(),
            ),
            resize_symmetric_binding: ModMouseKeybindTracker::new(
                config.mod_transform_bindings.resize_symmetric.clone(),
            ),
            tile_drag: None,
            float_drag: None,
            shift_drag: None,
            toggle_drag: None,
            resize_drag: None,
            resize_symmetric_drag: None,
        }
    }

    pub fn active(&self) -> bool {
        self.tile_binding.mod_held()
            || self.float_binding.mod_held()
            || self.shift_binding.mod_held()
            || self.toggle_binding.mod_held()
            || self.resize_binding.mod_held()
            || self.resize_symmetric_binding.mod_held()
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
                |_, _| ModTransformType::Tile,
                wm,
                &mut self.tile_drag,
            ),
            Self::handle_binding(
                event,
                &mut self.float_binding,
                |_, _| ModTransformType::Float,
                wm,
                &mut self.float_drag,
            ),
            Self::handle_binding(
                event,
                &mut self.shift_binding,
                |_, _| ModTransformType::Shift,
                wm,
                &mut self.shift_drag,
            ),
            Self::handle_binding(
                event,
                &mut self.toggle_binding,
                |_, _| ModTransformType::Toggle,
                wm,
                &mut self.toggle_drag,
            ),
            Self::handle_binding(
                event,
                &mut self.resize_binding,
                |bounds, pos| {
                    if Self::is_position_in_middle(bounds, pos) {
                        ModTransformType::Slide
                    } else {
                        let direction = Self::resize_mode(bounds, pos);
                        ModTransformType::Resize(direction)
                    }
                },
                wm,
                &mut self.resize_drag,
            ),
            Self::handle_binding(
                event,
                &mut self.resize_symmetric_binding,
                |bounds, pos| {
                    if Self::is_position_in_middle(bounds, pos) {
                        ModTransformType::Slide
                    } else {
                        let direction = Self::resize_mode(bounds, pos);
                        ModTransformType::ResizeSymmetric(direction)
                    }
                },
                wm,
                &mut self.resize_symmetric_drag,
            ),
        ]
        .into_iter()
        .filter_map(|x| x)
        .collect();

        // Sort events so End events come before Start events
        events.sort_by_key(|event| match event {
            ModTransformDragEvent::End(_, _, _) => 0,
            ModTransformDragEvent::Cancel(_, _) => 1,
            ModTransformDragEvent::Drag(_, _, _) => 2,
            ModTransformDragEvent::Start(_, _, _) => 3,
        });

        events
    }

    fn handle_binding<F>(
        event: &WMEvent,
        binding: &mut ModMouseKeybindTracker,
        determine_drag_type: F,
        wm: &WindowManager,
        current_drag: &mut Option<DragContext>,
    ) -> Option<ModTransformDragEvent>
    where
        F: Fn(&Bounds, &Position) -> ModTransformType,
    {
        match binding.handle_event(event) {
            Some(KeybindEvent::Start(pos)) => {
                let window = wm.find_window_at_position(&pos)?;
                let start_bounds = window.bounds().clone();

                let drag_type = determine_drag_type(&start_bounds, &pos);

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
                        drag.drag_type.clone(),
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
            Some(KeybindEvent::Cancel()) => {
                if let Some(drag) = current_drag.take() {
                    Some(ModTransformDragEvent::Cancel(
                        drag.window.id(),
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
