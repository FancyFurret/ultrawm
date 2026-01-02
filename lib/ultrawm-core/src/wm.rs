use crate::config::Config;
use crate::layouts::{ContainerTree, LayoutError, PlacementTarget, WindowLayout};
use crate::partition::{Partition, PartitionId};
use crate::platform::{Bounds, Platform, PlatformImpl, PlatformResult, Position, WindowId};
use crate::resize_handle::{ResizeHandle, ResizeMode};
use crate::serialization::{extract_window_ids, load_layout, save_layout};
use crate::tile_result::InsertResult;
use crate::window::{Window, WindowRef};
use crate::workspace::{Workspace, WorkspaceId};
use crate::workspace_animator::{WorkspaceAnimationConfig, WorkspaceAnimationThread};
use crate::PlatformError;
use indexmap::IndexSet;
use log::{error, info, trace, warn};
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
    all_windows: HashMap<WindowId, WindowRef>,
    /// Set when deferred resize methods are called, cleared on flush
    needs_flush: bool,
}

impl WindowManager {
    pub fn new() -> PlatformResult<Self> {
        let displays = Platform::list_all_displays()?;
        trace!("Displays ({}):", displays.len());
        for d in &displays {
            trace!(
                "  {:?} bounds={:?} work_area={:?}",
                d.name,
                d.bounds,
                d.work_area
            );
        }

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

        // Store all discovered windows so they're available for layout loading
        let mut all_windows: HashMap<WindowId, WindowRef> = HashMap::new();
        for window in &windows {
            all_windows.insert(window.id(), window.clone());
        }

        let mut wm = Self {
            partitions,
            workspaces: HashMap::new(),
            window_order: IndexSet::new(),
            animation_thread: WorkspaceAnimationThread::new(WorkspaceAnimationConfig {
                animation_fps: Config::window_tile_fps(),
            }),
            all_windows,
            needs_flush: false,
        };

        // Try to load saved layout
        let has_saved_layout = if let Ok(Some(saved_layout)) = load_layout() {
            for serialized_partition in saved_layout.partitions {
                // Find partition by name
                let partition_id = match wm
                    .partitions
                    .values()
                    .find(|p| p.name() == &serialized_partition.name)
                    .map(|p| p.id())
                {
                    Some(id) => id,
                    None => {
                        warn!(
                            "Saved layout references unknown partition: {}",
                            serialized_partition.name
                        );
                        continue;
                    }
                };

                // Load each workspace using the reusable function
                for serialized_workspace in &serialized_partition.workspaces {
                    if let Err(e) = wm.load_serialized_workspace(serialized_workspace, partition_id)
                    {
                        warn!(
                            "Failed to load workspace {}: {}",
                            serialized_workspace.id, e
                        );
                    }
                }
            }
            true
        } else {
            false
        };

        // Create default workspaces only if we don't have a saved layout
        if !has_saved_layout {
            for partition in wm.partitions.values_mut() {
                if partition.current_workspace().is_none() {
                    let workspace = Workspace::new::<ContainerTree>(
                        partition.bounds().clone(),
                        "Default".to_string(),
                        None,
                        None,
                    );
                    let workspace_id = workspace.id();
                    wm.workspaces.insert(workspace_id, workspace);
                    partition.assign_workspace(workspace_id);
                }
            }
        }

        // Flush all windows
        for workspace in wm.workspaces.values_mut() {
            workspace.flush_windows()?;
        }

        trace!("Partitions ({}):", wm.partitions.len());
        for p in wm.partitions.values() {
            trace!("  {:?} bounds={:?}", p.name(), p.bounds());
        }

        trace!("Tracking {} windows at startup...", windows.len());
        for window in windows {
            if wm.get_workspace_with_window(&window).is_none() {
                trace!("  Tracking window: id={}", window.id());
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
        trace!(
            "track_window: id={} visible={} title={:?}",
            window.id(),
            window.visible(),
            window.title()
        );

        // Always add to all_windows if not already present
        if !self.all_windows.contains_key(&window.id()) {
            self.all_windows.insert(window.id(), window.clone());
        }

        // Check if already in a workspace
        if self.get_workspace_with_window(&window).is_some() {
            trace!("  -> already tracked in workspace");
            return Ok(());
        }

        if !window.visible() {
            trace!("  -> not visible, stored in all_windows");
            return Ok(());
        }

        if Config::float_new_windows() {
            trace!("  -> floating window");
            let workspace = self.get_workspace_at_bounds_mut(&window.bounds())?;
            workspace.float_window(&window)?;
            self.float_window(window.id())?;
        } else {
            trace!("  -> tiling window at {:?}", window.bounds().position);
            self.tile_window(window.id(), &window.bounds().position)?;
        }

        Ok(())
    }

    pub fn unhide_window(&mut self, id: WindowId) -> WMResult<()> {
        let window = match self.all_windows.get(&id) {
            Some(w) => w.clone(),
            None => return Ok(()),
        };

        // If already in a workspace, nothing to do
        if self.get_workspace_with_window(&window).is_some() {
            return Ok(());
        }

        window.update_bounds();
        self.track_window(window)?;
        Ok(())
    }

    pub fn tile_window(&mut self, id: WindowId, position: &Position) -> WMResult<()> {
        let window = self.get_window(id)?;
        let was_floating = window.floating();
        let old_bounds = window.bounds().clone();

        if was_floating {
            window.set_floating(false);
        }

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
                    self.float_window(new_window.id())?;
                    new_window.set_bounds(old_bounds);
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

    pub fn insert_window_relative(
        &mut self,
        window_id: WindowId,
        target: PlacementTarget,
        workspace_id: WorkspaceId,
    ) -> WMResult<()> {
        let window = self.get_window(window_id)?;
        // If target_workspace_id is specified and different from current workspace, move the window first
        let current_workspace_id = self.get_workspace_with_window(&window).map(|w| w.id());
        if current_workspace_id != Some(workspace_id) {
            if let Some(old_ws_id) = current_workspace_id {
                let old_workspace = self.workspaces.get_mut(&old_ws_id).unwrap();
                old_workspace.remove_window(&window)?;
            }
        }

        let workspace = self
            .workspaces
            .get_mut(&workspace_id)
            .ok_or(WMError::WorkspaceNotFound(0))?;
        workspace.insert_window_relative(&window, target)?;

        self.animated_flush()?;
        self.try_save_layout();
        Ok(())
    }

    /// Animated flush that sends dirty windows to the animation thread
    pub fn animated_flush(&mut self) -> PlatformResult<()> {
        self.validate_workspaces();
        
        for workspace in self.workspaces.values_mut() {
            for window in workspace.windows().values() {
                window.flush_always_on_top()?;

                if !window.dirty() {
                    continue;
                }

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
        let new_workspace = self.get_workspace_at_bounds_mut(&bounds)?;
        let new_workspace_id = new_workspace.id();

        if let Some(old_ws_id) = old_workspace_id {
            if old_ws_id != new_workspace_id {
                let old_workspace = self
                    .workspaces
                    .get_mut(&old_ws_id)
                    .ok_or_else(|| WMError::WorkspaceNotFound(0))?;
                old_workspace.remove_window(&window)?;

                let new_workspace = self
                    .workspaces
                    .get_mut(&new_workspace_id)
                    .ok_or_else(|| WMError::WorkspaceNotFound(0))?;
                new_workspace.float_window(&window)?;
                self.try_save_layout();
            }
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
        self.animated_flush()?;
        self.move_to_top(window.id());
        self.try_save_layout();
        Ok(())
    }

    pub fn remove_window(&mut self, id: WindowId) -> WMResult<()> {
        let window = self.get_window(id)?;

        let workspace = self.get_workspace_for_window_mut(&id)?;
        workspace.remove_window(&window)?;
        self.animated_flush()?;
        self.try_save_layout();
        Ok(())
    }

    /// Validates all windows across all workspaces and removes invalid ones.
    /// Returns the number of invalid windows that were removed.
    pub fn validate_workspaces(&mut self) -> usize {
        let mut invalid_windows = Vec::new();

        for window in self.all_windows.values() {
            if !window.valid() && !invalid_windows.contains(&window.id()) {
                invalid_windows.push(window.id());
            }
        }

        // Remove invalid windows
        let removed_count = invalid_windows.len();
        for id in &invalid_windows {
            trace!("Removing invalid window: id={} title={:?}", id, {
                if let Some(w) = self.all_windows.get(id) {
                    w.title()
                } else {
                    "<unknown>".to_string()
                }
            });
            
            // Try to remove from workspace (may fail if not in workspace, that's ok)
            if let Ok(window) = self.get_window(*id) {
                if let Ok(workspace) = self.get_workspace_for_window_mut(id) {
                    let _ = workspace.remove_window(&window);
                }
            }
            
            // Remove from all_windows
            self.all_windows.remove(id);
            self.window_order.shift_remove(id);
        }

        if removed_count > 0 {
            // Flush and save layout after removing invalid windows
            let _ = self.animated_flush();
            self.try_save_layout();
        }

        removed_count
    }

    pub fn resize_window(&mut self, id: WindowId, bounds: &Bounds) -> WMResult<()> {
        let window = self.get_window(id)?;
        let workspace = self.get_workspace_for_window_mut(&id)?;

        workspace.resize_window(&window, bounds)?;
        workspace.flush_windows()?;
        self.needs_flush = false;
        self.try_save_layout();
        Ok(())
    }

    /// Set resize bounds without flushing - for use during live drag.
    /// Call flush() to apply pending changes.
    pub fn resize_window_deferred(&mut self, id: WindowId, bounds: &Bounds) -> WMResult<()> {
        let window = self.get_window(id)?;
        let workspace = self.get_workspace_for_window_mut(&id)?;
        workspace.resize_window(&window, bounds)?;
        self.needs_flush = true;
        Ok(())
    }

    pub fn get_window(&self, id: WindowId) -> WMResult<WindowRef> {
        self.all_windows.get(&id).cloned().ok_or_else(|| {
            error!("Window not found :*( :* : {id}");
            WMError::WindowNotFound(id)
        })
    }

    pub fn get_all_windows(&self) -> Vec<WindowRef> {
        let mut all_windows: Vec<WindowRef> = self.all_windows.values().cloned().collect();

        all_windows.sort_by_key(|w| {
            let order_index = self.window_order.get_index_of(&w.id()).unwrap_or(0);
            (w.floating(), order_index)
        });

        all_windows
    }

    pub fn get_tile_bounds(&self, id: WindowId, position: &Position) -> Option<Bounds> {
        let workspace = self.get_workspace_at_position(position).ok()?;
        let window = self.get_window(id).ok()?;
        workspace.get_tile_bounds(&window, position)
    }

    pub fn get_partition_with_window(&self, window: &WindowRef) -> Option<&Partition> {
        for partition in self.partitions.values() {
            if partition
                .current_workspace()
                .and_then(|ws_id| {
                    self.workspaces
                        .get(&ws_id)
                        .map(|ws| ws.has_window(&window.id()))
                })
                .unwrap_or(false)
            {
                return Some(partition);
            }
        }
        None
    }

    fn get_partition_with_window_mut(&mut self, window: &WindowRef) -> Option<&mut Partition> {
        for partition in self.partitions.values_mut() {
            if partition
                .current_workspace()
                .and_then(|ws_id| {
                    self.workspaces
                        .get(&ws_id)
                        .map(|ws| ws.has_window(&window.id()))
                })
                .unwrap_or(false)
            {
                return Some(partition);
            }
        }
        None
    }

    pub fn get_workspace_with_window(&self, window: &WindowRef) -> Option<&Workspace> {
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
        let all_windows = self.get_all_windows();

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
            workspace.resize_handle_moved(handle, position, mode);
            self.needs_flush = true;
        }
        Ok(())
    }

    /// Flush all pending window changes across all workspaces.
    /// Called periodically by the event loop during live resize operations.
    pub fn flush(&mut self) -> WMResult<()> {
        if !self.needs_flush {
            return Ok(());
        }
        self.needs_flush = false;
        self.validate_workspaces();
        for workspace in self.workspaces.values_mut() {
            workspace.flush_windows()?;
        }
        Ok(())
    }

    pub fn move_to_top(&mut self, id: WindowId) {
        self.window_order.shift_remove(&id);
        self.window_order.insert(id);
    }

    pub fn cleanup(&mut self) -> PlatformResult<()> {
        for workspace in self.workspaces.values_mut() {
            workspace.cleanup();
        }
        Ok(())
    }

    pub fn try_save_layout(&self) {
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

    pub fn load_layout_to_workspace(
        &mut self,
        workspace_id: WorkspaceId,
        layout: &serde_yaml::Value,
    ) -> WMResult<()> {
        let workspace = self
            .workspaces
            .get_mut(&workspace_id)
            .ok_or(WMError::WorkspaceNotFound(0))?;

        let partition_bounds = self
            .partitions
            .values()
            .find(|p| p.current_workspace() == Some(workspace_id))
            .map(|p| p.bounds().clone())
            .ok_or(WMError::LayoutError(LayoutError::Error(format!(
                "No partition found for workspace {}",
                workspace_id
            ))))?;

        let layout_window_ids = extract_window_ids(layout);
        let layout_windows: Vec<WindowRef> = layout_window_ids
            .iter()
            .filter_map(|id| self.all_windows.get(id).cloned())
            .collect();

        for window in &layout_windows {
            window.set_floating(false);
        }

        let new_layout = Box::new(ContainerTree::deserialize(
            partition_bounds.clone(),
            &layout_windows,
            layout,
        ));

        let workspace_name = workspace.name().to_string();
        let new_workspace = Workspace::new_with_id::<ContainerTree>(
            workspace_id,
            partition_bounds,
            workspace_name,
            Some(new_layout),
            None,
        );

        *self.workspaces.get_mut(&workspace_id).unwrap() = new_workspace;

        self.animated_flush()?;
        self.try_save_layout();

        Ok(())
    }

    fn load_serialized_workspace(
        &mut self,
        serialized_workspace: &crate::serialization::SerializedWorkspace,
        partition_id: PartitionId,
    ) -> WMResult<()> {
        if !self.workspaces.contains_key(&serialized_workspace.id) {
            let partition = self.partitions.get(&partition_id).unwrap();
            let workspace = Workspace::new_with_id::<ContainerTree>(
                serialized_workspace.id,
                partition.bounds().clone(),
                serialized_workspace.name.clone(),
                None,
                None,
            );
            self.workspaces.insert(serialized_workspace.id, workspace);
            self.partitions
                .get_mut(&partition_id)
                .unwrap()
                .assign_workspace(serialized_workspace.id);
        }

        self.load_layout_to_workspace(serialized_workspace.id, &serialized_workspace.layout)?;

        let workspace = self.workspaces.get_mut(&serialized_workspace.id).unwrap();
        for serialized_floating in &serialized_workspace.floating {
            if let Some(window) = self.all_windows.get(&serialized_floating.id) {
                let _ = workspace.float_window(window);
            }
        }

        Ok(())
    }
}
