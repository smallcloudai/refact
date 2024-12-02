use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::ChatMessage;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinksPost {
    messages: Vec<ChatMessage>,
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
    let _post = serde_json::from_slice::<LinksPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let mut links = Vec::new();

    if project_summarization_is_missing(gcx.clone()).await {
        links.push(Link {
            action: LinkAction::SummarizeProject,
            text: "Investigate Project".to_string(),
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