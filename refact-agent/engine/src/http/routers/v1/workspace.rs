use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetActiveGroupIdPost {
    pub group_id: usize,
}


pub async fn handle_v1_set_active_group_id(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<SetActiveGroupIdPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    gcx.write().await.active_group_id = Some(post.group_id);
    
    Ok(Response::builder().status(StatusCode::OK).body(Body::from(
        serde_json::to_string(&serde_json::json!({ "success": true })).unwrap()
    )).unwrap())
}


pub async fn handle_v1_get_active_group_candidates(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let api_key = gcx.read().await.cmdline.api_key.clone();
    let project_info = crate::files_correction::get_active_project_path(gcx.clone()).await
        .map(|x| {
            let remotes = if let Some(remotes) = crate::git::operations::get_git_remotes(&x).ok() {
                remotes.into_iter().map(|(_, url)| url.to_string()).collect()
            } else {
                vec![]
            };
            Some(serde_json::json!({
                "repo_remotes": remotes,
                "local_path": x.to_string_lossy().to_string()
            }))
        })
        .unwrap_or(None);

    let url = "https://test-teams-v1.smallcloud.ai/v1/get-active-group-candidates".to_string();
    let body = serde_json::json!({"project_info": project_info});
    let response = reqwest::Client::new().get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let response_body = resp.text().await.map_err(|e| {
                    ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read response body: {}", e))
                })?;
                Ok(Response::builder().status(StatusCode::OK).body(Body::from(response_body)).unwrap())
            } else {
                let status = resp.status();
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(
                    ScratchError::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Cloud server error: HTTP status {}, error: {}", status, error_text))
                )
            }
        }
        Err(e) => Err(
            ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Cloud server unavailable: {:?}", e))
        )
    }
}
