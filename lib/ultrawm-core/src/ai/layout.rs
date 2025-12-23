use crate::ai::client::{strip_markdown_code_block, AiClient, AiClientError, ChatMessage};
use crate::config::Config;
use crate::event_loop_wm::{WMOperationError, WMOperationResult};
use crate::layouts::PlacementTarget;
use crate::partition::PartitionId;
use crate::platform::WindowId;
use crate::wm::{WMError, WindowManager};
use crate::workspace::WorkspaceId;
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AiLayoutError {
    #[error(transparent)]
    Client(#[from] AiClientError),
    #[error("Failed to parse response: {0}")]
    ParseError(String),
}

#[derive(Debug, Serialize)]
struct AiPartitionState {
    id: PartitionId,
    name: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_id: Option<WorkspaceId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    layout: Option<serde_yaml::Value>,
}

#[derive(Debug, Serialize)]
struct AiWindowInfo {
    id: WindowId,
    title: String,
}

impl AiWindowInfo {
    fn new(id: WindowId, title: String) -> Self {
        Self { id, title }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiLayoutResponse {
    pub partitions: Vec<AiPartitionLayout>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiPartitionLayout {
    pub id: PartitionId,
    pub layout: serde_yaml::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiSingleWindowResponse {
    #[serde(flatten)]
    pub placement: WindowPlacement,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum WindowPlacement {
    Place {
        workspace_id: WorkspaceId,
        target: PlacementTarget,
    },
    Float,
}

pub fn handle_organize_all_windows(wm: &WindowManager) -> WMOperationResult<()> {
    let windows: Vec<AiWindowInfo> = wm
        .get_all_windows()
        .iter()
        .filter(|w| !w.title().is_empty())
        .map(|w| AiWindowInfo::new(w.id(), w.title()))
        .collect();

    let partitions = wm
        .partitions()
        .iter()
        .map(|(_, p)| AiPartitionState {
            id: p.id(),
            name: p.name().to_string(),
            x: p.bounds().position.x,
            y: p.bounds().position.y,
            width: p.bounds().size.width,
            height: p.bounds().size.height,
            workspace_id: None,
            layout: None,
        })
        .collect();

    let workspace = wm.workspaces().values().next().unwrap();
    let example_layout = workspace.layout().example_layout();
    let layout_description = workspace.layout().layout_description();
    let user_preferences = Config::ai().organization_preferences;

    // Build partition_id -> workspace_id mapping before moving into async
    let partition_to_workspace: HashMap<_, _> = wm
        .partitions()
        .iter()
        .filter_map(|(pid, p)| p.current_workspace().map(|wid| (*pid, wid)))
        .collect();

    tokio::spawn(async move {
        match organize_all_windows_async(
            windows,
            partitions,
            example_layout,
            layout_description,
            user_preferences,
        )
        .await
        {
            Ok(response) => {
                for partition_layout in &response.partitions {
                    if let Some(workspace_id) = partition_to_workspace.get(&partition_layout.id) {
                        crate::load_layout_to_workspace(
                            *workspace_id,
                            partition_layout.layout.clone(),
                        );
                    } else {
                        error!("Partition {} not found", partition_layout.id);
                    }
                }
            }
            Err(e) => {
                error!("AI error: {}", e);
            }
        }
    });

    Ok(())
}

/// Organize a single window using AI. Takes WindowManager and window_id directly.
pub fn handle_organize_single_window(
    wm: &WindowManager,
    window_id: WindowId,
) -> WMOperationResult<()> {
    let window_ref = wm.get_window(window_id)?;

    // Build window ID to title mapping
    let window_titles: HashMap<WindowId, String> = wm
        .get_all_windows()
        .iter()
        .filter(|w| !w.title().is_empty())
        .map(|w| (w.id(), w.title()))
        .collect();

    // Collect all partitions with their current workspace layouts
    let mut partitions: Vec<AiPartitionState> = Vec::new();
    for (_, partition) in wm.partitions() {
        let (workspace_id, layout) = if let Some(ws_id) = partition.current_workspace() {
            if let Some(workspace) = wm.workspaces().get(&ws_id) {
                let raw_layout = workspace.serialize();
                let enriched_layout = enrich_layout_with_titles(&raw_layout, &window_titles);
                (Some(ws_id), Some(enriched_layout))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        partitions.push(AiPartitionState {
            id: partition.id(),
            name: partition.name().to_string(),
            x: partition.bounds().position.x,
            y: partition.bounds().position.y,
            width: partition.bounds().size.width,
            height: partition.bounds().size.height,
            workspace_id,
            layout,
        });
    }

    // Get layout metadata from any workspace (they should all be the same type for now)
    let workspace = wm
        .workspaces()
        .values()
        .next()
        .ok_or(WMOperationError::Error(WMError::WorkspaceNotFound(0)))?;
    let layout_description = workspace.layout().layout_description();
    let placement_help = workspace.layout().placement_help();
    let user_preferences = Config::ai().organization_preferences;

    let window_info = AiWindowInfo::new(window_id, window_ref.title());

    tokio::spawn(async move {
        match organize_single_window_async(
            window_info,
            partitions,
            layout_description,
            placement_help,
            user_preferences,
        )
        .await
        {
            Ok(response) => match response.placement {
                WindowPlacement::Float => {
                    crate::float_window(window_id);
                }
                WindowPlacement::Place {
                    workspace_id,
                    target,
                } => {
                    crate::place_window_relative(window_id, target, workspace_id);
                }
            },
            Err(e) => {
                error!("AI error: {}", e);
            }
        }
    });

    Ok(())
}

async fn organize_all_windows_async(
    windows: Vec<AiWindowInfo>,
    partitions: Vec<AiPartitionState>,
    example_layout: serde_yaml::Value,
    layout_description: String,
    user_preferences: String,
) -> Result<AiLayoutResponse, AiLayoutError> {
    let client = AiClient::from_config()?;

    let example_yaml = serde_yaml::to_string(&example_layout).unwrap_or_default();

    // Build partition info
    let partitions_info: String = partitions
        .iter()
        .map(|p| {
            format!(
                "  - id: {}, name: {}, pos: ({}, {}), size: {}x{}",
                p.id, p.name, p.x, p.y, p.width, p.height
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Build windows info
    let windows_info: String = windows
        .iter()
        .map(|w| format!("  - id: {}, title: {}", w.id, w.title))
        .collect::<Vec<_>>()
        .join("\n");

    let system_prompt = format!(
        r#"Arrange windows into tiled layouts. Output YAML only. No markdown.

{layout_description}

Example layout format:
{example_yaml}

Response format:
partitions:
  - id: <partition_id>
    layout: <layout as above>

Rules: ratios sum to 1.0, output window IDs only, omit windows to float them, each window used at most once.

Partitions:
{partitions_info}

Windows:
{windows_info}
"#,
        layout_description = layout_description
    );

    let user_prompt = if user_preferences.is_empty() {
        "Organize these windows.".to_string()
    } else {
        user_preferences
    };

    info!("System prompt:\n{}", system_prompt);
    info!("User prompt: {}", user_prompt);

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    let response = client.chat(messages).await?;
    let response = strip_markdown_code_block(&response);
    info!("AI response:\n{}", response);

    // Parse the response as YAML
    let layout_response: AiLayoutResponse = serde_yaml::from_str(&response).map_err(|e| {
        AiLayoutError::ParseError(format!(
            "Failed to parse AI response as YAML: {}. Response was:\n{}",
            e, response
        ))
    })?;

    Ok(layout_response)
}

async fn organize_single_window_async(
    window: AiWindowInfo,
    partitions: Vec<AiPartitionState>,
    layout_description: String,
    placement_help: String,
    user_preferences: String,
) -> Result<AiSingleWindowResponse, AiLayoutError> {
    let client = AiClient::from_config()?;

    // Serialize partitions to YAML (includes layout info)
    let partitions_yaml = serde_yaml::to_string(&partitions).unwrap_or_default();

    let system_prompt = format!(
        r#"Place a window relative to an existing window or container in ANY partition. Output YAML only.

{layout_description}

{placement_help}

Available partitions (each includes workspace_id and layout if available):
{partitions_yaml}

Response format (choose one):
action: place
workspace_id: <workspace_id>
target: <placement_target> (format depends on layout - see placement help above)

OR

action: float

Rules:
- You must place the window in ANY workspace by specifying workspace_id
- Follow the placement format specified in the placement help above
- All fields (workspace_id, target) must be at the same indentation level as "action", not indented under it
"#,
        layout_description = layout_description,
        placement_help = placement_help
    );

    let user_prompt = if user_preferences.is_empty() {
        format!("Place window {} (title: {})", window.id, window.title)
    } else {
        format!(
            "{}\n\nPlace window {} (title: {})",
            user_preferences, window.id, window.title
        )
    };

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    let response = client.chat(messages).await?;
    let response = strip_markdown_code_block(&response);
    info!("AI single window response:\n{}", response);

    let placement_response: AiSingleWindowResponse =
        serde_yaml::from_str(&response).map_err(|e| {
            AiLayoutError::ParseError(format!(
                "Failed to parse AI response: {}. Response was:\n{}",
                e, response
            ))
        })?;

    Ok(placement_response)
}

fn enrich_layout_with_titles(layout: &Value, window_titles: &HashMap<WindowId, String>) -> Value {
    match layout {
        Value::Mapping(map) => {
            let mut enriched = serde_yaml::Mapping::new();
            let mut window_id: Option<WindowId> = None;

            // First pass: enrich all values and find window ID if present
            for (key, value) in map {
                let enriched_value = enrich_layout_with_titles(value, window_titles);
                enriched.insert(key.clone(), enriched_value);

                // Check if this is the "id" field of a window
                if let Some(id_key) = key.as_str() {
                    if id_key == "id" {
                        if let Some(id) = value.as_u64() {
                            window_id = Some(id as WindowId);
                        }
                    }
                }
            }

            // If we found a window ID, add the title to this mapping
            if let Some(id) = window_id {
                if let Some(title) = window_titles.get(&id) {
                    enriched.insert(
                        Value::String("title".to_string()),
                        Value::String(title.clone()),
                    );
                }
            }

            Value::Mapping(enriched)
        }
        Value::Sequence(seq) => {
            let enriched: Vec<Value> = seq
                .iter()
                .map(|v| enrich_layout_with_titles(v, window_titles))
                .collect();
            Value::Sequence(enriched)
        }
        Value::Tagged(tagged) => enrich_layout_with_titles(&tagged.value, window_titles),
        other => other.clone(),
    }
}
