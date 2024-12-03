use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::ChatMessage;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::integrations::go_to_configuration_message;
use crate::subchat::subchat_single;

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

    if project_summarization_is_missing(gcx.clone()).await && post.messages.is_empty() {
        links.push(Link {
            action: LinkAction::SummarizeProject,
            text: "Investigate Project".to_string(),
            goto: None,
        });
    }

    for failed_tool_name in failed_tool_names_after_last_user_message(&post.messages) {
        links.push(Link {
            action: LinkAction::Goto,
            text: format!("Configure {failed_tool_name}"),
            goto: Some(format!("SETTINGS:{failed_tool_name}")),
        })
    }

    // TODO: Only do this for "Explore", "Agent" or configuration chats.
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

async fn project_summarization_is_missing(gcx: Arc<ARwLock<GlobalContext>>) -> bool {
    let active_file = gcx.read().await.documents_state.active_file_path.clone();
    let workspace_folders = crate::files_correction::get_project_dirs(gcx.clone()).await;
    if workspace_folders.is_empty() {
        tracing::info!("No projects found, project summarization is not relevant.");
        return false;
    }

    let (active_project_path, _) = crate::files_in_workspace::detect_vcs_for_a_file_path(&active_file.unwrap_or_default())
        .await.unwrap_or_else(|| (workspace_folders.first().unwrap().clone(), ""));

    !active_project_path.join(".refact").join("project_summary.yaml").exists()
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