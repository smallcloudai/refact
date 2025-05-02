use std::sync::Arc;
use std::sync::Weak;
use async_trait::async_trait;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use tokio::time::timeout;
use tokio::time::Duration;
use rmcp::{RoleClient, service::RunningService};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::global_context::GlobalContext;
use crate::integrations::integr_abstract::IntegrationCommon;
use crate::integrations::sessions::get_session_hashmap_key;
use crate::integrations::utils::{serialize_num_to_str, deserialize_str_to_num};
use super::session_mcp::{SessionMCP, add_log_entry, cancel_mcp_client};
use super::tool_mcp::ToolMCP;

#[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
pub struct CommonMCPSettings {
    #[serde(default = "default_init_timeout", serialize_with = "serialize_num_to_str", deserialize_with = "deserialize_str_to_num")]
    pub init_timeout: u64,
    #[serde(default = "default_request_timeout", serialize_with = "serialize_num_to_str", deserialize_with = "deserialize_str_to_num")]
    pub request_timeout: u64,
}

pub fn default_init_timeout() -> u64 { 60 }

pub fn default_request_timeout() -> u64 { 30 }

pub trait MCPTransportInitializer: Send + Sync {
    async fn init_mcp_transport(
        &self,
        logs: Arc<AMutex<Vec<String>>>,
        debug_name: String,
        init_timeout: u64,
        request_timeout: u64,
        session: Arc<AMutex<Box<dyn crate::integrations::sessions::IntegrationSession>>>
    ) -> Option<RunningService<RoleClient, ()>>;
}

pub async fn mcp_integr_tools(
    gcx_option: Option<Weak<ARwLock<GlobalContext>>>,
    config_path: &str,
    common: &IntegrationCommon,
    request_timeout: u64
) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
    let session_key = get_session_hashmap_key("mcp", config_path);

    let gcx = match gcx_option {
        Some(gcx_weak) => match gcx_weak.upgrade() {
            Some(gcx) => gcx,
            None => {
                tracing::error!("Error: System is shutting down");
                return vec![];
            }
        },
        None => {
            tracing::error!("Error: MCP integration is not set up yet");
            return vec![];
        }
    };

    let session_maybe = gcx.read().await.integration_sessions.get(&session_key).cloned();
    let session = match session_maybe {
        Some(session) => session,
        None => {
            tracing::error!("No session for {:?}, strange (1)", session_key);
            return vec![];
        }
    };

    let mut result: Vec<Box<dyn crate::tools::tools_description::Tool + Send>> = vec![];
    {
        let mut session_locked = session.lock().await;
        let session_downcasted: &mut SessionMCP = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
        if session_downcasted.mcp_client.is_none() {
            tracing::error!("No mcp_client for {:?}, strange (2)", session_key);
            return vec![];
        }
        for tool in session_downcasted.mcp_tools.iter() {
            result.push(Box::new(ToolMCP {
                common: common.clone(),
                config_path: config_path.to_string(),
                mcp_client: session_downcasted.mcp_client.clone().unwrap(),
                mcp_tool: tool.clone(),
                request_timeout,
            }));
        }
    }

    result
}

