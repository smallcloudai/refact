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

const EXTRACTION_PROMPT: &str = r#"Analyze this conversation and provide:

1. FIRST LINE: A JSON with overview and title:
{"overview": "<2-3 sentence summary of what was accomplished>", "title": "<2-4 word descriptive title>"}

2. FOLLOWING LINES: Extract separate, useful memory items (3-10 max):
{"type": "<type>", "content": "<concise insight>"}

Types for memory items:
- pattern: Reusable code patterns or approaches discovered
- preference: User preferences about coding style, communication, tools
- lesson: What went wrong and how it was fixed
- decision: Important architectural or design decisions made
- insight: General useful observations about the codebase or project

Rules:
- Overview should capture the main goal and outcome
- Title should be descriptive and specific (e.g., "Fix Auth Middleware" not "Bug Fix")
- Each memory item should be self-contained and actionable
- Keep content concise (1-3 sentences max)
- Only extract genuinely useful, reusable knowledge
- Skip trivial details or conversation noise

Example output:
{"overview": "Implemented a custom VecDB splitter for trajectory files to enable semantic search over past conversations. Added two new tools for searching and retrieving trajectory context.", "title": "Trajectory Search Tools"}
{"type": "pattern", "content": "When implementing async file operations in this project, use tokio::fs instead of std::fs to avoid blocking."}
{"type": "preference", "content": "User prefers concise code without excessive comments."}
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

    if messages.len() < 10 {
        return Ok(false);
    }

    let trajectory_id = trajectory.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let current_title = trajectory.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();

    let is_title_generated = trajectory.get("extra")
        .and_then(|e| e.get("isTitleGenerated"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let chat_messages = build_chat_messages(messages);

    let extraction = extract_memos_and_meta(gcx.clone(), chat_messages, &current_title, is_title_generated).await?;

    let traj_obj = trajectory.as_object_mut().ok_or("Invalid trajectory")?;

    if let Some(ref meta) = extraction.meta {
        traj_obj.insert("overview".to_string(), Value::String(meta.overview.clone()));
        if is_title_generated && !meta.title.is_empty() {
            traj_obj.insert("title".to_string(), Value::String(meta.title.clone()));
            info!("trajectory_memos: updated title '{}' -> '{}' for {}", current_title, meta.title, trajectory_id);
        }
    }

    let memo_title = extraction.meta.as_ref()
        .filter(|_| is_title_generated)
        .map(|m| m.title.clone())
        .unwrap_or(current_title);

    for memo in extraction.memos {
        let frontmatter = create_frontmatter(
            Some(&format!("[{}] {}", memo.memo_type, memo_title)),
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

    traj_obj.insert("memo_extracted".to_string(), Value::Bool(true));

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
            if role == "context_file" || role == "cd_instruction" {
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

struct TrajectoryMeta {
    overview: String,
    title: String,
}

struct ExtractionResult {
    meta: Option<TrajectoryMeta>,
    memos: Vec<ExtractedMemo>,
}

async fn extract_memos_and_meta(
    gcx: Arc<ARwLock<GlobalContext>>,
    mut messages: Vec<ChatMessage>,
    current_title: &str,
    is_title_generated: bool,
) -> Result<ExtractionResult, String> {
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

    let title_hint = if is_title_generated {
        format!("\n\nNote: The current title \"{}\" was auto-generated. Please provide a better descriptive title.", current_title)
    } else {
        String::new()
    };

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: ChatContent::SimpleText(format!("{}{}", EXTRACTION_PROMPT, title_hint)),
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
        ccx, &model_id, messages, None, None, false, Some(0.0), None, 1, None, false, None, None, None,
    ).await.map_err(|e| e.to_string())?;

    let response_text = response.into_iter()
        .flatten()
        .last()
        .and_then(|m| match m.content {
            ChatContent::SimpleText(t) => Some(t),
            _ => None,
        })
        .unwrap_or_default();

    let mut meta: Option<TrajectoryMeta> = None;
    let mut memos: Vec<ExtractedMemo> = Vec::new();

    for line in response_text.lines() {
        let line = line.trim();
        if !line.starts_with('{') {
            continue;
        }

        let parsed: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let (Some(overview), Some(title)) = (
            parsed.get("overview").and_then(|v| v.as_str()),
            parsed.get("title").and_then(|v| v.as_str()),
        ) {
            meta = Some(TrajectoryMeta {
                overview: overview.to_string(),
                title: title.to_string(),
            });
            continue;
        }

        if let (Some(memo_type), Some(content)) = (
            parsed.get("type").and_then(|v| v.as_str()),
            parsed.get("content").and_then(|v| v.as_str()),
        ) {
            if memos.len() < 10 {
                memos.push(ExtractedMemo {
                    memo_type: memo_type.to_string(),
                    content: content.to_string(),
                });
            }
        }
    }

    Ok(ExtractionResult { meta, memos })
}
