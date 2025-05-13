use std::sync::Arc;
use axum::{Extension, extract::Query};
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock as ARwLock;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::integrations::mcp;
use crate::integrations::mcp::session_mcp::{fetch_and_update_mcp_resources, SessionMCP};
use crate::integrations::sessions::get_session_hashmap_key;
use crate::call_validation::ChatMessage;

#[derive(Serialize)]
pub struct McpServerDesc {
    pub name: String,
    pub config_path: String,
    pub num_tools: usize,
    pub num_resources: usize,
}

#[derive(Deserialize)]
pub struct McpResourceQuery {
    pub config_path: String,
}

#[derive(Deserialize)]
pub struct McpResourceContentQuery {
    pub config_path: String,
    pub uri: String,
}

#[derive(Serialize)]
pub struct ResourceContentResponse {
    pub messages: Vec<ChatMessage>,
}

pub async fn handle_mcp_servers(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let gcx_locked = gcx.read().await;
    let mut servers = Vec::new();
    for (key, session_arc) in gcx_locked.integration_sessions.iter() {
        if !key.starts_with("mcp ⚡") {
            continue;
        }
        let mut session_locked = session_arc.lock().await;
        let session_mcp = session_locked.as_any_mut().downcast_mut::<SessionMCP>()
            .ok_or_else(|| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "Failed to downcast session".to_string()))?;

        let name = std::path::Path::new(&session_mcp.config_path)
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown");

        servers.push(McpServerDesc {
            name: name.to_string(),
            config_path: session_mcp.config_path.clone(),
            num_tools: session_mcp.mcp_tools.len(),
            num_resources: session_mcp.mcp_resources.as_ref().map_or(0, |r| r.len()),
        });
    }
    let body = serde_json::to_string_pretty(&servers).expect("Failed to serialize servers");
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap())
}

pub async fn handle_mcp_resources(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Query(query): Query<McpResourceQuery>,
) -> Result<Response<Body>, ScratchError> {
    let session_key = get_session_hashmap_key("mcp", &query.config_path);

    let session_arc = {
        let gcx_locked = gcx.read().await;
        gcx_locked.integration_sessions.get(&session_key)
            .ok_or_else(|| ScratchError::new(StatusCode::NOT_FOUND, format!("No session for key {}", session_key)))?
            .clone()
    };

    let resources = fetch_and_update_mcp_resources(session_arc)
        .await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let body = serde_json::to_string_pretty(&resources).expect("Failed to serialize resources");
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap())
}

pub async fn handle_mcp_resource_content(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Query(query): Query<McpResourceContentQuery>,
) -> Result<Response<Body>, ScratchError> {
    let session_key = get_session_hashmap_key("mcp", &query.config_path);

    let session_arc = {
        let gcx_locked = gcx.read().await;
        gcx_locked.integration_sessions.get(&session_key)
            .ok_or_else(|| ScratchError::new(StatusCode::NOT_FOUND, format!("No session for key {}", session_key)))?
            .clone()
    };

    let resource_contents = mcp::mcp_resources::read_resource(session_arc, query.uri.clone())
        .await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let elements = mcp::mcp_resources::convert_resource_contents_to_multimodal_elements(resource_contents);
    let message = ChatMessage::from_multimodal_elements("user".to_string(), elements);

    let response = ResourceContentResponse { messages: vec![message] };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string_pretty(&response).unwrap()))
        .unwrap())
}
