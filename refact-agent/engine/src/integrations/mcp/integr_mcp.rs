use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Weak;
use std::process::Stdio;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tokio::time::timeout;
use tokio::time::Duration;
use rmcp::transport::sse::ReqwestSseClient;
use rmcp::transport::SseTransport;
use rmcp::serve_client;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tracing::Level;

use crate::global_context::GlobalContext;
use crate::integrations::integr_abstract::{IntegrationTrait, IntegrationCommon};
use crate::integrations::utils::{serialize_num_to_str, deserialize_str_to_num};
use super::session_mcp::{SessionMCP, _add_log_entry, _session_kill_process};
use super::tool_mcp::ToolMCP;
use super::MCP_INTEGRATION_SCHEMA;

#[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
pub struct SettingsMCP {
    #[serde(rename = "command", default)]
    pub mcp_command: String,
    #[serde(default, rename = "env")]
    pub mcp_env: HashMap<String, String>,
    #[serde(default, rename = "url")]
    pub mcp_url: String,
    #[serde(default = "default_headers", rename = "headers")]
    pub mcp_headers: HashMap<String, String>,
    #[serde(default = "default_init_timeout", serialize_with = "serialize_num_to_str", deserialize_with = "deserialize_str_to_num")]
    pub init_timeout: u64,
    #[serde(default = "default_request_timeout", serialize_with = "serialize_num_to_str", deserialize_with = "deserialize_str_to_num")]
    pub request_timeout: u64,
}

fn default_init_timeout() -> u64 { 60 }
fn default_request_timeout() -> u64 { 30 }
fn default_headers() -> HashMap<String, String> {
    HashMap::from([
        ("User-Agent".to_string(), "Refact.ai (+https://github.com/smallcloudai/refact)".to_string()),
        ("Accept".to_string(), "text/event-stream".to_string()),
        ("Content-Type".to_string(), "application/json".to_string()),
    ])
}

#[derive(Default)]
pub struct IntegrationMCP {
    pub gcx_option: Option<Weak<ARwLock<GlobalContext>>>,  // need default to zero, to have access to all the virtual functions and then set it up
    pub cfg: SettingsMCP,
    pub common: IntegrationCommon,
    pub config_path: String,
}

