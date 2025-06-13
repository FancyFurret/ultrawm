use crate::config::Config;
use crate::drag_handle::DragHandle;
use crate::layouts::{ContainerTree, ResizeDirection};
use crate::partition::{Partition, PartitionId};
use crate::platform::{
    Bounds, Platform, PlatformImpl, PlatformResult, PlatformWindow, PlatformWindowImpl, Position,
    WindowId,
};
use crate::serialize::serialize_wm;
use crate::window::{Window, WindowRef};
use crate::workspace::{Workspace, WorkspaceId};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct WindowManager {
    windows: HashMap<WindowId, WindowRef>,
    partitions: HashMap<PartitionId, Partition>,
    workspaces: HashMap<WorkspaceId, Workspace>,
}

impl WindowManager {
    pub fn new() -> PlatformResult<Self> {
        // TODO: Load from file
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

        // Also for now, just make 1 workspace per partition. Will be configurable later.
        let mut workspaces: HashMap<WorkspaceId, Workspace> = partitions
            .values_mut()
            .map(|partition| {
                let windows = Self::get_windows_in_bounds(&mut windows, partition.bounds());
                let workspace = Workspace::new::<ContainerTree>(
                    partition.bounds().clone(),
                    &windows,
                    "Default".to_string(),
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

    pub fn track_window(&mut self, window: PlatformWindow) -> Result<(), ()> {
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

    pub fn tile_window(&mut self, id: WindowId, position: &Position) -> Result<(), ()> {
        let window = self.get_window(id).ok_or(())?;
        let workspace = self.get_workspace_at_position_mut(position).ok_or(())?;
        workspace.tile_window(&window, position)?;
        workspace.flush_windows()
    }

    pub fn remove_window(&mut self, id: WindowId) -> Result<(), ()> {
        let window = self.get_window(id).ok_or(())?;

        for workspace in self.workspaces.values_mut() {
            if workspace.remove_window(&window).is_ok() {
                self.windows.remove(&id);
                workspace.flush_windows()?;
                return Ok(());
            }
        }

        Err(())
    }

    pub fn resize_window(
        &mut self,
        window: &WindowRef,
        bounds: &Bounds,
        direction: ResizeDirection,
    ) -> Result<(), ()> {
        for workspace in self.workspaces.values_mut() {
            if workspace.has_window(window.id()) {
                workspace.resize_window(window, bounds, direction);
                workspace.flush_windows()?;
                return Ok(());
            }
        }

        Err(())
    }

    pub fn get_window(&self, id: WindowId) -> Option<WindowRef> {
        self.windows.get(&id).cloned()
    }

    pub fn get_tile_bounds(&self, id: WindowId, position: &Position) -> Option<Bounds> {
        let workspace = self.get_workspace_at_position(position)?;
        let window = self.get_window(id)?;
        workspace.get_tile_bounds(&window, position)
    }

    pub fn serialize(&self) -> serde_yaml::Value {
        serialize_wm(self)
    }

    fn get_workspace_at_position(&self, position: &Position) -> Option<&Workspace> {
        // First, determine which partition the position is in
        let partition = self
            .partitions
            .values()
            .find(|p| p.bounds().contains(&position))?;

        // Then, get the workspace for that partition
        self.workspaces.get(&partition.current_workspace()?)
    }

    fn get_workspace_at_position_mut(&mut self, position: &Position) -> Option<&mut Workspace> {
        // First, determine which partition the position is in
        let partition = self
            .partitions
            .values()
            .find(|p| p.bounds().contains(&position))?;

        // Then, get the workspace for that partition
        self.workspaces.get_mut(&partition.current_workspace()?)
    }

    fn get_windows_in_bounds(windows: &mut Vec<WindowRef>, bounds: &Bounds) -> Vec<WindowRef> {
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

    /// Returns a list of drag handles for the workspace that covers the given position.
    pub fn drag_handles(&self, position: &Position) -> Vec<DragHandle> {
        if let Some(workspace) = self.get_workspace_at_position(position) {
            workspace.drag_handles()
        } else {
            Vec::new()
        }
    }

    /// Finds the first drag handle that contains the given position (if any).
    pub fn drag_handle_at_position(&self, position: &Position) -> Option<DragHandle> {
        let thickness = Config::drag_handle_width() as i32;
        self.drag_handles(position)
            .into_iter()
            .find(|h| match h.orientation {
                crate::drag_handle::HandleOrientation::Vertical => {
                    let dx = (position.x - h.center.x).abs();
                    let dy = (position.y - h.center.y).abs();
                    dx <= thickness / 2 && dy <= h.length as i32 / 2
                }
                crate::drag_handle::HandleOrientation::Horizontal => {
                    let dx = (position.x - h.center.x).abs();
                    let dy = (position.y - h.center.y).abs();
                    dy <= thickness / 2 && dx <= h.length as i32 / 2
                }
            })
    }

    pub fn cleanup(&mut self) -> PlatformResult<()> {
        Ok(())
    }
}
