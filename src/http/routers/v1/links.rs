use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;
use tracing::error;

use crate::agentic::generate_commit_message::generate_commit_message_by_diff;
use crate::call_validation::{ChatMessage, ChatMeta, ChatMode};
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::integrations::go_to_configuration_message;
use crate::tools::tool_patch_aux::tickets_parsing::get_tickets_from_messages;
use crate::agentic::generate_follow_up_message::generate_follow_up_message;

#[derive(Deserialize, Clone, Debug)]
pub struct LinksPost {
    messages: Vec<ChatMessage>,
    model_name: String,
    meta: ChatMeta,
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

    if post.meta.chat_mode == ChatMode::Configure && !get_tickets_from_messages(gcx.clone(), &post.messages).await.is_empty() {
        links.push(Link {
            action: LinkAction::PatchAll,
            text: "Save and return".to_string(),
            goto: Some("SETTINGS:DEFAULT".to_string()),
        });
    }

    if post.meta.chat_mode == ChatMode::Agent {
        if let Ok(commit_msg) = generate_commit_messages_with_current_changes(gcx.clone())
            .await.map_err(|e| error!(e)) {
            links.push(Link {
                action: LinkAction::Commit,
                text: format!("git commit -m \"{}\"", commit_msg),
                goto: None,
            });
        }
    }

    if post.meta.chat_mode != ChatMode::Configure {
        for failed_integr_name in failed_integration_names_after_last_user_message(&post.messages) {
            links.push(Link {
                action: LinkAction::Goto,
                text: format!("Configure {failed_integr_name}"),
                goto: Some(format!("SETTINGS:{failed_integr_name}")),
            })
        }
    }

    if post.meta.chat_mode != ChatMode::NoTools && links.is_empty() {
        let follow_up_message = generate_follow_up_message(post.messages.clone(), gcx.clone(), &post.model_name, &post.meta.chat_id).await
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

async fn generate_commit_messages_with_current_changes(gcx: Arc<ARwLock<GlobalContext>>) -> Result<String, String> {
    let active_project_path = crate::files_correction::get_active_project_path(gcx.clone()).await.ok_or("No active project found".to_string())?;
    let repository = git2::Repository::open(&active_project_path).map_err(|e| e.to_string())?;
    let diff = crate::git::git_diff_from_all_changes(&repository)?;
    let commit_msg = generate_commit_message_by_diff(gcx.clone(), &diff, &None).await.map_err(|e| e.to_string())?;
    Ok(commit_msg)
}

// TODO: Move all logic below to more appropiate files
async fn project_summarization_is_missing(gcx: Arc<ARwLock<GlobalContext>>) -> bool {
    match crate::files_correction::get_active_project_path(gcx.clone()).await {
        Some(active_project_path) => {
            !active_project_path.join(".refact").join("project_summary.yaml").exists()
        }
        None => {
            tracing::info!("No projects found, project summarization is not relevant.");
            false
        }
    }
}

fn failed_integration_names_after_last_user_message(messages: &Vec<ChatMessage>) -> Vec<String> {
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