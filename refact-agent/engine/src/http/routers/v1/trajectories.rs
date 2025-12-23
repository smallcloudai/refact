use std::path::PathBuf;
use std::sync::Arc;
use axum::extract::Path;
use axum::http::{Response, StatusCode};
use axum::Extension;
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use tokio::sync::broadcast;
use tokio::fs;
use tracing::{info, warn};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::ChatMessage;
use crate::custom_error::ScratchError;
use crate::global_context::{GlobalContext, try_load_caps_quickly_if_not_present};
use crate::files_correction::get_project_dirs;
use crate::subchat::subchat_single;

const TRAJECTORIES_FOLDER: &str = ".refact/trajectories";
const TITLE_GENERATION_PROMPT: &str = "Summarize this chat in 2-4 words. Prefer filenames, classes, entities, and avoid generic terms. Write only the title, nothing else.";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrajectoryEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrajectoryMeta {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub model: String,
    pub mode: String,
    pub message_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrajectoryData {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub model: String,
    pub mode: String,
    pub tool_use: String,
    pub messages: Vec<serde_json::Value>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

async fn get_trajectories_dir(gcx: Arc<ARwLock<GlobalContext>>) -> Result<PathBuf, String> {
    let project_dirs = get_project_dirs(gcx).await;
    let workspace_root = project_dirs.first().ok_or("No workspace folder found")?;
    Ok(workspace_root.join(TRAJECTORIES_FOLDER))
}

fn validate_trajectory_id(id: &str) -> Result<(), ScratchError> {
    if id.contains('/') || id.contains('\\') || id.contains("..") || id.contains('\0') {
        return Err(ScratchError::new(StatusCode::BAD_REQUEST, "Invalid trajectory id".to_string()));
    }
    Ok(())
}

async fn atomic_write_json(path: &PathBuf, data: &impl Serialize) -> Result<(), String> {
    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(&tmp_path, &json).await.map_err(|e| e.to_string())?;
    fs::rename(&tmp_path, path).await.map_err(|e| e.to_string())?;
    Ok(())
}

fn is_placeholder_title(title: &str) -> bool {
    let normalized = title.trim().to_lowercase();
    normalized.is_empty() || normalized == "new chat" || normalized == "untitled"
}

fn extract_first_user_message(messages: &[serde_json::Value]) -> Option<String> {
    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        if role != "user" {
            continue;
        }

        // Handle string content
        if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.chars().take(200).collect());
            }
        }

        // Handle array content (multimodal)
        if let Some(content_arr) = msg.get("content").and_then(|c| c.as_array()) {
            for item in content_arr {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.chars().take(200).collect());
                    }
                }
                if let Some(text) = item.get("m_content").and_then(|t| t.as_str()) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.chars().take(200).collect());
                    }
                }
            }
        }
    }
    None
}

fn build_title_generation_context(messages: &[serde_json::Value]) -> String {
    let mut context = String::new();
    let max_messages = 6;
    let max_chars_per_message = 500;

    for (i, msg) in messages.iter().take(max_messages).enumerate() {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("unknown");

        // Skip tool messages and context files for title generation
        if role == "tool" || role == "context_file" || role == "cd_instruction" {
            continue;
        }

        let content_text = if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
            content.to_string()
        } else if let Some(content_arr) = msg.get("content").and_then(|c| c.as_array()) {
            content_arr.iter()
                .filter_map(|item| {
                    item.get("text").and_then(|t| t.as_str())
                        .or_else(|| item.get("m_content").and_then(|t| t.as_str()))
                })
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            continue;
        };

        let truncated: String = content_text.chars().take(max_chars_per_message).collect();
        if !truncated.trim().is_empty() {
            context.push_str(&format!("{}: {}\n\n", role, truncated));
        }

        if i >= max_messages - 1 {
            break;
        }
    }

    context
}

fn clean_generated_title(raw_title: &str) -> String {
    let cleaned = raw_title
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('`')
        .trim_matches('*')
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Limit to ~60 chars
    if cleaned.chars().count() > 60 {
        cleaned.chars().take(57).collect::<String>() + "..."
    } else {
        cleaned
    }
}