pub async fn _session_apply_settings(
    gcx: Arc<ARwLock<GlobalContext>>,
    config_path: String,
    new_cfg: SettingsMCP,
) {
    let session_key = format!("{}", config_path);

    let session_arc = {
        let mut gcx_write = gcx.write().await;
        let session = gcx_write.integration_sessions.get(&session_key).cloned();
        if session.is_none() {
            let new_session: Arc<AMutex<Box<dyn crate::integrations::sessions::IntegrationSession>>> = Arc::new(AMutex::new(Box::new(SessionMCP {
                debug_name: session_key.clone(),
                config_path: config_path.clone(),
                launched_cfg: new_cfg.clone(),
                mcp_client: None,
                mcp_tools: Vec::new(),
                mcp_resources: None,
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

    let new_cfg_clone = new_cfg.clone();
    let session_arc_clone = session_arc.clone();

    {
        let mut session_locked = session_arc.lock().await;
        let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();

        // If it's same config, and there is an mcp client, or startup task is running, skip
        if new_cfg == session_downcasted.launched_cfg {
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
                mcp_session.launched_cfg = new_cfg_clone.clone();
                (
                    std::mem::take(&mut mcp_session.mcp_client),
                    mcp_session.logs.clone(),
                    mcp_session.debug_name.clone(),
                    std::mem::take(&mut mcp_session.stderr_file_path),
                )
            };

            let log = async |level: Level, msg: String| {
                match level {
                    Level::ERROR => tracing::error!("{msg} for {debug_name}"),
                    Level::WARN => tracing::warn!("{msg} for {debug_name}"),
                    _ => tracing::info!("{msg} for {debug_name}"),
                }
                _add_log_entry(logs.clone(), msg).await;
            };

            log(Level::INFO, "Applying new settings".to_string()).await;

            if let Some(mcp_client) = mcp_client {
                _session_kill_process(&debug_name, mcp_client, logs.clone()).await;
            }
            if let Some(stderr_file) = &stderr_file {
                if let Err(e) = tokio::fs::remove_file(stderr_file).await {
                    log(Level::ERROR, format!("Failed to remove {}: {}", stderr_file.to_string_lossy(), e)).await;
                }
            }

            let client = match (new_cfg_clone.mcp_url.trim(), new_cfg_clone.mcp_command.trim()) {
                ("", "") => {
                    log(Level::ERROR, "Url and command are both empty, set up either url for sse protocol, or command for stdio protocol".to_string()).await;
                    return;
                },
                (url, "") => {
                    let mut header_map = reqwest::header::HeaderMap::new();
                    for (k, v) in &new_cfg_clone.mcp_headers {
                        match (reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                            reqwest::header::HeaderValue::from_str(v),
                        ) {
                            (Ok(name), Ok(value)) => {
                                header_map.insert(name, value);
                            }
                            _ => log(Level::WARN, format!("Invalid header: {}: {}", k, v)).await,
                        }
                    }
                    let reqwest_client = match reqwest::Client::builder().default_headers(header_map).build() {
                        Ok(reqwest_client) => reqwest_client,
                        Err(e) => {
                            log(Level::ERROR, format!("Failed to build reqwest client: {}", e)).await;
                            return;
                        }
                    };
                    let sse_client = match ReqwestSseClient::new_with_client(url, reqwest_client).await {
                        Ok(sse_client) => sse_client,
                        Err(e) => {
                            log(Level::ERROR, format!("Failed to init SSE client: {}", e)).await;
                            return;
                        },
                    };
                    let transport = match SseTransport::start_with_client(sse_client).await {
                        Ok(t) => t,
                        Err(e) => {
                            log(Level::ERROR, format!("Failed to init SSE transport: {}", e)).await;
                            return;
                        }
                    };
                    match timeout(Duration::from_secs(new_cfg_clone.init_timeout), serve_client((), transport)).await {
                        Ok(Ok(client)) => client,
                        Ok(Err(e)) => {
                            log(Level::ERROR, format!("Failed to init SSE server: {}", e)).await;
                            return;
                        },
                        Err(_) => {
                            log(Level::ERROR, format!("Request timed out after {} seconds", new_cfg_clone.init_timeout)).await;
                            return;
                        }
                    }
                },
                ("", command) => {
                    let parsed_args = match shell_words::split(&command) {
                        Ok(args) => {
                            if args.is_empty() {
                                log(Level::ERROR, "Empty command".to_string()).await;
                                return;
                            }
                            args
                        }
                        Err(e) => {
                            log(Level::ERROR, format!("Failed to parse command: {}", e)).await;
                            return;
                        }
                    };

                    let mut command = tokio::process::Command::new(&parsed_args[0]);
                    command.args(&parsed_args[1..]);
                    for (key, value) in &new_cfg_clone.mcp_env {
                        command.env(key, value);
                    }

                    match NamedTempFile::new().map(|f| f.keep()) {
                        Ok(Ok((file, path))) => {
                            {
                                let mut session_locked = session_arc_clone.lock().await;
                                let mcp_session = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();

                                mcp_session.stderr_file_path = Some(path.clone());
                                mcp_session.stderr_cursor = Arc::new(AMutex::new(0));
                            }
                            command.stderr(Stdio::from(file));
                        },
                        Ok(Err(e)) => tracing::error!("Failed to persist stderr file for {debug_name}: {e}"),
                        Err(e)  => tracing::error!("Failed to create stderr file for {debug_name}: {e}"),
                    }

                    let transport = match rmcp::transport::TokioChildProcess::new(command) {
                        Ok(t) => t,
                        Err(e) => {
                            log(Level::ERROR, format!("Failed to init Tokio child process: {}", e)).await;
                            return;
                        }
                    };
                    match timeout(Duration::from_secs(new_cfg_clone.init_timeout), serve_client((), transport)).await {
                        Ok(Ok(client)) => client,
                        Ok(Err(e)) => {
                            log(Level::ERROR, format!("Failed to init stdio server: {}", e)).await;
                            return;
                        },
                        Err(_) => {
                            log(Level::ERROR, format!("Request timed out after {} seconds", new_cfg_clone.init_timeout)).await;
                            return;
                        }
                    }
                },
                (_url, _command) => {
                    log(Level::ERROR, "Url and command cannot be specified at the same time, set up either url for sse protocol, or command for stdio protocol".to_string()).await;
                    return;
                },
            };

            log(Level::INFO, "Listing tools".to_string()).await;

            let tools = match timeout(Duration::from_secs(new_cfg_clone.request_timeout), client.list_all_tools()).await {
                Ok(Ok(result)) => result,
                Ok(Err(tools_error)) => {
                    log(Level::ERROR, format!("Failed to list tools: {:?}", tools_error)).await;
                    Vec::new()
                },
                Err(_) => {
                    log(Level::ERROR, format!("Request timed out after {} seconds", new_cfg_clone.request_timeout)).await;
                    Vec::new()
                }
            };
            let tools_len = tools.len();

            let resources = match timeout(Duration::from_secs(new_cfg_clone.request_timeout), client.list_all_resources()).await {
                Ok(Ok(r)) => Some(r),
                Ok(Err(e)) => {
                    log(Level::ERROR, format!("Failed to list resources: {:?}", e)).await;
                    None
                },
                Err(_) => {
                    log(Level::ERROR, format!("Listing resources timed out after {} seconds", new_cfg_clone.request_timeout)).await;
                    None
                }
            };

            if tools.is_empty() && resources.is_none() { return; }

            {
                let mut session_locked = session_arc_clone.lock().await;
                let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();

                session_downcasted.mcp_client = Some(Arc::new(AMutex::new(Some(client))));
                session_downcasted.mcp_tools = tools;
                session_downcasted.mcp_resources = resources;

                session_downcasted.mcp_tools.len()
            };

            log(Level::INFO, format!("MCP session setup complete with {tools_len} tools")).await;
        });

        let startup_task_abort_handle = startup_task_join_handle.abort_handle();
        session_downcasted.startup_task_handles = Some(
            (Arc::new(AMutex::new(Some(startup_task_join_handle))), startup_task_abort_handle)
        );
    }
}

#[async_trait]
impl IntegrationTrait for IntegrationMCP {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn integr_settings_apply(&mut self, gcx: Arc<ARwLock<GlobalContext>>, config_path: String, value: &serde_json::Value) -> Result<(), serde_json::Error> {
        self.gcx_option = Some(Arc::downgrade(&gcx));
        self.cfg = serde_json::from_value(value.clone())?;
        self.common = serde_json::from_value(value.clone())?;
        self.config_path = config_path;
        _session_apply_settings(gcx.clone(), self.config_path.clone(), self.cfg.clone()).await;  // possibly saves coroutine in session
        Ok(())
    }

    fn integr_settings_as_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    async fn integr_tools(&self, _integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        let session_key = format!("{}", self.config_path);

        let gcx = match self.gcx_option.clone() {
            Some(gcx_weak) => match gcx_weak.upgrade() {
                Some(gcx) => gcx,
                None => {
                    tracing::error!("Error: System is shutting down");
                    return vec![];
                }
            },
            None => {
                tracing::error!("Error: MCP is not set up yet");
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
                    common: self.common.clone(),
                    config_path: self.config_path.clone(),
                    mcp_client: session_downcasted.mcp_client.clone().unwrap(),
                    mcp_tool: tool.clone(),
                    request_timeout: self.cfg.request_timeout,
                }));
            }
        }

        result
    }

    fn integr_schema(&self) -> &str {
        MCP_INTEGRATION_SCHEMA
    }
}
