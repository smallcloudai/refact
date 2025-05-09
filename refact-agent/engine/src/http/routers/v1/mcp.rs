use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::Serialize;
use tokio::sync::RwLock as ARwLock;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::integrations::mcp::session_mcp::SessionMCP;

#[derive(Serialize)]
pub struct McpServerDesc {
    pub name: String,
    pub config_path: String,
    pub num_tools: usize,
    pub num_resources: usize,
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