async fn generate_title_llm(
    gcx: Arc<ARwLock<GlobalContext>>,
    messages: &[serde_json::Value],
) -> Option<String> {
    let caps = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => caps,
        Err(e) => {
            warn!("Failed to load caps for title generation: {:?}", e);
            return None;
        }
    };

    // Use light model if available, otherwise default
    let model_id = if !caps.defaults.chat_light_model.is_empty() {
        caps.defaults.chat_light_model.clone()
    } else {
        caps.defaults.chat_default_model.clone()
    };

    if model_id.is_empty() {
        warn!("No model available for title generation");
        return None;
    }

    let context = build_title_generation_context(messages);
    if context.trim().is_empty() {
        return None;
    }

    let prompt = format!("Chat conversation:\n{}\n\n{}", context, TITLE_GENERATION_PROMPT);

    let ccx = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        2048,
        5,
        false,
        vec![],
        "title-generation".to_string(),
        false,
        model_id.clone(),
    ).await));

    let chat_messages = vec![
        ChatMessage::new("user".to_string(), prompt),
    ];

    match subchat_single(
        ccx,
        &model_id,
        chat_messages,
        Some(vec![]),  // No tools
        Some("none".to_string()),  // No tool choice
        false,
        Some(0.3),  // Low temperature for consistent titles
        Some(50),   // Max tokens - titles should be short
        1,          // n=1
        None,       // No reasoning effort
        false,      // No system prompt
        None,       // No usage collector
        None,       // No tool id
        None,       // No chat id
    ).await {
        Ok(results) => {
            if let Some(messages) = results.first() {
                if let Some(last_msg) = messages.last() {
                    let raw_title = last_msg.content.content_text_only();
                    let cleaned = clean_generated_title(&raw_title);
                    if !cleaned.is_empty() && cleaned.to_lowercase() != "new chat" {
                        info!("Generated title: {}", cleaned);
                        return Some(cleaned);
                    }
                }
            }
            None
        }
        Err(e) => {
            warn!("Title generation failed: {}", e);
            None
        }
    }
}

async fn spawn_title_generation_task(
    gcx: Arc<ARwLock<GlobalContext>>,
    id: String,
    messages: Vec<serde_json::Value>,
    trajectories_dir: PathBuf,
) {
    tokio::spawn(async move {
        // Generate title via LLM
        let generated_title = generate_title_llm(gcx.clone(), &messages).await;

        let title = match generated_title {
            Some(t) => t,
            None => {
                // Fallback to truncated first user message
                match extract_first_user_message(&messages) {
                    Some(first_msg) => {
                        let truncated: String = first_msg.chars().take(60).collect();
                        if truncated.len() < first_msg.len() {
                            format!("{}...", truncated.trim_end())
                        } else {
                            truncated
                        }
                    }
                    None => return, // No title to generate
                }
            }
        };

        // Read current trajectory data
        let file_path = trajectories_dir.join(format!("{}.json", id));
        let content = match fs::read_to_string(&file_path).await {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read trajectory for title update: {}", e);
                return;
            }
        };

        let mut data: TrajectoryData = match serde_json::from_str(&content) {
            Ok(d) => d,
            Err(e) => {
                warn!("Failed to parse trajectory for title update: {}", e);
                return;
            }
        };

        // Update title and mark as generated
        data.title = title.clone();
        data.extra.insert("isTitleGenerated".to_string(), serde_json::json!(true));

        // Write back
        if let Err(e) = atomic_write_json(&file_path, &data).await {
            warn!("Failed to write trajectory with generated title: {}", e);
            return;
        }

        info!("Updated trajectory {} with generated title: {}", id, title);

        // Emit SSE event with new title
        let event = TrajectoryEvent {
            event_type: "updated".to_string(),
            id: id.clone(),
            updated_at: Some(data.updated_at.clone()),
            title: Some(title),
        };

        if let Some(tx) = &gcx.read().await.trajectory_events_tx {
            let _ = tx.send(event);
        }
    });
}

