use std::sync::Arc;
use chrono::{DateTime, Utc, Duration};
use serde_json::Value;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use tokio::fs;
use tracing::{info, warn};
use walkdir::WalkDir;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage};
use crate::files_correction::get_project_dirs;
use crate::global_context::{GlobalContext, try_load_caps_quickly_if_not_present};
use crate::memories::{memories_add, create_frontmatter};
use crate::subchat::subchat_single;

const ABANDONED_THRESHOLD_HOURS: i64 = 2;
const CHECK_INTERVAL_SECS: u64 = 300;
const TRAJECTORIES_FOLDER: &str = ".refact/trajectories";

const EXTRACTION_PROMPT: &str = r#"Analyze this conversation and extract separate, useful memory items that would help in future similar tasks.

For EACH distinct insight, output a JSON object on its own line with this format:
{"type": "<type>", "content": "<concise insight>"}

Types:
- pattern: Reusable code patterns or approaches discovered
- preference: User preferences about coding style, communication, tools
- lesson: What went wrong and how it was fixed
- decision: Important architectural or design decisions made
- insight: General useful observations about the codebase or project

Rules:
- Each insight should be self-contained and actionable
- Keep content concise (1-3 sentences max)
- Only extract genuinely useful, reusable knowledge
- Skip trivial details or conversation noise
- Output 3-10 items maximum

Example output:
{"type": "pattern", "content": "When implementing async file operations in this project, use tokio::fs instead of std::fs to avoid blocking."}
{"type": "preference", "content": "User prefers concise code without excessive comments."}
{"type": "lesson", "content": "The build failed because serde_json was missing from Cargo.toml dependencies."}
"#;

pub async fn trajectory_memos_background_task(gcx: Arc<ARwLock<GlobalContext>>) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(CHECK_INTERVAL_SECS)).await;

        if let Err(e) = process_abandoned_trajectories(gcx.clone()).await {
            warn!("trajectory_memos: error processing trajectories: {}", e);
        }
    }
}

async fn process_abandoned_trajectories(gcx: Arc<ARwLock<GlobalContext>>) -> Result<(), String> {
    let project_dirs = get_project_dirs(gcx.clone()).await;
    let workspace_root = match project_dirs.first() {
        Some(root) => root.clone(),
        None => return Ok(()),
    };

    let trajectories_dir = workspace_root.join(TRAJECTORIES_FOLDER);
    if !trajectories_dir.exists() {
        return Ok(());
    }

    let now = Utc::now();
    let threshold = now - Duration::hours(ABANDONED_THRESHOLD_HOURS);

    for entry in WalkDir::new(&trajectories_dir).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().map(|e| e != "json").unwrap_or(true) {
            continue;
        }

        match process_single_trajectory(gcx.clone(), path.to_path_buf(), &threshold).await {
            Ok(true) => info!("trajectory_memos: extracted memos from {}", path.display()),
            Ok(false) => {},
            Err(e) => warn!("trajectory_memos: failed to process {}: {}", path.display(), e),
        }
    }

    Ok(())
}

async fn process_single_trajectory(
    gcx: Arc<ARwLock<GlobalContext>>,
    path: std::path::PathBuf,
    threshold: &DateTime<Utc>,
) -> Result<bool, String> {
    let content = fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
    let mut trajectory: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    if trajectory.get("memo_extracted").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Ok(false);
    }

    let updated_at = trajectory.get("updated_at")
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let is_abandoned = match updated_at {
        Some(dt) => dt < *threshold,
        None => false,
    };

    if !is_abandoned {
        return Ok(false);
    }

    let messages = trajectory.get("messages")
        .and_then(|v| v.as_array())
        .ok_or("No messages")?;

    if messages.len() < 4 {
        return Ok(false);
    }

    let trajectory_id = trajectory.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let title = trajectory.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();

    let chat_messages = build_chat_messages(messages);
    if chat_messages.len() < 3 {
        return Ok(false);
    }

    let memos = extract_memos(gcx.clone(), chat_messages).await?;

    for memo in memos {
        let frontmatter = create_frontmatter(
            Some(&format!("[{}] {}", memo.memo_type, title)),
            &[memo.memo_type.clone(), "trajectory".to_string()],
            &[],
            &[],
            "trajectory",
        );

        let content_with_source = format!(
            "{}\n\n---\nSource: trajectory `{}`",
            memo.content,
            trajectory_id
        );

        if let Err(e) = memories_add(gcx.clone(), &frontmatter, &content_with_source).await {
            warn!("trajectory_memos: failed to save memo: {}", e);
        }
    }

    trajectory.as_object_mut()
        .ok_or("Invalid trajectory")?
        .insert("memo_extracted".to_string(), Value::Bool(true));

    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(&trajectory).map_err(|e| e.to_string())?;
    fs::write(&tmp_path, &json).await.map_err(|e| e.to_string())?;
    fs::rename(&tmp_path, &path).await.map_err(|e| e.to_string())?;

    Ok(true)
}

fn build_chat_messages(messages: &[Value]) -> Vec<ChatMessage> {
    messages.iter()
        .filter_map(|msg| {
            let role = msg.get("role").and_then(|v| v.as_str())?;
            if role == "context_file" || role == "cd_instruction" || role == "system" {
                return None;
            }

            let content = if let Some(c) = msg.get("content").and_then(|v| v.as_str()) {
                c.to_string()
            } else if let Some(arr) = msg.get("content").and_then(|v| v.as_array()) {
                arr.iter()
                    .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                return None;
            };

            if content.trim().is_empty() {
                return None;
            }

            Some(ChatMessage {
                role: role.to_string(),
                content: ChatContent::SimpleText(content.chars().take(3000).collect()),
                ..Default::default()
            })
        })
        .collect()
}

struct ExtractedMemo {
    memo_type: String,
    content: String,
}

async fn extract_memos(
    gcx: Arc<ARwLock<GlobalContext>>,
    mut messages: Vec<ChatMessage>,
) -> Result<Vec<ExtractedMemo>, String> {
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await
        .map_err(|e| e.message)?;

    let model_id = if caps.defaults.chat_light_model.is_empty() {
        caps.defaults.chat_default_model.clone()
    } else {
        caps.defaults.chat_light_model.clone()
    };

    let n_ctx = caps.chat_models.get(&model_id)
        .map(|m| m.base.n_ctx)
        .unwrap_or(4096);

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: ChatContent::SimpleText(EXTRACTION_PROMPT.to_string()),
        ..Default::default()
    });

    let ccx = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        n_ctx,
        1,
        false,
        messages.clone(),
        "".to_string(),
        false,
        model_id.clone(),
    ).await));

    let response = subchat_single(
        ccx, &model_id, messages, None, None, false, Some(0.0), None, 1, None, true, None, None, None,
    ).await.map_err(|e| e.to_string())?;

    let response_text = response.into_iter()
        .flatten()
        .last()
        .and_then(|m| match m.content {
            ChatContent::SimpleText(t) => Some(t),
            _ => None,
        })
        .unwrap_or_default();

    let memos: Vec<ExtractedMemo> = response_text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if !line.starts_with('{') {
                return None;
            }
            let parsed: Value = serde_json::from_str(line).ok()?;
            Some(ExtractedMemo {
                memo_type: parsed.get("type").and_then(|v| v.as_str())?.to_string(),
                content: parsed.get("content").and_then(|v| v.as_str())?.to_string(),
            })
        })
        .take(10)
        .collect();

    Ok(memos)
}
