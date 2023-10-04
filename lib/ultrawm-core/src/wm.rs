use crate::config::{Config, ConfigRef};
use crate::layouts::ContainerTree;
use crate::partition::{Partition, PartitionId};
use crate::platform::{
    Bounds, Platform, PlatformImpl, PlatformResult, PlatformWindow, PlatformWindowImpl, Position,
};
use crate::serialize::serialize_wm;
use crate::window::Window;
use crate::workspace::{Workspace, WorkspaceId};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct WindowManager {
    #[allow(dead_code)]
    config: ConfigRef,
    partitions: HashMap<PartitionId, Partition>,
    workspaces: HashMap<WorkspaceId, Workspace>,
}

impl WindowManager {
    pub fn new() -> PlatformResult<Self> {
        let config = Rc::new(Config::default());

        let displays = Platform::list_all_displays()?;

        // For now, just make 1 partition per display. Will be configurable later.
        let mut partitions: HashMap<PartitionId, Partition> = displays
            .into_iter()
            .map(|d| {
                let partition = Partition::new(d.name, d.work_area);
                (partition.id(), partition)
            })
            .collect();

        let mut windows = Platform::list_all_windows()?;

        // Sort by window id so that re-running the WM is more stable
        windows.sort_by_key(|w| w.id());

        // Also for now, just make 1 workspace per partition. Will be configurable later.
        let mut workspaces: HashMap<WorkspaceId, Workspace> = partitions
            .values_mut()
            .map(|partition| {
                let windows = Self::get_windows_in_partition(&mut windows, partition);
                let workspace = Workspace::new::<ContainerTree>(
                    config.clone(),
                    partition.bounds().clone(),
                    windows.iter().map(|w| Window::new(w.clone())).collect(),
                    "Default".to_string(),
                );
                partition.assign_workspace(workspace.id());
                (workspace.id(), workspace)
            })
            .collect();

        for workspace in workspaces.values_mut() {
            workspace.layout_mut().flush()?;
        }

        Ok(Self {
            config,
            partitions,
            workspaces,
        })
    }

    pub fn partitions(&self) -> &HashMap<PartitionId, Partition> {
        &self.partitions
    }

    pub fn workspaces(&self) -> &HashMap<WorkspaceId, Workspace> {
        &self.workspaces
    }

    fn get_windows_in_partition(
        windows: &mut Vec<PlatformWindow>,
        partition: &Partition,
    ) -> Vec<PlatformWindow> {
        let mut windows_in_partition = Vec::new();
        let mut i = 0;
        while i < windows.len() {
            let window = windows.get(i).unwrap();
            let window_bounds = Bounds::from_position(window.position(), window.size());
            if partition.bounds().intersects(&window_bounds) {
                windows_in_partition.push(windows.remove(i));
            } else {
                i += 1;
            }
        }

        windows_in_partition
    }

    pub fn get_tile_preview_for_position(
        &self,
        window: &PlatformWindow,
        position: &Position,
    ) -> Option<Bounds> {
        // First, determine which partition the mouse is in
        let partition = self
            .partitions
            .values()
            .find(|p| p.bounds().contains(position))?;

        // Then, get the workspace for that partition
        let workspace = self.workspaces.get(&partition.current_workspace()?)?;

        // Then, get the window layout for that workspace
        workspace
            .layout()
            .get_tile_preview_for_position(window, position)
    }

    pub fn insert_window_at_position(
        &mut self,
        window: &PlatformWindow,
        position: &Position,
    ) -> Result<(), ()> {
        // First, determine which partition the mouse is in
        let partition = self
            .partitions
            .values()
            .find(|p| p.bounds().contains(&position))
            .unwrap();

        // Then, get the workspace for that partition
        let workspace = self
            .workspaces
            .get_mut(&partition.current_workspace().ok_or(())?)
            .ok_or(())?;

        // Then, get the window layout for that workspace
        workspace
            .layout_mut()
            .insert_window_at_position(window, position)
    }

    pub fn flush_windows(&mut self) -> PlatformResult<()> {
        for partition in self.partitions.values() {
            let workspace = self
                .workspaces
                .get_mut(&partition.current_workspace().ok_or(())?)
                .ok_or(())?;

            workspace.layout_mut().flush()?;
        }

        Ok(())
    }

    pub fn serialize(&self) -> serde_yaml::Value {
        serialize_wm(self)
    }
}
