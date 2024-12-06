use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock as ARwLock;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::integrations::docker::docker_and_isolation_load;

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DockerContainerListPost {
    pub label: Option<String>,
    pub image: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DockerContainerListResponse {
    pub container_list: Vec<DockerContainerListOutput>,
    pub has_connection_to_docker_daemon: bool,
    pub docker_error: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DockerContainerListOutput {
    id: String,
    status: String,
    name: String,
    created: Option<String>,
    user: Option<String>,
    #[serde(default)]
    env: Vec<String>,
    #[serde(default)]
    command: Vec<String>,
    image: Option<String>,
    working_dir: Option<String>,
    labels: Value,
    ports: Value,
}

pub async fn handle_v1_docker_container_action(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<DockerContainerActionPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let (docker, _) = docker_and_isolation_load(gcx.clone()).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Cannot load docker tool: {}", e)))?;

    let docker_command = match post.action {
        DockerAction::Kill => format!("container kill {}", post.container),
        DockerAction::Start => format!("container start {}", post.container),
        DockerAction::Remove => format!("container remove --volumes {}", post.container),
        DockerAction::Stop => format!("container stop {}", post.container),
    };
    let (output, _) = docker.command_execute(&docker_command, gcx.clone(), true, false).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Command {} failed: {}", docker_command, e)))?;

    Ok(Response::builder().status(StatusCode::OK).body(Body::from(
        serde_json::to_string(&serde_json::json!({ "success": true, "output": output })).unwrap()
    )).unwrap())
}

pub async fn handle_v1_docker_container_list(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<DockerContainerListPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let docker = match docker_and_isolation_load(gcx.clone()).await {
        Ok((docker, _)) => docker,
        Err(e) => return Ok(docker_container_list_response(vec![], false, &e)),
    };

    let docker_command = match post.label {
        Some(label) => format!("container list --all --no-trunc --format json --filter label={label}"),
        None => "container list --all --no-trunc --format json".to_string(),
    };

    let unparsed_output = match docker.command_execute(&docker_command, gcx.clone(), true, false).await {
        Ok((unparsed_output, _)) => unparsed_output,
        Err(e) => return Ok(docker_container_list_response(vec![], false, &e)),
    };

    let mut output: Vec<Value> = unparsed_output.lines().map(|line| serde_json::from_str(line)).collect::<Result<Vec<_>, _>>()
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Container list JSON problem: {}", e)))?;
    
    if let Some(image) = post.image {
        output = output.into_iter().filter(|container| {
            container["Image"].as_str().map_or(false, |image_name| image_name.contains(&image))
        }).collect();
    }

    let container_ids = output.iter().map(|container| {
        container["ID"].as_str().map(|id| id.to_string())
            .ok_or_else(|| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Missing container ID in output:\n{:?}", output)))
    }).collect::<Result<Vec<String>, ScratchError>>()?;

    if container_ids.len() == 0 {
        return Ok(docker_container_list_response(vec![], true, ""));
    }

    let inspect_command = format!("container inspect --format json {}", container_ids.join(" "));
    let inspect_unparsed_output = match docker.command_execute(&inspect_command, gcx.clone(), true, false).await {
        Ok((inspect_unparsed_output, _)) => inspect_unparsed_output,
        Err(e) => return Ok(docker_container_list_response(vec![], false, &e)),
    };

    let inspect_output = serde_json::from_str::<Vec<serde_json::Value>>(&inspect_unparsed_output)
       .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Container inspect JSON problem: {}", e)))?;

    let response_body: Vec<DockerContainerListOutput> = inspect_output.into_iter()
        .map(|container| {
            let mut container_name = extract_string_field(&container, &["Name"], "Missing container name")?;
            if container_name.starts_with('/') { container_name = container_name[1..].to_string() };

            Ok(DockerContainerListOutput {
                id: extract_string_field(&container, &["Id"], "Missing container ID")?
                    .get(0..12).unwrap_or("").to_string(),
                name: container_name,
                status: extract_string_field(&container, &["State", "Status"], "Missing container status")?,
                created: container["Created"].as_str().map(ToString::to_string),
                user: container["Config"]["User"].as_str().map(ToString::to_string),
                env: extract_string_array_field(&container, &["Config", "Env"]),
                command: extract_string_array_field(&container, &["Config", "Cmd"]),
                image: container["Config"]["Image"].as_str().map(ToString::to_string),
                working_dir: container["Config"]["WorkingDir"].as_str().map(ToString::to_string),
                labels: container["Config"]["Labels"].clone(),
                ports: container["NetworkSettings"]["Ports"].clone(),
            })
        }).collect::<Result<Vec<_>, ScratchError>>()?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&serde_json::json!({"containers": response_body})).unwrap()))
        .unwrap())
}

fn docker_container_list_response(
    container_list: Vec<DockerContainerListOutput>, 
    has_connection_to_daemon: bool,
    error: &str, 
) -> Response<Body> {
    let response = DockerContainerListResponse {
        container_list,
        has_connection_to_docker_daemon: has_connection_to_daemon,
        docker_error: error.to_string(),
    };
    Response::builder().status(StatusCode::OK).header("Content-Type", "application/json")
           .body(Body::from(serde_json::to_string(&response).unwrap())).unwrap()
}

fn extract_string_field<'a>(container: &'a serde_json::Value, field_path: &[&str], error_message: &str) -> Result<String, ScratchError> {
    field_path.iter().fold(container, |acc, &key| &acc[key]).as_str().map(ToString::to_string)
        .ok_or_else(|| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}:\n{:?}", error_message, container)))
}

fn extract_string_array_field(container: &serde_json::Value, field_path: &[&str]) -> Vec<String> {
    field_path.iter().fold(container, |acc, &key| &acc[key]).as_array()
        .map(|arr| arr.iter().filter_map(|item| item.as_str().map(ToString::to_string)).collect())
        .unwrap_or_default()
}