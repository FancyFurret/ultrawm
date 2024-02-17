use crate::platform::Bounds;
use crate::wm::WindowManager;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SerializedWindowManager {
    partitions: Vec<SerializedPartition>,
}

#[derive(Serialize, Deserialize)]
struct SerializedPartition {
    name: String,
    bounds: Bounds,
    workspaces: Vec<SerializedWorkspace>,
}

#[derive(Serialize, Deserialize)]
struct SerializedWorkspace {
    name: String,
    layout: serde_yaml::Value,
}

pub fn serialize_wm(wm: &WindowManager) -> serde_yaml::Value {
    let serialized = SerializedWindowManager {
        partitions: wm
            .partitions()
            .iter()
            .map(|(_, partition)| SerializedPartition {
                name: partition.name().to_string(),
                bounds: partition.bounds().clone(),
                workspaces: partition
                    .assigned_workspaces()
                    .iter()
                    .map(|id| {
                        let workspace = wm.workspaces().get(id).unwrap();
                        SerializedWorkspace {
                            name: workspace.name().to_string(),
                            layout: workspace.serialize(),
                        }
                    })
                    .collect(),
            })
            .collect(),
    };

    serde_yaml::to_value(serialized).unwrap()
}
