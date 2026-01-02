use crate::layouts::ContainerTree;
use crate::layouts::WindowLayout;
use crate::partition::{Partition, PartitionId};
use crate::paths;
use crate::platform::{Bounds, WindowId};
use crate::window::WindowRef;
use crate::wm::WindowManager;
use crate::workspace::{Workspace, WorkspaceId};
use crate::Config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct SerializedWindowManager {
    pub partitions: Vec<SerializedPartition>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedPartition {
    pub id: PartitionId,
    pub name: String,
    pub bounds: Bounds,
    pub workspaces: Vec<SerializedWorkspace>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedWorkspace {
    pub id: WorkspaceId,
    pub name: String,
    pub layout: serde_yaml::Value,
    pub floating: Vec<SerializedWindow>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedWindow {
    pub id: WindowId,
    pub bounds: Bounds,
}

fn serialize_wm(wm: &WindowManager) -> serde_yaml::Value {
    let serialized = SerializedWindowManager {
        partitions: wm
            .partitions()
            .iter()
            .map(|(_, partition)| SerializedPartition {
                id: partition.id(),
                name: partition.name().to_string(),
                bounds: partition.bounds().clone(),
                workspaces: partition
                    .assigned_workspaces()
                    .iter()
                    .map(|id| {
                        let workspace = wm.workspaces().get(id).unwrap();
                        SerializedWorkspace {
                            id: workspace.id(),
                            name: workspace.name().to_string(),
                            layout: workspace.serialize(),
                            floating: workspace
                                .windows()
                                .iter()
                                .filter(|(_, window)| window.floating())
                                .map(|(id, window)| SerializedWindow {
                                    id: id.clone(),
                                    bounds: window.bounds().clone(),
                                })
                                .collect(),
                        }
                    })
                    .collect(),
            })
            .collect(),
    };

    serde_yaml::to_value(serialized).unwrap()
}

pub fn deserialize_partition(
    serialized: &SerializedPartition,
    available_windows: &Vec<WindowRef>,
) -> (Partition, HashMap<WorkspaceId, Workspace>) {
    let mut partition = Partition::new(serialized.name.clone(), serialized.bounds.clone());
    let mut workspaces = HashMap::new();

    for serialized_workspace in &serialized.workspaces {
        let workspace = deserialize_workspace(serialized_workspace, &partition, available_windows);
        let id = workspace.id();
        workspaces.insert(id, workspace);
        partition.assign_workspace(id)
    }

    (partition, workspaces)
}

pub fn deserialize_workspace(
    serialized: &SerializedWorkspace,
    partition: &Partition,
    available_windows: &Vec<WindowRef>,
) -> Workspace {
    let layout = Box::new(ContainerTree::deserialize(
        partition.bounds().clone(),
        available_windows,
        &serialized.layout,
    ));

    let mut floating = HashMap::new();
    for window in available_windows.iter() {
        if serialized.floating.iter().any(|w| w.id == window.id()) {
            floating.insert(window.id(), window.clone());
        }
    }

    let workspace = Workspace::new_with_id::<ContainerTree>(
        serialized.id,
        partition.bounds().clone(),
        serialized.name.clone(),
        Some(layout),
        Some(floating),
    );

    workspace
}

/// Extract window IDs from saved layout for matching
pub fn extract_window_ids(layout: &serde_yaml::Value) -> Vec<WindowId> {
    let mut window_ids = Vec::new();
    extract_window_ids_recursive(layout, &mut window_ids);
    window_ids
}

fn extract_window_ids_recursive(value: &serde_yaml::Value, window_ids: &mut Vec<WindowId>) {
    match value {
        serde_yaml::Value::Tagged(tagged) => {
            // Handle tagged values like !Window and !Container
            // Recurse into the inner value
            extract_window_ids_recursive(&tagged.value, window_ids);
        }
        serde_yaml::Value::Mapping(map) => {
            // Check if this is a window object with an ID
            if let Some(id_value) = map.get(&serde_yaml::Value::String("id".to_string())) {
                if let Some(id) = id_value.as_u64() {
                    window_ids.push(id as WindowId);
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

/// Save the current window manager layout to file
pub fn save_layout(wm: &WindowManager) -> Result<(), Box<dyn std::error::Error>> {
    if !Config::persistence() {
        return Ok(());
    }

    let layout_data = serialize_wm(wm);
    let layout_yaml = serde_yaml::to_string(&layout_data)?;

    if let Some(path) = paths::layout_file_path() {
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
pub fn load_layout() -> Result<Option<SerializedWindowManager>, Box<dyn std::error::Error>> {
    if !Config::persistence() {
        return Ok(None);
    }

    if let Some(path) = paths::layout_file_path() {
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let layout: SerializedWindowManager = serde_yaml::from_str(&contents)?;
            return Ok(Some(layout));
        }
    }
    Ok(None)
}

pub fn reset_layout() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = paths::layout_file_path() {
        if path.exists() {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::{Mapping, Number, Value};

    fn create_test_yaml() -> Value {
        let mut root = Mapping::new();
        let mut partitions = Vec::new();

        // Create first partition
        let mut partition1 = Mapping::new();
        partition1.insert(
            Value::String("name".to_string()),
            Value::String("Main".to_string()),
        );

        let mut workspaces1 = Vec::new();
        let mut workspace1 = Mapping::new();
        workspace1.insert(
            Value::String("name".to_string()),
            Value::String("Workspace1".to_string()),
        );

        let mut layout1 = Mapping::new();
        layout1.insert(
            Value::String("type".to_string()),
            Value::String("container".to_string()),
        );
        let mut window1 = Mapping::new();
        window1.insert(
            Value::String("id".to_string()),
            Value::Number(Number::from(12345u64)),
        );
        layout1.insert(Value::String("window".to_string()), Value::Mapping(window1));

        workspace1.insert(Value::String("layout".to_string()), Value::Mapping(layout1));
        workspaces1.push(Value::Mapping(workspace1));

        partition1.insert(
            Value::String("workspaces".to_string()),
            Value::Sequence(workspaces1),
        );
        partitions.push(Value::Mapping(partition1));

        // Create second partition
        let mut partition2 = Mapping::new();
        partition2.insert(
            Value::String("name".to_string()),
            Value::String("Secondary".to_string()),
        );

        let mut workspaces2 = Vec::new();
        let mut workspace2 = Mapping::new();
        workspace2.insert(
            Value::String("name".to_string()),
            Value::String("Workspace2".to_string()),
        );

        let mut layout2 = Mapping::new();
        layout2.insert(
            Value::String("type".to_string()),
            Value::String("split".to_string()),
        );
        let mut children = Vec::new();

        let mut child1 = Mapping::new();
        child1.insert(
            Value::String("id".to_string()),
            Value::Number(Number::from(67890u64)),
        );
        children.push(Value::Mapping(child1));

        let mut child2 = Mapping::new();
        child2.insert(
            Value::String("id".to_string()),
            Value::Number(Number::from(11111u64)),
        );
        children.push(Value::Mapping(child2));

        layout2.insert(
            Value::String("children".to_string()),
            Value::Sequence(children),
        );
        workspace2.insert(Value::String("layout".to_string()), Value::Mapping(layout2));
        workspaces2.push(Value::Mapping(workspace2));

        partition2.insert(
            Value::String("workspaces".to_string()),
            Value::Sequence(workspaces2),
        );
        partitions.push(Value::Mapping(partition2));

        root.insert(
            Value::String("partitions".to_string()),
            Value::Sequence(partitions),
        );
        Value::Mapping(root)
    }

    #[test]
    fn test_extract_window_ids_simple() {
        let mut layout = Mapping::new();
        layout.insert(
            Value::String("id".to_string()),
            Value::Number(Number::from(12345u64)),
        );
        let yaml = Value::Mapping(layout);

        let ids = extract_window_ids(&yaml);
        assert_eq!(ids, vec![12345]);
    }

    #[test]
    fn test_extract_window_ids_nested() {
        let yaml = create_test_yaml();
        let ids = extract_window_ids(&yaml);

        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&12345));
        assert!(ids.contains(&67890));
        assert!(ids.contains(&11111));
    }

    #[test]
    fn test_extract_window_ids_empty() {
        let yaml = Value::Mapping(Mapping::new());
        let ids = extract_window_ids(&yaml);
        assert!(ids.is_empty());
    }

    #[test]
    fn test_extract_window_ids_no_ids() {
        let mut layout = Mapping::new();
        layout.insert(
            Value::String("type".to_string()),
            Value::String("container".to_string()),
        );
        layout.insert(
            Value::String("name".to_string()),
            Value::String("test".to_string()),
        );
        let yaml = Value::Mapping(layout);

        let ids = extract_window_ids(&yaml);
        assert!(ids.is_empty());
    }

    #[test]
    fn test_extract_window_ids_sequence() {
        let mut windows = Vec::new();

        let mut window1 = Mapping::new();
        window1.insert(
            Value::String("id".to_string()),
            Value::Number(Number::from(1111u64)),
        );
        windows.push(Value::Mapping(window1));

        let mut window2 = Mapping::new();
        window2.insert(
            Value::String("id".to_string()),
            Value::Number(Number::from(2222u64)),
        );
        windows.push(Value::Mapping(window2));

        let yaml = Value::Sequence(windows);
        let ids = extract_window_ids(&yaml);

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&1111));
        assert!(ids.contains(&2222));
    }

    #[test]
    fn test_extract_window_ids_mixed_types() {
        let mut root = Mapping::new();
        root.insert(
            Value::String("id".to_string()),
            Value::Number(Number::from(1111u64)),
        );
        root.insert(
            Value::String("name".to_string()),
            Value::String("test".to_string()),
        );
        root.insert(
            Value::String("count".to_string()),
            Value::Number(Number::from(42)),
        );
        root.insert(Value::String("enabled".to_string()), Value::Bool(true));

        let mut children = Vec::new();
        let mut child = Mapping::new();
        child.insert(
            Value::String("id".to_string()),
            Value::Number(Number::from(2222u64)),
        );
        children.push(Value::Mapping(child));

        root.insert(
            Value::String("children".to_string()),
            Value::Sequence(children),
        );

        let yaml = Value::Mapping(root);
        let ids = extract_window_ids(&yaml);

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&1111));
        assert!(ids.contains(&2222));
    }

    #[test]
    fn test_layout_file_path() {
        let path = paths::layout_file_path();

        // Should return a path (assuming dirs crate works in test environment)
        if let Some(path) = path {
            assert!(path.to_string_lossy().contains("UltraWM"));
            assert!(path.to_string_lossy().ends_with("layout.yaml"));
        }
        // If dirs doesn't work in test env, path might be None, which is also valid
    }

    #[test]
    fn test_extract_window_ids_recursive_deep_nesting() {
        let mut level3 = Mapping::new();
        level3.insert(
            Value::String("id".to_string()),
            Value::Number(Number::from(3333u64)),
        );

        let mut level2 = Mapping::new();
        level2.insert(
            Value::String("id".to_string()),
            Value::Number(Number::from(2222u64)),
        );
        level2.insert(Value::String("nested".to_string()), Value::Mapping(level3));

        let mut level1 = Mapping::new();
        level1.insert(
            Value::String("id".to_string()),
            Value::Number(Number::from(1111u64)),
        );
        level1.insert(Value::String("child".to_string()), Value::Mapping(level2));

        let yaml = Value::Mapping(level1);
        let ids = extract_window_ids(&yaml);

        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&1111));
        assert!(ids.contains(&2222));
        assert!(ids.contains(&3333));
    }

    #[test]
    fn test_extract_window_ids_invalid_id_types() {
        let mut layout = Mapping::new();
        // String ID should be ignored
        layout.insert(
            Value::String("id".to_string()),
            Value::String("not-a-number".to_string()),
        );
        // Boolean ID should be ignored
        layout.insert(Value::String("other_id".to_string()), Value::Bool(true));
        // Valid numeric ID should be extracted
        layout.insert(
            Value::String("valid_id".to_string()),
            Value::Number(Number::from(4444u64)),
        );

        let yaml = Value::Mapping(layout);
        let ids = extract_window_ids(&yaml);

        // Should only find the valid numeric ID (but it's not keyed as "id")
        assert!(ids.is_empty()); // Because only "id" key is checked, not "valid_id"
    }

    #[test]
    fn test_serialized_structures_serde() {
        // Test that our serialized structures can be serialized and deserialized
        let workspace = SerializedWorkspace {
            id: 0,
            name: "Test Workspace".to_string(),
            layout: Value::String("test layout".to_string()),
            floating: vec![],
        };

        let partition = SerializedPartition {
            id: 0,
            name: "Test Partition".to_string(),
            bounds: Bounds::new(0, 0, 1920, 1080),
            workspaces: vec![workspace],
        };

        let wm = SerializedWindowManager {
            partitions: vec![partition],
        };

        // Should serialize without error
        let serialized = serde_yaml::to_string(&wm).unwrap();
        assert!(serialized.contains("Test Partition"));
        assert!(serialized.contains("Test Workspace"));

        // Should deserialize back to the same structure
        let deserialized: SerializedWindowManager = serde_yaml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.partitions.len(), 1);
        assert_eq!(deserialized.partitions[0].name, "Test Partition");
        assert_eq!(deserialized.partitions[0].workspaces.len(), 1);
        assert_eq!(
            deserialized.partitions[0].workspaces[0].name,
            "Test Workspace"
        );
    }
}
