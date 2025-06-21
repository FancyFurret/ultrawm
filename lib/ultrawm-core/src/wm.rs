use crate::config::Config;
use crate::layouts::{ContainerTree, LayoutError, ResizeDirection};
use crate::partition::{Partition, PartitionId};
use crate::platform::{
    Bounds, MouseButtons, Platform, PlatformImpl, PlatformResult, PlatformWindow,
    PlatformWindowImpl, Position, WindowId,
};
use crate::resize_handle::ResizeHandle;
use crate::serialization::{extract_workspace_layout, load_layout, save_layout};
use crate::tile_result::InsertResult;
use crate::window::{Window, WindowRef};
use crate::workspace::{Workspace, WorkspaceId};
use crate::PlatformError;
use log::{error, warn};
use std::collections::HashMap;
use std::rc::Rc;
use thiserror::Error;

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

#[derive(Debug)]
pub struct WindowManager {
    windows: HashMap<WindowId, WindowRef>,
    partitions: HashMap<PartitionId, Partition>,
    workspaces: HashMap<WorkspaceId, Workspace>,
}

impl WindowManager {
    pub fn new() -> PlatformResult<Self> {
        let _config = Config::current();

        let displays = Platform::list_all_displays()?;

        // For now, just make 1 partition per display. Will be configurable later.
        let mut partitions: HashMap<PartitionId, Partition> = displays
            .into_iter()
            .map(|d| {
                let partition = Partition::new(d.name, d.work_area);
                (partition.id(), partition)
            })
            .collect();

        // Sort by window id so that re-running the WM is more stable
        let mut windows = Platform::list_visible_windows()?
            .iter()
            .map(|w| Rc::new(Window::new(w.clone())))
            .collect::<Vec<_>>();
        windows.sort_by_key(|w| w.id());
        let windows_map: HashMap<WindowId, WindowRef> =
            windows.iter().map(|w| (w.id(), w.clone())).collect();

        // Try to load saved layout
        let saved_layout = match load_layout() {
            Ok(Some(layout)) => Some(layout),
            Ok(None) => None,
            Err(e) => {
                error!("Failed to load saved layout: {}, creating new layout", e);
                None
            }
        };

        // Also for now, just make 1 workspace per partition. Will be configurable later.
        let mut workspaces: HashMap<WorkspaceId, Workspace> = partitions
            .values_mut()
            .map(|partition| {
                // Extract the specific layout data for this workspace
                let workspace_layout = saved_layout.as_ref().and_then(|layout| {
                    extract_workspace_layout(layout, partition.name(), "Default")
                });

                let workspace = Workspace::new_with_saved_layout::<ContainerTree>(
                    partition.bounds().clone(),
                    &windows,
                    "Default".to_string(),
                    workspace_layout.as_ref(),
                );

                partition.assign_workspace(workspace.id());
                (workspace.id(), workspace)
            })
            .collect();

        for workspace in workspaces.values_mut() {
            workspace.flush_windows()?;
        }

        Ok(Self {
            partitions,
            workspaces,
            windows: windows_map,
        })
    }

    pub fn partitions(&self) -> &HashMap<PartitionId, Partition> {
        &self.partitions
    }

    pub fn workspaces(&self) -> &HashMap<WorkspaceId, Workspace> {
        &self.workspaces
    }

    pub fn track_window(&mut self, window: PlatformWindow) -> WMResult<()> {
        if self.windows.contains_key(&window.id()) {
            return Ok(());
        }

        let window = Rc::new(Window::new(window));
        self.windows.insert(window.id(), window.clone());

        if !Config::float_new_windows() {
            self.tile_window(window.id(), &window.bounds().position)?;
        }

        Ok(())
    }

    pub fn tile_window(&mut self, id: WindowId, position: &Position) -> WMResult<()> {
        let window = self.get_window(id)?;

        let old_workspace_id = self.get_workspace_with_window(&window).map(|w| w.id());
        let new_workspace_id = self.get_workspace_at_position(position)?.id();

        let result = self
            .workspaces
            .get_mut(&new_workspace_id)
            .unwrap()
            .tile_window(&window, position)?;

        if old_workspace_id.is_some() && old_workspace_id.unwrap() != new_workspace_id {
            let old_workspace = self.workspaces.get_mut(&old_workspace_id.unwrap()).unwrap();
            if let InsertResult::Swap(new_window) = result {
                old_workspace.replace_window(&window, &new_window)?;
            } else {
                old_workspace.remove_window(&window)?;
            }
            old_workspace.flush_windows()?;
        }

        let new_workspace = self.workspaces.get_mut(&new_workspace_id).unwrap();
        new_workspace.flush_windows()?;

        // Save layout after tiling
        if let Err(e) = save_layout(self) {
            warn!("Warning: Failed to save layout: {}", e);
        }

        Ok(())
    }

    pub fn remove_window(&mut self, id: WindowId) -> WMResult<()> {
        let window = self.get_window(id)?;
        let workspace = self.get_workspace_for_window_mut(&id)?;

        workspace.remove_window(&window)?;
        workspace.flush_windows()?;
        self.try_save_layout();
        Ok(())
    }

    pub fn resize_window(
        &mut self,
        id: WindowId,
        bounds: &Bounds,
        direction: ResizeDirection,
    ) -> WMResult<()> {
        let window = self.get_window(id)?;
        let workspace = self.get_workspace_for_window_mut(&id)?;

        workspace.resize_window(&window, bounds, direction)?;
        workspace.flush_windows()?;
        self.try_save_layout();
        Ok(())
    }

    pub fn get_window(&self, id: WindowId) -> WMResult<WindowRef> {
        self.windows
            .get(&id)
            .cloned()
            .ok_or(WMError::WindowNotFound(id))
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

    fn get_workspace_at_position(&self, position: &Position) -> WMResult<&Workspace> {
        // First, determine which partition the position is in
        let partition = self
            .partitions
            .values()
            .find(|p| p.bounds().contains(&position))
            .ok_or(WMError::NoWorkspaceAtPosition(position.clone()))?;

        // Then, get the workspace for that partition
        self.workspaces
            .get(&partition.current_workspace().unwrap())
            .ok_or(WMError::NoWorkspaceAtPosition(position.clone()))
    }

    fn get_workspace_at_position_mut(&mut self, position: &Position) -> WMResult<&mut Workspace> {
        // First, determine which partition the position is in
        let partition = self
            .partitions
            .values()
            .find(|p| p.bounds().contains(&position))
            .ok_or(WMError::NoWorkspaceAtPosition(position.clone()))?;

        // Then, get the workspace for that partition
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

    /// If the position is on the edge a window, that window is returned.
    pub fn find_window_at_resize_edge(&self, position: &Position) -> Option<WindowRef> {
        let thickness = 15;
        for window in self.windows.values() {
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
        buttons: &MouseButtons,
    ) -> WMResult<()> {
        if let Ok(workspace) = self.get_workspace_at_position_mut(position) {
            if workspace.resize_handle_moved(handle, position, buttons) {
                workspace.flush_windows()?;
                self.try_save_layout();
            }
        }
        Ok(())
    }

    pub fn cleanup(&mut self) -> PlatformResult<()> {
        Ok(())
    }

    fn try_save_layout(&self) {
        if let Err(e) = save_layout(self) {
            warn!("Failed to save layout: {e}");
        }
    }
}
