use crate::platform::Bounds;
use crate::wm::WindowManager;
use crate::Config;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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

fn serialize_wm(wm: &WindowManager) -> serde_yaml::Value {
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

/// Extract layout data for a specific workspace from the loaded YAML
pub fn extract_workspace_layout(
    saved_data: &serde_yaml::Value,
    partition_name: &str,
    workspace_name: &str,
) -> Option<serde_yaml::Value> {
    // Navigate through the YAML structure to find the specific workspace layout
    if let Some(partitions) = saved_data.get("partitions").and_then(|p| p.as_sequence()) {
        for partition in partitions {
            if let Some(name) = partition.get("name").and_then(|n| n.as_str()) {
                if name == partition_name {
                    if let Some(workspaces) =
                        partition.get("workspaces").and_then(|w| w.as_sequence())
                    {
                        for workspace in workspaces {
                            if let Some(ws_name) = workspace.get("name").and_then(|n| n.as_str()) {
                                if ws_name == workspace_name {
                                    return workspace.get("layout").cloned();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Extract window IDs from saved layout for matching
pub fn extract_window_ids(layout: &serde_yaml::Value) -> Vec<crate::platform::WindowId> {
    let mut window_ids = Vec::new();
    extract_window_ids_recursive(layout, &mut window_ids);
    window_ids
}

fn extract_window_ids_recursive(
    value: &serde_yaml::Value,
    window_ids: &mut Vec<crate::platform::WindowId>,
) {
    match value {
        serde_yaml::Value::Mapping(map) => {
            // Check if this is a window object with an ID
            if let Some(id_value) = map.get(&serde_yaml::Value::String("id".to_string())) {
                if let Some(id) = id_value.as_u64() {
                    window_ids.push(id as crate::platform::WindowId);
                }
            }

            // Recursively search all values
            for (_, v) in map.iter() {
                extract_window_ids_recursive(v, window_ids);
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            for item in seq.iter() {
                extract_window_ids_recursive(item, window_ids);
            }
        }
        _ => {}
    }
}

/// Get the path where layout should be saved
pub fn layout_file_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join("UltraWM").join("layout.yaml"))
}

/// Save the current window manager layout to file
pub fn save_layout(wm: &WindowManager) -> Result<(), Box<dyn std::error::Error>> {
    if !Config::persistence() {
        return Ok(());
    }

    let layout_data = serialize_wm(wm);
    let layout_yaml = serde_yaml::to_string(&layout_data)?;

    if let Some(path) = layout_file_path() {
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&path, layout_yaml)?;
    } else {
        return Err("Could not determine layout file path".into());
    }

    Ok(())
}

/// Load layout from file if it exists
pub fn load_layout() -> Result<Option<serde_yaml::Value>, Box<dyn std::error::Error>> {
    if !Config::persistence() {
        return Ok(None);
    }

    if let Some(path) = layout_file_path() {
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let layout: serde_yaml::Value = serde_yaml::from_str(&contents)?;
            return Ok(Some(layout));
        }
    }
    Ok(None)
}

pub fn reset_layout() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = layout_file_path() {
        if path.exists() {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}
