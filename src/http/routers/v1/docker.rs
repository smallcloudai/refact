use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::integrations::docker::integr_docker::docker_tool_load;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DockerAction {
    Kill,
    Start,
    Remove,
    Stop,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DockerContainerActionPost {
    pub action: DockerAction,
    pub container: String,
}

pub async fn handle_v1_docker_container_action(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<DockerContainerActionPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let docker = docker_tool_load(gcx.clone()).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Cannot load docker tool: {}", e)))?;

    let docker_command = match post.action {
        DockerAction::Kill => format!("container kill {}", post.container),
        DockerAction::Start => format!("container start {}", post.container),
        DockerAction::Remove => format!("container remove --volumes {}", post.container),
        DockerAction::Stop => format!("container stop {}", post.container),
    };
    let (output, _) = docker.command_execute(&docker_command, gcx.clone(), true).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Command {} failed: {}", docker_command, e)))?;

    Ok(Response::builder().status(StatusCode::OK).body(Body::from(
        serde_json::to_string(&serde_json::json!({ "success": true, "output": output })).unwrap()
    )).unwrap())
}

pub async fn handle_v1_docker_container_list(
    Extension(_gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    todo!()
}