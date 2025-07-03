use crate::config::Config;
use crate::layouts::{ContainerTree, LayoutError};
use crate::partition::{Partition, PartitionId};
use crate::platform::{Bounds, Platform, PlatformImpl, PlatformResult, Position, WindowId};
use crate::resize_handle::{ResizeHandle, ResizeMode};
use crate::serialization::{deserialize_partition, load_layout, save_layout};
use crate::tile_result::InsertResult;
use crate::window::{Window, WindowRef};
use crate::workspace::{Workspace, WorkspaceId};
use crate::workspace_animator::{WorkspaceAnimationConfig, WorkspaceAnimationThread};
use crate::PlatformError;
use indexmap::{IndexMap, IndexSet};
use log::{error, warn};
use std::collections::HashMap;
use std::rc::Rc;
use thiserror::Error;

// Number of partitions to create per display
// Temporary
const PARTITIONS_PER_DISPLAY: u32 = 1;

#[derive(Debug, Error)]
pub enum WMError {
    #[error("Window not found: {0}")]
    WindowNotFound(WindowId),

    #[error("No workspace found for window: {0:?}")]
    WorkspaceNotFound(WindowId),

    #[error("No workspace found at position: {0:?}")]
    NoWorkspaceAtPosition(Position),

    #[error(transparent)]
    LayoutError(#[from] LayoutError),

    #[error(transparent)]
    Platform(#[from] PlatformError),
}

pub type WMResult<T> = Result<T, WMError>;

pub struct WindowManager {
    partitions: HashMap<PartitionId, Partition>,
    workspaces: HashMap<WorkspaceId, Workspace>,
    window_order: IndexSet<WindowId>,
    animation_thread: WorkspaceAnimationThread,
    minimized_windows: HashMap<WindowId, WindowRef>,
}

impl WindowManager {
    pub fn new() -> PlatformResult<Self> {
        let displays = Platform::list_all_displays()?;

        let mut partitions: HashMap<PartitionId, Partition> = HashMap::new();
        for display in displays {
            let partition_width = display.work_area.size.width / PARTITIONS_PER_DISPLAY;

            for i in 0..PARTITIONS_PER_DISPLAY {
                let partition_bounds = Bounds::new(
                    display.work_area.position.x + (i as i32 * partition_width as i32),
                    display.work_area.position.y,
                    partition_width,
                    display.work_area.size.height,
                );

                let partition_name = if PARTITIONS_PER_DISPLAY == 1 {
                    display.name.clone()
                } else {
                    format!("{}_partition_{}", display.name, i + 1)
                };

                let partition = Partition::new(partition_name, partition_bounds);
                partitions.insert(partition.id(), partition);
            }
        }

        // Sort by window id so that re-running the WM is more stable
        let mut windows = Platform::list_visible_windows()?
            .iter()
            .map(|w| Rc::new(Window::new(w.clone())))
            .collect::<Vec<_>>();
        windows.sort_by_key(|w| w.id());

        // Try to load saved layout
        let saved_layout = match load_layout() {
            Ok(Some(layout)) => Some(layout),
            Ok(None) => None,
            Err(e) => {
                warn!("Failed to load saved layout: {}, creating new layout", e);
                None
            }
        };

        // If there is a saved layout, use it
        let mut workspaces: HashMap<WorkspaceId, Workspace> = HashMap::new();
        if let Some(layout) = saved_layout {
            for serialized_partition in layout.partitions {
                let (p, ws) = deserialize_partition(&serialized_partition, &windows);
                let name = p.name();
                if let Some(partition) = partitions.iter_mut().find(|(_, p)| p.name() == name) {
                    partition.1.assign_workspace(p.current_workspace().unwrap());
                }

                for (workspace_id, workspace) in ws {
                    workspaces.insert(workspace_id, workspace);
                }
            }
        } else {
            // Also for now, just make 1 workspace per partition. Will be configurable later.
            workspaces = partitions
                .values_mut()
                .map(|partition| {
                    let workspace = Workspace::new::<ContainerTree>(
                        partition.bounds().clone(),
                        "Default".to_string(),
                        None,
                        None,
                    );

                    partition.assign_workspace(workspace.id());
                    (workspace.id(), workspace)
                })
                .collect();
        }

        let mut existing_windows = IndexMap::new();
        for workspace in workspaces.values_mut() {
            for window in workspace.windows().values() {
                existing_windows.insert(window.id(), window.clone());
            }

            workspace.flush_windows()?;
        }

        // Float any unused windows
        let mut wm = Self {
            partitions,
            workspaces,
            window_order: IndexSet::new(),
            animation_thread: WorkspaceAnimationThread::new(WorkspaceAnimationConfig {
                animation_fps: Config::window_tile_fps(),
            }),
            minimized_windows: HashMap::new(),
        };

        for existing_window in existing_windows.values() {
            if existing_window.floating() {
                wm.float_window(existing_window.id()).unwrap_or_else(|e| {
                    error!("Failed to float existing window: {e}");
                })
            }
        }

        for window in windows {
            if wm.get_window(window.id()).is_err() {
                wm.track_window(window.clone()).unwrap_or_else(|e| {
                    error!("Failed to track window: {e}");
                })
            }
        }

        Ok(wm)
    }

    pub fn partitions(&self) -> &HashMap<PartitionId, Partition> {
        &self.partitions
    }

    pub fn workspaces(&self) -> &HashMap<WorkspaceId, Workspace> {
        &self.workspaces
    }

    pub fn track_window(&mut self, window: WindowRef) -> WMResult<()> {
        if self.get_window(window.id()).is_ok() {
            return Ok(());
        }

        if self.minimized_windows.contains_key(&window.id()) {
            return Ok(());
        }

        if !window.visible() {
            self.minimized_windows.insert(window.id(), window.clone());
            return Ok(());
        }

        if Config::float_new_windows() {
            let workspace = self.get_workspace_at_bounds_mut(&window.bounds())?;
            workspace.float_window(&window)?;
            self.float_window(window.id())?;
        } else {
            self.tile_window(window.id(), &window.bounds().position)?;
        }

        Ok(())
    }

    pub fn unhide_window(&mut self, id: WindowId) -> WMResult<()> {
        if !self.minimized_windows.contains_key(&id) {
            return Ok(());
        }

        let window = self.minimized_windows.remove(&id).unwrap();
        window.update_bounds();
        self.track_window(window)?;
        Ok(())
    }

    pub fn tile_window(&mut self, id: WindowId, position: &Position) -> WMResult<()> {
        let window = self.get_window(id)?;
        let was_floating = window.floating();
        let old_bounds = window.bounds().clone();

        let old_workspace_id = self.get_workspace_with_window(&window).map(|w| w.id());
        let new_workspace_id = self.get_workspace_at_position(position)?.id();

        let result = self
            .workspaces
            .get_mut(&new_workspace_id)
            .unwrap()
            .tile_window(&window, position)?;

        if let Some(id) = old_workspace_id {
            // Handle the swap case where we need to float a window
            if let InsertResult::Swap(new_window) = &result {
                if was_floating {
                    new_window.set_bounds(old_bounds);
                    self.float_window(new_window.id())?;
                } else {
                    let old_workspace = self.workspaces.get_mut(&id).unwrap();
                    old_workspace.replace_window(&window, &new_window)?;
                }
            } else if id != new_workspace_id {
                let old_workspace = self.workspaces.get_mut(&id).unwrap();
                old_workspace.remove_window(&window)?;
            }
        }

        self.animated_flush()?;
        self.try_save_layout();
        Ok(())
    }

    /// Animated flush that sends dirty windows to the animation thread
    pub fn animated_flush(&mut self) -> PlatformResult<()> {
        for workspace in self.workspaces.values_mut() {
            for window in workspace.windows().values() {
                if window.dirty() {
                    if Config::window_tile_animate() {
                        let platform_window = window.platform_window().clone();
                        let start_bounds = window.platform_bounds();
                        let target_bounds = window.window_bounds().clone();
                        let duration_ms = Config::window_tile_animation_ms();

                        self.animation_thread.animate_window(
                            window.id(),
                            platform_window,
                            start_bounds,
                            target_bounds,
                            duration_ms,
                        );
                    } else {
                        window.flush()?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn focus_window(&mut self, id: WindowId) -> WMResult<()> {
        let window = self.get_window(id)?;
        window
            .focus()
            .unwrap_or_else(|e| error!("Could not focus window: {e}"));
        self.move_to_top(id);
        Ok(())
    }

    pub fn update_floating_window(&mut self, id: WindowId) -> WMResult<()> {
        let window = self.get_window(id)?;
        let bounds = window.window_bounds();
        let old_workspace_id = self.get_workspace_with_window(&window).map(|w| w.id());
        let new_workspace_id = self.get_workspace_at_bounds_mut(&bounds)?.id();
        if old_workspace_id.is_some() && old_workspace_id.unwrap() != new_workspace_id {
            let old_workspace = self.workspaces.get_mut(&old_workspace_id.unwrap()).unwrap();
            old_workspace.remove_window(&window)?;

            let new_workspace = self.workspaces.get_mut(&new_workspace_id).unwrap();
            new_workspace.float_window(&window)?;
            self.try_save_layout();
        }

        Ok(())
    }

    pub fn float_window(&mut self, id: WindowId) -> WMResult<()> {
        let window = self.get_window(id)?;
        let workspace = if let Some(workspace) = self.get_workspace_with_window_mut(&window) {
            workspace.remove_window(&window)?;
            workspace
        } else {
            self.get_workspace_at_position_mut(&window.bounds().position)?
        };

        workspace.float_window(&window)?;
        workspace.flush_windows()?;
        self.move_to_top(window.id());
        self.try_save_layout();
        Ok(())
    }

    pub fn remove_window(&mut self, id: WindowId) -> WMResult<()> {
        let window = self.get_window(id)?;
        self.minimized_windows.insert(id, window.clone());

        let workspace = self.get_workspace_for_window_mut(&id)?;
        workspace.remove_window(&window)?;
        self.animated_flush()?;
        self.try_save_layout();
        Ok(())
    }

    pub fn resize_window(&mut self, id: WindowId, bounds: &Bounds) -> WMResult<()> {
        let window = self.get_window(id)?;
        let workspace = self.get_workspace_for_window_mut(&id)?;

        workspace.resize_window(&window, bounds)?;
        workspace.flush_windows()?;
        self.try_save_layout();
        Ok(())
    }

    pub fn get_window(&self, id: WindowId) -> WMResult<WindowRef> {
        for workspace in self.workspaces.values() {
            if let Some(window) = workspace.windows().get(&id) {
                return Ok(window.clone());
            }
        }

        Err(WMError::WindowNotFound(id))
    }

    pub fn get_tile_bounds(&self, id: WindowId, position: &Position) -> Option<Bounds> {
        let workspace = self.get_workspace_at_position(position).ok()?;
        let window = self.get_window(id).ok()?;
        workspace.get_tile_bounds(&window, position)
    }

    fn get_workspace_with_window(&self, window: &WindowRef) -> Option<&Workspace> {
        for workspace in self.workspaces.values() {
            if workspace.has_window(&window.id()) {
                return Some(workspace);
            }
        }
        None
    }

    fn get_workspace_with_window_mut(&mut self, window: &WindowRef) -> Option<&mut Workspace> {
        for workspace in self.workspaces.values_mut() {
            if workspace.has_window(&window.id()) {
                return Some(workspace);
            }
        }
        None
    }
    fn get_workspace_at_position(&self, position: &Position) -> WMResult<&Workspace> {
        let partition = self
            .partitions
            .values()
            .find(|p| p.bounds().contains(&position))
            .ok_or(WMError::NoWorkspaceAtPosition(position.clone()))?;

        self.workspaces
            .get(&partition.current_workspace().unwrap())
            .ok_or(WMError::NoWorkspaceAtPosition(position.clone()))
    }

    fn get_workspace_at_position_mut(&mut self, position: &Position) -> WMResult<&mut Workspace> {
        let partition = self
            .partitions
            .values()
            .find(|p| p.bounds().contains(&position))
            .ok_or(WMError::NoWorkspaceAtPosition(position.clone()))?;

        Ok(self
            .workspaces
            .get_mut(&partition.current_workspace().unwrap())
            .unwrap())
    }

    fn get_workspace_at_bounds_mut(&mut self, bounds: &Bounds) -> WMResult<&mut Workspace> {
        let partition = self
            .partitions
            .values()
            .find(|p| p.bounds().intersects(&bounds))
            .ok_or(WMError::NoWorkspaceAtPosition(bounds.position.clone()))?;

        Ok(self
            .workspaces
            .get_mut(&partition.current_workspace().unwrap())
            .unwrap())
    }

    fn get_workspace_for_window_mut(&mut self, window_id: &WindowId) -> WMResult<&mut Workspace> {
        for workspace in self.workspaces.values_mut() {
            if workspace.has_window(window_id) {
                return Ok(workspace);
            }
        }

        Err(WMError::WorkspaceNotFound(*window_id))
    }

    fn get_windows_for_partition(windows: &mut Vec<WindowRef>, bounds: &Bounds) -> Vec<WindowRef> {
        let mut windows_in_partition = Vec::new();
        let mut i = 0;
        while i < windows.len() {
            let window = windows.get(i).unwrap();
            let center = window.bounds().center();
            if bounds.contains(&center) {
                windows_in_partition.push(windows.remove(i));
            } else {
                i += 1;
            }
        }

        windows_in_partition
    }

    /// Finds a window at the given position. This will return the top-most window/the floating window first.
    pub fn find_window_at_position(&self, position: &Position) -> Option<WindowRef> {
        let mut all_windows = Vec::new();

        // Collect all windows from all workspaces
        for workspace in self.workspaces.values() {
            all_windows.extend(workspace.windows().values().cloned());
        }

        // Sort by floating status first (floating windows always have priority), then by window order
        all_windows.sort_by_key(|w| {
            let order_index = self.window_order.get_index_of(&w.id()).unwrap_or(0);
            (w.floating(), order_index)
        });

        // Find the last window (highest priority) that contains the position
        let found = all_windows
            .into_iter()
            .rev()
            .find(|w| w.bounds().contains(position))?;

        if found.tiled() {
            if self.resize_handle_at_position_internal(position).is_some() {
                return None;
            }
        }

        Some(found)
    }

    /// If the position is on the edge a window, that window is returned.
    pub fn find_window_at_resize_edge(&self, position: &Position) -> Option<WindowRef> {
        let thickness = 15;
        let workspace = self.get_workspace_at_position(position).ok()?;
        for window in workspace.windows().values() {
            let bounds = window.window_bounds();

            let on_left_edge = (position.x - bounds.position.x).abs() <= thickness;
            let on_right_edge =
                (position.x - (bounds.position.x + bounds.size.width as i32)).abs() <= thickness;
            let on_top_edge = (position.y - bounds.position.y).abs() <= thickness;
            let on_bottom_edge =
                (position.y - (bounds.position.y + bounds.size.height as i32)).abs() <= thickness;

            // Position must be within the window's bounds on the axis perpendicular to the edge
            let within_vertical_bounds = position.y >= bounds.position.y
                && position.y <= bounds.position.y + bounds.size.height as i32;
            let within_horizontal_bounds = position.x >= bounds.position.x
                && position.x <= bounds.position.x + bounds.size.width as i32;

            if ((on_left_edge || on_right_edge) && within_vertical_bounds)
                || ((on_top_edge || on_bottom_edge) && within_horizontal_bounds)
            {
                return Some(window.clone());
            }
        }
        None
    }

    /// Returns a list of drag handles for the workspace that covers the given position.
    pub fn resize_handles(&self, position: &Position) -> Vec<ResizeHandle> {
        if let Ok(workspace) = self.get_workspace_at_position(position) {
            workspace.resize_handles()
        } else {
            Vec::new()
        }
    }

    /// Finds the first drag handle that contains the given position (if any).
    pub fn resize_handle_at_position(&self, position: &Position) -> Option<ResizeHandle> {
        if let Some(window) = self.find_window_at_position(position) {
            if window.floating() {
                return None;
            }
        }

        self.resize_handle_at_position_internal(position)
    }

    fn resize_handle_at_position_internal(&self, position: &Position) -> Option<ResizeHandle> {
        let thickness = Config::resize_handle_width() as i32;
        self.resize_handles(position)
            .into_iter()
            .find(|h| match h.orientation {
                crate::resize_handle::HandleOrientation::Vertical => {
                    let dx = (position.x - h.center.x).abs();
                    let dy = (position.y - h.center.y).abs();
                    dx <= thickness / 2 && dy <= h.length as i32 / 2
                }
                crate::resize_handle::HandleOrientation::Horizontal => {
                    let dx = (position.x - h.center.x).abs();
                    let dy = (position.y - h.center.y).abs();
                    dy <= thickness / 2 && dx <= h.length as i32 / 2
                }
            })
    }

    pub fn resize_handle_moved(
        &mut self,
        handle: &ResizeHandle,
        position: &Position,
        mode: &ResizeMode,
    ) -> WMResult<()> {
        if let Ok(workspace) = self.get_workspace_at_position_mut(position) {
            if workspace.resize_handle_moved(handle, position, mode) {
                workspace.flush_windows()?;
                self.try_save_layout();
            }
        }
        Ok(())
    }

    fn move_to_top(&mut self, id: WindowId) {
        self.window_order.shift_remove(&id);
        self.window_order.insert(id);
    }

    pub fn cleanup(&mut self) -> PlatformResult<()> {
        for workspace in self.workspaces.values_mut() {
            workspace.cleanup();
        }
        Ok(())
    }

    fn try_save_layout(&self) {
        if let Err(e) = save_layout(self) {
            warn!("Failed to save layout: {e}");
        }
    }

    pub fn config_changed(&mut self) -> PlatformResult<()> {
        for workspace in self.workspaces.values_mut() {
            workspace.config_changed()?;
        }
        Ok(())
    }
}