pub async fn mcp_session_setup<T: MCPTransportInitializer + 'static>(
    gcx: Arc<ARwLock<GlobalContext>>,
    config_path: String,
    new_cfg_value: Value,
    transport_initializer: T,
    init_timeout: u64,
    request_timeout: u64
) {
    let session_key = get_session_hashmap_key("mcp", &config_path);

    let session_arc = {
        let mut gcx_write = gcx.write().await;
        let session = gcx_write.integration_sessions.get(&session_key).cloned();
        if session.is_none() {
            let new_session: Arc<AMutex<Box<dyn crate::integrations::sessions::IntegrationSession>>> = Arc::new(AMutex::new(Box::new(SessionMCP {
                debug_name: session_key.clone(),
                config_path: config_path.clone(),
                launched_cfg: new_cfg_value.clone(),
                mcp_client: None,
                mcp_tools: Vec::new(),
                startup_task_handles: None,
                logs: Arc::new(AMutex::new(Vec::new())),
                stderr_file_path: None,
                stderr_cursor: Arc::new(AMutex::new(0)),
            })));
            tracing::info!("MCP START SESSION {:?}", session_key);
            gcx_write.integration_sessions.insert(session_key.clone(), new_session.clone());
            new_session
        } else {
            session.unwrap()
        }
    };

    let session_arc_clone = session_arc.clone();

    {
        let mut session_locked = session_arc.lock().await;
        let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();

        // If it's same config, and there is an mcp client, or startup task is running, skip
        if new_cfg_value == session_downcasted.launched_cfg {
            if session_downcasted.mcp_client.is_some() || session_downcasted.startup_task_handles.as_ref().map_or(
                false, |h| !h.1.is_finished()
            ) {
                return;
            }
        }

        let startup_task_join_handle = tokio::spawn(async move {
            let (mcp_client, logs, debug_name, stderr_file) = {
                let mut session_locked = session_arc_clone.lock().await;
                let mcp_session = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
                mcp_session.stderr_cursor = Arc::new(AMutex::new(0));
                mcp_session.launched_cfg = new_cfg_value.clone();
                (
                    std::mem::take(&mut mcp_session.mcp_client),
                    mcp_session.logs.clone(),
                    mcp_session.debug_name.clone(),
                    std::mem::take(&mut mcp_session.stderr_file_path),
                )
            };

            let log = async |level: tracing::Level, msg: String| {
                match level {
                    tracing::Level::ERROR => tracing::error!("{msg} for {debug_name}"),
                    tracing::Level::WARN => tracing::warn!("{msg} for {debug_name}"),
                    _ => tracing::info!("{msg} for {debug_name}"),
                }
                add_log_entry(logs.clone(), msg).await;
            };

            log(tracing::Level::INFO, "Applying new settings".to_string()).await;

            if let Some(mcp_client) = mcp_client {
                cancel_mcp_client(&debug_name, mcp_client, logs.clone()).await;
            }
            if let Some(stderr_file) = &stderr_file {
                if let Err(e) = tokio::fs::remove_file(stderr_file).await {
                    log(tracing::Level::ERROR, format!("Failed to remove {}: {}", stderr_file.to_string_lossy(), e)).await;
                }
            }

            let client = match transport_initializer.init_mcp_transport(
                logs.clone(),
                debug_name.clone(),
                init_timeout,
                request_timeout,
                session_arc_clone.clone()
            ).await {
                Some(client) => client,
                None => return,
            };

            log(tracing::Level::INFO, "Listing tools".to_string()).await;

            let tools = match timeout(Duration::from_secs(request_timeout), client.list_all_tools()).await {
                Ok(Ok(result)) => result,
                Ok(Err(tools_error)) => {
                    log(tracing::Level::ERROR, format!("Failed to list tools: {:?}", tools_error)).await;
                    Vec::new()
                },
                Err(_) => {
                    log(tracing::Level::ERROR, format!("Request timed out after {} seconds", request_timeout)).await;
                    Vec::new()
                }
            };
            let tools_len = tools.len();

            let resources = match timeout(Duration::from_secs(request_timeout), client.list_all_resources()).await {
                Ok(Ok(r)) => Some(r),
                Ok(Err(e)) => {
                    log(tracing::Level::ERROR, format!("Failed to list resources: {:?}", e)).await;
                    None
                },
                Err(_) => {
                    log(tracing::Level::ERROR, format!("Listing resources timed out after {request_timeout} seconds")).await;
                    None
                }
            };
            let resources_len = resources.as_ref().map(|r| r.len());

            if tools.is_empty() && resources.is_none() { return; }

            {
                let mut session_locked = session_arc_clone.lock().await;
                let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();

                session_downcasted.mcp_client = Some(Arc::new(AMutex::new(Some(client))));
                session_downcasted.mcp_tools = tools;
                session_downcasted.mcp_resources = resources;

                session_downcasted.mcp_tools.len()
            };

            let resources_len_str = resources_len.map_or("no".to_string(), |len| len.to_string());
            log(tracing::Level::INFO, format!("MCP session setup complete with {tools_len} tools and {resources_len_str} resources")).await;
        });

        let startup_task_abort_handle = startup_task_join_handle.abort_handle();
        session_downcasted.startup_task_handles = Some(
            (Arc::new(AMutex::new(Some(startup_task_join_handle))), startup_task_abort_handle)
        );
    }
}
