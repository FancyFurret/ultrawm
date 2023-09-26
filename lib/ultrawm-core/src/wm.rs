use crate::config::{Config, ConfigRef};
use crate::layouts::{ContainerTree, WindowLayout};
use crate::partition::{Partition, PartitionId};
use crate::platform::{
    Bounds, Platform, PlatformImpl, PlatformResult, PlatformWindow, PlatformWindowImpl,
};
use crate::window::Window;
use crate::workspace::{Workspace, WorkspaceId};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct WindowManager {
    partitions: HashMap<PartitionId, Partition>,
    workspaces: HashMap<WorkspaceId, Workspace>,
    config: ConfigRef,
}

impl WindowManager {
    pub fn new() -> PlatformResult<Self> {
        let config = Rc::new(Config::default());

        let displays = Platform::list_all_displays()?;

        // For now, just make 1 partition per display. Will be configurable later.
        let partitions: HashMap<PartitionId, Partition> = displays
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
        let workspaces: HashMap<WorkspaceId, Workspace> = partitions
            .values()
            .map(|partition| {
                let windows = Self::get_windows_in_partition(&mut windows, partition);
                let layout = ContainerTree::new(
                    config.clone(),
                    partition.bounds().clone(),
                    windows.iter().map(Window::new).collect(),
                )
                .expect("Could not create layout");
                let workspace = Workspace::new(Box::new(layout), "Default".to_string());
                (workspace.id(), workspace)
            })
            .collect();

        for workspace in workspaces.values() {
            workspace.flush()?;
        }

        Ok(Self {
            config,
            partitions,
            workspaces,
        })
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
}