pub async fn handle_v1_trajectories_list(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
) -> Result<Response<Body>, ScratchError> {
    let trajectories_dir = get_trajectories_dir(gcx).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let mut result: Vec<TrajectoryMeta> = Vec::new();

    if trajectories_dir.exists() {
        let mut entries = fs::read_dir(&trajectories_dir).await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&path).await {
                if let Ok(data) = serde_json::from_str::<TrajectoryData>(&content) {
                    result.push(TrajectoryMeta {
                        id: data.id,
                        title: data.title,
                        created_at: data.created_at,
                        updated_at: data.updated_at,
                        model: data.model,
                        mode: data.mode,
                        message_count: data.messages.len(),
                    });
                }
            }
        }
    }

    result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&result).unwrap()))
        .unwrap())
}

pub async fn handle_v1_trajectories_get(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Path(id): Path<String>,
) -> Result<Response<Body>, ScratchError> {
    validate_trajectory_id(&id)?;

    let trajectories_dir = get_trajectories_dir(gcx).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let file_path = trajectories_dir.join(format!("{}.json", id));

    if !file_path.exists() {
        return Err(ScratchError::new(StatusCode::NOT_FOUND, "Trajectory not found".to_string()));
    }

    let content = fs::read_to_string(&file_path).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(content))
        .unwrap())
}

pub async fn handle_v1_trajectories_save(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Path(id): Path<String>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    validate_trajectory_id(&id)?;

    let data: TrajectoryData = serde_json::from_slice(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    if data.id != id {
        return Err(ScratchError::new(StatusCode::BAD_REQUEST, "ID mismatch".to_string()));
    }

    let trajectories_dir = get_trajectories_dir(gcx.clone()).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    fs::create_dir_all(&trajectories_dir).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let file_path = trajectories_dir.join(format!("{}.json", id));
    let is_new = !file_path.exists();

    // Check if we need to generate a title
    let is_title_generated = data.extra.get("isTitleGenerated")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let should_generate_title = is_placeholder_title(&data.title)
        && !is_title_generated
        && !data.messages.is_empty();

    atomic_write_json(&file_path, &data).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let event = TrajectoryEvent {
        event_type: if is_new { "created".to_string() } else { "updated".to_string() },
        id: id.clone(),
        updated_at: Some(data.updated_at.clone()),
        title: if is_new { Some(data.title.clone()) } else { None },
    };

    if let Some(tx) = &gcx.read().await.trajectory_events_tx {
        let _ = tx.send(event);
    }

    // Spawn async title generation if needed
    if should_generate_title {
        spawn_title_generation_task(
            gcx.clone(),
            id.clone(),
            data.messages.clone(),
            trajectories_dir,
        ).await;
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"status":"ok"}"#))
        .unwrap())
}

pub async fn handle_v1_trajectories_delete(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Path(id): Path<String>,
) -> Result<Response<Body>, ScratchError> {
    validate_trajectory_id(&id)?;

    let trajectories_dir = get_trajectories_dir(gcx.clone()).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let file_path = trajectories_dir.join(format!("{}.json", id));

    if !file_path.exists() {
        return Err(ScratchError::new(StatusCode::NOT_FOUND, "Trajectory not found".to_string()));
    }

    fs::remove_file(&file_path).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let event = TrajectoryEvent {
        event_type: "deleted".to_string(),
        id: id.clone(),
        updated_at: None,
        title: None,
    };

    if let Some(tx) = &gcx.read().await.trajectory_events_tx {
        let _ = tx.send(event);
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"status":"ok"}"#))
        .unwrap())
}

pub async fn handle_v1_trajectories_subscribe(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
) -> Result<Response<Body>, ScratchError> {
    let rx = {
        let gcx_locked = gcx.read().await;
        match &gcx_locked.trajectory_events_tx {
            Some(tx) => tx.subscribe(),
            None => return Err(ScratchError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "Trajectory events not available".to_string()
            )),
        }
    };

    let stream = async_stream::stream! {
        let mut rx = rx;
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let json = serde_json::to_string(&event).unwrap_or_default();
                    yield Ok::<_, std::convert::Infallible>(format!("data: {}\n\n", json));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(Body::wrap_stream(stream))
        .unwrap())
}
