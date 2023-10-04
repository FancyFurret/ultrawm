use crate::platform::Bounds;
use crate::workspace::WorkspaceId;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};

pub type PartitionId = usize;

#[derive(Debug)]
pub struct Partition {
    id: PartitionId,
    name: String,
    bounds: Bounds,
    current_workspace: Option<WorkspaceId>,
    assigned_workspaces: HashSet<WorkspaceId>,
}

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

impl Partition {
    pub fn new(name: String, bounds: Bounds) -> Self {
        let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {
            id,
            name,
            bounds,
            current_workspace: None,
            assigned_workspaces: HashSet::new(),
        }
    }

    pub fn id(&self) -> PartitionId {
        self.id
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    pub fn current_workspace(&self) -> Option<WorkspaceId> {
        self.current_workspace
    }

    pub fn assigned_workspaces(&self) -> &HashSet<WorkspaceId> {
        &self.assigned_workspaces
    }

    pub fn assign_workspace(&mut self, workspace_id: WorkspaceId) {
        self.current_workspace.get_or_insert(workspace_id);
        self.assigned_workspaces.insert(workspace_id);
    }
}
