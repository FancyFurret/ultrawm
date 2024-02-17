use crate::layouts::container_tree::container::{
    ContainerChildRef, ContainerRef, ContainerWindowRef,
};
use crate::layouts::ContainerTree;
use crate::platform::{Bounds, PlatformWindowImpl, WindowId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SerializedContainerTree {
    root: SerializedContainer,
    bounds: Bounds,
}

#[derive(Serialize, Deserialize)]
struct SerializedContainer {
    bounds: Bounds,
    children: Vec<SerializedContainerChild>,
}

#[derive(Serialize, Deserialize)]
enum SerializedContainerChild {
    Container(SerializedContainer),
    Window(SerializedWindow),
}

#[derive(Serialize, Deserialize)]
struct SerializedWindow {
    bounds: Bounds,
    id: WindowId,
    title: String,
}

pub fn serialize_tree(tree: &ContainerTree) -> serde_yaml::Value {
    let serialized = SerializedContainerTree {
        root: serialize_container(&tree.root()),
        bounds: tree.bounds(),
    };

    serde_yaml::to_value(serialized).unwrap()
}

fn serialize_container(container: &ContainerRef) -> SerializedContainer {
    SerializedContainer {
        bounds: container.bounds(),
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
        bounds: window.bounds(),
        id: window.window().platform_window().id(),
        title: window.window().platform_window().title().to_string(),
    }
}
