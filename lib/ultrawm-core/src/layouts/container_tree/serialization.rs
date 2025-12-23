use crate::layouts::container_tree::container::{
    Container, ContainerChildRef, ContainerRef, ContainerWindow, ContainerWindowRef,
};
use crate::layouts::{ContainerId, Direction};
use crate::platform::{Bounds, PlatformWindowImpl, WindowId};
use crate::window::WindowRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct SerializedContainerTree {
    pub root: SerializedContainer,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedContainer {
    #[serde(default)]
    pub id: ContainerId,
    pub direction: Direction,
    pub ratios: Vec<f32>,
    pub children: Vec<SerializedContainerChild>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SerializedContainerChild {
    Container(SerializedContainer),
    Window(SerializedWindow),
}

#[derive(Serialize, Deserialize)]
pub struct SerializedWindow {
    pub id: WindowId,
}

pub fn serialize_container(container: &ContainerRef) -> SerializedContainer {
    SerializedContainer {
        id: container.id(),
        direction: container.direction(),
        ratios: container.ratios().clone(),
        children: container
            .children()
            .iter()
            .map(|child| match child {
                ContainerChildRef::Container(container) => {
                    SerializedContainerChild::Container(serialize_container(container))
                }
                ContainerChildRef::Window(window) => {
                    SerializedContainerChild::Window(serialize_window(window))
                }
            })
            .collect(),
    }
}

fn serialize_window(window: &ContainerWindowRef) -> SerializedWindow {
    SerializedWindow {
        id: window.window().platform_window().id(),
    }
}

pub(crate) fn deserialize_container(
    serialized: &SerializedContainer,
    bounds: Bounds,
    available_windows: &HashMap<WindowId, WindowRef>,
    windows_map: &mut HashMap<WindowId, ContainerWindowRef>,
    parent: Option<ContainerRef>,
) -> Option<ContainerRef> {
    if serialized.children.is_empty() {
        return None;
    }

    let parent_ref = parent.map(|p| p.self_ref());
    let container = Container::new(bounds.clone(), serialized.direction, parent_ref.clone());

    // Start with the saved ratios and track which ones to remove
    let mut ratios = serialized.ratios.clone();
    let mut indices_to_remove = Vec::new();

    for (index, child) in serialized.children.iter().enumerate() {
        match child {
            SerializedContainerChild::Container(child_container) => {
                // Use parent bounds as placeholder - will be recalculated from ratios
                if let Some(child) = deserialize_container(
                    child_container,
                    bounds.clone(),
                    available_windows,
                    windows_map,
                    Some(container.clone()),
                ) {
                    // Check how many children the returned container has
                    let child_count = child.children().len();

                    if child_count == 0 {
                        indices_to_remove.push(index);
                    } else if child_count == 1 && parent_ref.is_some() {
                        let single_child = child.children()[0].clone();
                        match single_child {
                            ContainerChildRef::Container(grandchild_container) => {
                                container.add_container(grandchild_container);
                            }
                            ContainerChildRef::Window(grandchild_window) => {
                                container.add_window(grandchild_window);
                            }
                        }
                    } else {
                        container.add_container(child);
                    }
                } else {
                    indices_to_remove.push(index);
                }
            }
            SerializedContainerChild::Window(window_data) => {
                if let Some(window_ref) = available_windows.get(&window_data.id) {
                    let container_window = ContainerWindow::new(window_ref.clone());
                    let window_ref = container.add_window(container_window);
                    windows_map.insert(window_data.id, window_ref);
                } else {
                    // Window doesn't exist - mark ratio for removal
                    indices_to_remove.push(index);
                }
            }
        }
    }

    // Remove ratios in reverse order to avoid index shifting
    for &index in indices_to_remove.iter().rev() {
        if index < ratios.len() {
            ratios.remove(index);
        }
    }

    // Apply the remaining ratios
    container.set_ratios(ratios);

    // If no children were successfully added, return None
    if container.children().is_empty() {
        return None;
    }

    Some(container)
}
