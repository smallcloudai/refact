use std::path::PathBuf;
use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tracing::error;

use crate::agentic::generate_commit_message::generate_commit_message_by_diff;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::ChatMessage;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::integrations::go_to_configuration_message;
use crate::subchat::subchat_single;
use crate::tools::tool_patch_aux::tickets_parsing::get_tickets_from_messages;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinksPost {
    messages: Vec<ChatMessage>,
    model_name: String,
    chat_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
enum LinkAction {
    PatchAll,
    FollowUp,
    Commit,
    Goto,
    SummarizeProject,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Link {
    action: LinkAction,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    goto: Option<String>,
}

pub async fn handle_v1_links(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<LinksPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let mut links = Vec::new();

    if post.messages.is_empty() && project_summarization_is_missing(gcx.clone()).await {
        links.push(Link {
            action: LinkAction::SummarizeProject,
            text: "Investigate Project".to_string(),
            goto: None,
        });
    }

    // TODO: Only do this for configuration chats, detect it in system prompt.
    if !get_tickets_from_messages(gcx.clone(), &post.messages).await.is_empty() {
        links.push(Link {
            action: LinkAction::PatchAll,
            text: "Save and return".to_string(),
            goto: Some("SETTINGS:DEFAULT".to_string()),
        });
    }

    // TODO: Only do this for "Agent" chat.
    if let Ok(diff) = get_diff_with_all_changes_in_current_project(gcx.clone()).await.map_err(|e| error!(e)) {
        if let Ok(commit_msg) = generate_commit_message_by_diff(gcx.clone(), &diff, &None).await.map_err(|e| error!(e)) {
            links.push(Link {
                action: LinkAction::Commit,
                text: format!("git commit -m \"{}\"", commit_msg),
                goto: None,
            });
        }
    }

    // TODO: This probably should not appear in configuration chats, unless we can know that this is not the main one being configured
    for failed_tool_name in failed_tool_names_after_last_user_message(&post.messages) {
        links.push(Link {
            action: LinkAction::Goto,
            text: format!("Configure {failed_tool_name}"),
            goto: Some(format!("SETTINGS:{failed_tool_name}")),
        })
    }

    // TODO: Only do this for "Explore", "Agent" or configuration chats, detect it in system prompt.
    if links.is_empty() {
        let follow_up_message = generate_follow_up_message(post.messages.clone(), gcx.clone(), &post.model_name, &post.chat_id).await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error generating follow-up message: {}", e)))?;
        links.push(Link {
            action: LinkAction::FollowUp,
            text: follow_up_message,
            goto: None,
        });
    }
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::json!({"links": links}).to_string()))
        .unwrap())
}

async fn get_diff_with_all_changes_in_current_project(gcx: Arc<ARwLock<GlobalContext>>) -> Result<String, String> {
    let active_project_path = get_active_project_path(gcx.clone()).await.ok_or("No active project found".to_string())?;
    let repository = git2::Repository::open(&active_project_path).map_err(|e| e.to_string())?;
    crate::git::git_diff_from_all_changes(&repository)
}

async fn get_active_project_path(gcx: Arc<ARwLock<GlobalContext>>) -> Option<PathBuf> {
    let active_file = gcx.read().await.documents_state.active_file_path.clone();
    let workspace_folders = crate::files_correction::get_project_dirs(gcx.clone()).await;
    if workspace_folders.is_empty() { return None; }

    Some(crate::files_in_workspace::detect_vcs_for_a_file_path(
        &active_file.unwrap_or_else(|| workspace_folders[0].clone())
    ).await.map(|(path, _)| path).unwrap_or_else(|| workspace_folders[0].clone()))
}

async fn project_summarization_is_missing(gcx: Arc<ARwLock<GlobalContext>>) -> bool {
    match get_active_project_path(gcx.clone()).await {
        Some(active_project_path) => {
            !active_project_path.join(".refact").join("project_summary.yaml").exists()
        }
        None => {
            tracing::info!("No projects found, project summarization is not relevant.");
            false
        }
    }
}

fn failed_tool_names_after_last_user_message(messages: &Vec<ChatMessage>) -> Vec<String> {
    let last_user_msg_index = messages.iter().rposition(|m| m.role == "user").unwrap_or(0);
    let tool_calls = messages[last_user_msg_index..].iter().filter(|m| m.role == "assistant")
        .filter_map(|m| m.tool_calls.as_ref()).flatten().collect::<Vec<_>>();

    let mut result = Vec::new();
    for tool_call in tool_calls {
        if let Some(answer_text) = messages.iter()
            .find(|m| m.role == "tool" && m.tool_call_id == tool_call.id)
            .map(|m| m.content.content_text_only()) {
            if answer_text.contains(&go_to_configuration_message(&tool_call.function.name)) {
                result.push(tool_call.function.name.clone());
            }
        }
    }
    result.sort();
    result.dedup();
    result
}

async fn generate_follow_up_message(
    mut messages: Vec<ChatMessage>, 
    gcx: Arc<ARwLock<GlobalContext>>, 
    model_name: &str, 
    chat_id: &str,
) -> Result<String, String> {
    if messages.first().map(|m| m.role == "system").unwrap_or(false) {
        messages.remove(0);
    }
    messages.insert(0, ChatMessage::new(
        "system".to_string(),
        "Generate a 2-3 word user response, like 'Can you fix it?' for errors or 'Proceed' for plan validation".to_string(),
    ));
    let ccx = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        1024,
        1,
        false,
        messages.clone(),
        chat_id.to_string(),
        false,
    ).await));
    let new_messages = subchat_single(
        ccx.clone(),
        model_name,
        messages,
        vec![],
        None,
        false,
        Some(0.5),
        None,
        1,
        None,
        None,
        None,
    ).await?;
    new_messages.into_iter().next().map(|x| x.into_iter().last().map(|last_m| {
        last_m.content.content_text_only() })).flatten().ok_or("No commit message found".to_string())
}