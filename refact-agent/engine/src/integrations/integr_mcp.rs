use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Weak;
use std::future::Future;
use std::process::Stdio;
use async_trait::async_trait;
use rmcp::model::PaginatedRequestParamInner;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tokio::time::timeout;
use tokio::time::Duration;
use tokio::task::{AbortHandle, JoinHandle};
use rmcp::{RoleClient, ServiceExt, service::RunningService};
use rmcp::model::{CallToolRequestParam, Tool as McpTool};
use tempfile::NamedTempFile;
use tracing::Level;

use crate::custom_error::MapErrToString;
use crate::global_context::GlobalContext;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::integrations::integr_abstract::{IntegrationTrait, IntegrationCommon, IntegrationConfirmation};
use crate::integrations::sessions::IntegrationSession;
use crate::integrations::process_io_utils::read_file_with_cursor;

const MCP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MCP_SERVER_INIT_TIMEOUT: Duration = Duration::from_secs(60);
const MCP_SERVER_STOP_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
pub struct SettingsMCP {
    #[serde(default = "default_server_transport")]
    pub server_transport: String,
    #[serde(rename = "command", default)]
    pub mcp_command: String,
    #[serde(default, rename = "env")]
    pub mcp_env: HashMap<String, String>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

fn default_server_transport() -> String {
    "stdio".to_string()
}

pub struct ToolMCP {
    pub common: IntegrationCommon,
    pub config_path: String,
    pub mcp_client: Arc<AMutex<Option<RunningService<RoleClient, ()>>>>,
    pub mcp_tool: McpTool,
}

#[derive(Default)]
pub struct IntegrationMCP {
    pub gcx_option: Option<Weak<ARwLock<GlobalContext>>>,  // need default to zero, to have access to all the virtual functions and then set it up
    pub cfg: SettingsMCP,
    pub common: IntegrationCommon,
    pub config_path: String,
}

pub struct SessionMCP {
    pub debug_name: String,
    pub config_path: String,        // to check if expired or not
    pub launched_cfg: SettingsMCP,  // a copy to compare against IntegrationMCP::cfg, to see if anything has changed
    pub mcp_client: Option<Arc<AMutex<Option<RunningService<RoleClient, ()>>>>>,
    pub mcp_tools: Vec<McpTool>,
    pub startup_task_handles: Option<(Arc<AMutex<Option<JoinHandle<()>>>>, AbortHandle)>,
    pub logs: Arc<AMutex<Vec<String>>>,          // Store log messages
    pub stderr_file_path: Option<PathBuf>,       // Path to the temporary file for stderr
    pub stderr_cursor: Arc<AMutex<u64>>,         // Position in the file where we last read from
}

impl IntegrationSession for SessionMCP {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn is_expired(&self) -> bool {
        !std::path::Path::new(&self.config_path).exists()
    }

    fn try_stop(&mut self, self_arc: Arc<AMutex<Box<dyn IntegrationSession>>>) -> Box<dyn Future<Output = String> + Send> {
        Box::new(async move {
            let (debug_name, client, logs, startup_task_handles, stderr_file) = {
                let mut session_locked = self_arc.lock().await;
                let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
                (
                    session_downcasted.debug_name.clone(),
                    session_downcasted.mcp_client.clone(),
                    session_downcasted.logs.clone(),
                    session_downcasted.startup_task_handles.clone(),
                    session_downcasted.stderr_file_path.clone(),
                )
            };

            if let Some((_, abort_handle)) = startup_task_handles {
                _add_log_entry(logs.clone(), "Aborted startup task".to_string()).await;
                abort_handle.abort();
            }

            if let Some(client) = client {
                _session_kill_process(&debug_name, client, logs).await;
            }
            if let Some(stderr_file) = &stderr_file {
                if let Err(e) = tokio::fs::remove_file(stderr_file).await {
                    tracing::error!("Failed to remove {}: {}", stderr_file.to_string_lossy(), e);
                }
            }

            "".to_string()
        })
    }
}

async fn _add_log_entry(session_logs: Arc<AMutex<Vec<String>>>, entry: String) {
    let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
    let log_entry = format!("[{}] {}", timestamp, entry);

    let mut session_logs_locked = session_logs.lock().await;
    session_logs_locked.extend(log_entry.lines().into_iter().map(|s| s.to_string()));

    if session_logs_locked.len() > 100 {
        let excess = session_logs_locked.len() - 100;
        session_logs_locked.drain(0..excess);
    }
}

pub async fn update_logs_from_stderr(
    stderr_file_path: &PathBuf,
    stderr_cursor: Arc<AMutex<u64>>,
    session_logs: Arc<AMutex<Vec<String>>>
) -> Result<(), String> {
    let (buffer, bytes_read) = read_file_with_cursor(stderr_file_path, stderr_cursor.clone()).await
        .map_err_with_prefix("Failed to read file:")?;
    if bytes_read > 0 && !buffer.trim().is_empty() {
        _add_log_entry(session_logs, buffer.trim().to_string()).await;
    }
    Ok(())
}

async fn _session_kill_process(
    debug_name: &str,
    mcp_client: Arc<AMutex<Option<RunningService<RoleClient, ()>>>>,
    session_logs: Arc<AMutex<Vec<String>>>,
) {
    tracing::info!("Stopping MCP Server for {}", debug_name);
    _add_log_entry(session_logs.clone(), "Stopping MCP Server".to_string()).await;

    let client_to_cancel = {
        let mut mcp_client_locked = mcp_client.lock().await;
        mcp_client_locked.take()
    };

    if let Some(client) = client_to_cancel {
        match timeout(MCP_SERVER_STOP_TIMEOUT, client.cancel()).await {
            Ok(Ok(reason)) => {
                let success_msg = format!("MCP server stopped: {:?}", reason);
                tracing::info!("{} for {}", success_msg, debug_name);
                _add_log_entry(session_logs, success_msg).await;
            },
            Ok(Err(e)) => {
                let error_msg = format!("Failed to stop MCP: {:?}", e);
                tracing::error!("{} for {}", error_msg, debug_name);
                _add_log_entry(session_logs, error_msg).await;
            },
            Err(_) => {
                let error_msg = format!("MCP server stop operation timed out after {} seconds", MCP_SERVER_STOP_TIMEOUT.as_secs());
                tracing::error!("{} for {}", error_msg, debug_name);
                _add_log_entry(session_logs, error_msg).await;
            }
        }
    }
}

async fn _session_apply_settings(
    gcx: Arc<ARwLock<GlobalContext>>,
    config_path: String,
    new_cfg: SettingsMCP,
) {
    let session_key = format!("{}", config_path);

    let session_arc = {
        let mut gcx_write = gcx.write().await;
        let session = gcx_write.integration_sessions.get(&session_key).cloned();
        if session.is_none() {
            let new_session: Arc<AMutex<Box<dyn IntegrationSession>>> = Arc::new(AMutex::new(Box::new(SessionMCP {
                debug_name: session_key.clone(),
                config_path: config_path.clone(),
                launched_cfg: new_cfg.clone(),
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

            let client = match new_cfg_clone.server_transport.to_lowercase().trim() {
                "stdio" => {
                    let parsed_args = match shell_words::split(&new_cfg_clone.mcp_command) {
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

                    let transport = match rmcp::transport::TokioChildProcess::new(&mut command) {
                        Ok(t) => t,
                        Err(e) => {
                            log(Level::ERROR, format!("Failed to init Tokio child process: {}", e)).await;
                            return;
                        }
                    };
                    match timeout(MCP_SERVER_INIT_TIMEOUT, ().serve(transport)).await {
                        Ok(Ok(client)) => client,
                        Ok(Err(e)) => {
                            log(Level::ERROR, format!("Failed to init stdio server: {}", e)).await;
                            return;
                        },
                        Err(_) => {
                            log(Level::ERROR, format!("Request timed out after {} seconds", MCP_SERVER_INIT_TIMEOUT.as_secs())).await;
                            return;
                        }
                    }
                },
                "sse" => {
                    if new_cfg_clone.url.is_empty() {
                        log(Level::ERROR, "URL is required for MCP with SSE transport".to_string()).await;
                        return;
                    }

                    let mut header_map = reqwest::header::HeaderMap::new();
                    for (k, v) in &new_cfg_clone.headers {
                        match (reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                            reqwest::header::HeaderValue::from_str(v),
                        ) {
                            (Ok(name), Ok(value)) => {
                                header_map.insert(name, value);
                            }
                            _ => log(Level::WARN, format!("Invalid header: {}: {}", k, v)).await,
                        }
                    }
                    let client = match reqwest::Client::builder().default_headers(header_map).build() {
                        Ok(c) => c,
                        Err(e) => {
                            log(Level::ERROR, format!("Failed to build reqwest client: {}", e)).await;
                            return;
                        }
                    };
                    let transport = match rmcp::transport::SseTransport::start_with_client(&new_cfg_clone.url, client).await {
                        Ok(t) => t,
                        Err(e) => {
                            log(Level::ERROR, format!("Failed to init SSE transport: {}", e)).await;
                            return;
                        }
                    };
                    match timeout(MCP_SERVER_INIT_TIMEOUT, ().serve(transport)).await {
                        Ok(Ok(client)) => client,
                        Ok(Err(e)) => {
                            log(Level::ERROR, format!("Failed to init SSE server: {}", e)).await;
                            return;
                        },
                        Err(_) => {
                            log(Level::ERROR, format!("Request timed out after {} seconds", MCP_SERVER_INIT_TIMEOUT.as_secs())).await;
                            return;
                        }
                    }
                }
                _ => {
                    log(Level::ERROR, format!("Unsupported server transport: {}", new_cfg_clone.server_transport)).await;
                    return;
                }
            };

            log(Level::INFO, "Listing tools".to_string()).await;

            let tools = match timeout(MCP_REQUEST_TIMEOUT, client.list_all_tools()).await {
                Ok(Ok(result)) => result,
                Ok(Err(tools_error)) => {
                    log(Level::ERROR, format!("Failed to list tools: {:?}", tools_error)).await;
                    return;
                },
                Err(_) => {
                    log(Level::ERROR, format!("Request timed out after {} seconds", MCP_REQUEST_TIMEOUT.as_secs())).await;
                    return;
                }
            };
            let tools_len = tools.len();

            {
                let mut session_locked = session_arc_clone.lock().await;
                let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();

                session_downcasted.mcp_client = Some(Arc::new(AMutex::new(Some(client))));
                session_downcasted.mcp_tools = tools;

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

async fn _session_wait_startup_task(
    session_arc: Arc<AMutex<Box<dyn IntegrationSession>>>,
) {
    let startup_task_handles = {
        let mut session_locked = session_arc.lock().await;
        let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
        session_downcasted.startup_task_handles.clone()
    };

    if let Some((join_handler_arc, _)) = startup_task_handles {
        let mut join_handler_locked = join_handler_arc.lock().await;
        if let Some(join_handler) = join_handler_locked.take() {
            let _ = join_handler.await;
        }
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
                }));
            }
        }

        result
    }

    fn integr_schema(&self) -> &str {
        MCP_INTEGRATION_SCHEMA
    }
}

#[async_trait]
impl Tool for ToolMCP {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let session_key = format!("{}", self.config_path);
        let gcx = ccx.lock().await.global_context.clone();
        let session_option = gcx.read().await.integration_sessions.get(&session_key).cloned();
        if session_option.is_none() {
            tracing::error!("No session for {:?}, strange (2)", session_key);
            return Err(format!("No session for {:?}", session_key));
        }
        let session = session_option.unwrap();
        _session_wait_startup_task(session.clone()).await;

        let json_args = serde_json::json!(args);
        tracing::info!("\n\nMCP CALL tool '{}' with arguments: {:?}", self.mcp_tool.name, json_args);

        let session_logs = {
            let mut session_locked = session.lock().await;
            let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
            session_downcasted.logs.clone()
        };

        _add_log_entry(session_logs.clone(), format!("Executing tool '{}' with arguments: {:?}", self.mcp_tool.name, json_args)).await;

        let result_probably = {
            let mcp_client_locked = self.mcp_client.lock().await;
            if let Some(client) = &*mcp_client_locked {
                match timeout(MCP_REQUEST_TIMEOUT,
                    client.call_tool(CallToolRequestParam {
                        name: self.mcp_tool.name.clone(),
                        arguments: match json_args {
                            serde_json::Value::Object(map) => Some(map),
                            _ => None,
                        },
                    })
                ).await {
                    Ok(result) => result,
                    Err(_) => Err(rmcp::service::ServiceError::Timeout {
                        timeout: MCP_REQUEST_TIMEOUT
                    }),
                }
            } else {
                return Err("MCP client is not available".to_string());
            }
        };

        let tool_output = match result_probably {
            Ok(result) => {
                if result.is_error.unwrap_or(false) {
                    let error_msg = format!("Tool execution error: {:?}", result.content);
                    _add_log_entry(session_logs.clone(), error_msg.clone()).await;
                    return Err(error_msg);
                }

                if let Some(content) = result.content.get(0) {
                    if let rmcp::model::RawContent::Text(text_content) = &content.raw {
                        let text = text_content.text.clone();
                        let success_msg = format!("Tool '{}' executed successfully", self.mcp_tool.name);
                        _add_log_entry(session_logs.clone(), success_msg).await;
                        text
                    } else {
                        let error_msg = format!("Unexpected tool output format: {:?}", result.content);
                        tracing::error!("{}", error_msg);
                        _add_log_entry(session_logs.clone(), error_msg.clone()).await;
                        return Err("Unexpected tool output format".to_string());
                    }
                } else {
                    String::new()
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to call tool: {:?}", e);
                tracing::error!("{}", error_msg);
                _add_log_entry(session_logs.clone(), error_msg).await;
                return Err(e.to_string());
            }
        };

        let result = vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(tool_output),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })];

        Ok((false, result))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }

    fn tool_description(&self) -> ToolDesc {
        // self.mcp_tool.input_schema = Object {
        //     "properties": Object {
        //         "a": Object {
        //             "title": String("A"),
        //             "type": String("integer")
        //         },
        //         "b": Object {
        //             "title": String("B"),
        //             "type": String("integer")
        //         }
        //     },
        //     "required": Array [
        //         String("a"),
        //         String("b")
        //     ],
        //     "title": String("addArguments"),
        //     "type": String("object")
        // }
        let mut parameters = vec![];
        let mut parameters_required = vec![];

        if let Some(serde_json::Value::Object(properties)) = self.mcp_tool.input_schema.get("properties") {
            for (name, prop) in properties {
                if let serde_json::Value::Object(prop_obj) = prop {
                    let param_type = prop_obj.get("type").and_then(|v| v.as_str()).unwrap_or("string").to_string();
                    let description = prop_obj.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    parameters.push(ToolParam {
                        name: name.clone(),
                        param_type,
                        description,
                    });
                }
            }
        }
        if let Some(serde_json::Value::Array(required)) = self.mcp_tool.input_schema.get("required") {
            for req in required {
                if let Some(req_str) = req.as_str() {
                    parameters_required.push(req_str.to_string());
                }
            }
        }

        ToolDesc {
            name: self.tool_name(),
            agentic: true,
            experimental: false,
            description: self.mcp_tool.description.to_string(),
            parameters,
            parameters_required,
        }
    }

    fn tool_name(&self) -> String  {
        let yaml_name = std::path::Path::new(&self.config_path)
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown");
        let sanitized_yaml_name = format!("{}_{}", yaml_name, self.mcp_tool.name)
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>();
        sanitized_yaml_name
    }

    async fn command_to_match_against_confirm_deny(
        &self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        _args: &HashMap<String, serde_json::Value>,
    ) -> Result<String, String> {
        let command = self.mcp_tool.name.clone();
        tracing::info!("MCP command_to_match_against_confirm_deny() returns {:?}", command);
        Ok(command.to_string())
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(self.common.confirmation.clone())
    }

    fn has_config_path(&self) -> Option<String> {
        Some(self.config_path.clone())
    }
}

pub const MCP_INTEGRATION_SCHEMA: &str = r#"
fields:
  server_transport:
    f_type: enum
    f_enum_values: ["stdio", "sse"]
    f_default: "stdio"
    f_desc: "The transport protocol to use. 'stdio' for local processes, 'sse' for remote servers using Server-Sent Events."
  command:
    f_type: string
    f_desc: "The MCP command to execute (for stdio transport), like `npx -y <some-mcp-server>`, `/my/path/venv/python -m <some-mcp-server>`, or `docker run -i --rm <some-mcp-image>`. On Windows, use `npx.cmd` or `npm.cmd` instead of `npx` or `npm`."
  env:
    f_type: string_to_string_map
    f_desc: "Environment variables to pass to the MCP command (for stdio transport)."
  url:
    f_type: string
    f_desc: "The URL of the MCP server (for sse transport), e.g., 'https://api.example.com/mcp/sse'."
  headers:
    f_type: string_to_string_map
    f_desc: "HTTP headers to include in requests to the MCP server (for sse transport)."
    f_default:
      User-Agent: "Refact.ai (+https://github.com/smallcloudai/refact)"
      Accept: text/event-stream
      Content-Type: application/json
description: |
  You can add almost any MCP (Model Context Protocol) server here! This supports both local MCP servers (stdio)
  and remote MCP servers (sse). You can read more about MCP here: https://www.anthropic.com/news/model-context-protocol

  For local servers, use server_transport="stdio" and provide the command to execute.
  For remote servers, use server_transport="sse" and provide the URL of the server.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
confirmation:
  ask_user_default: ["*"]
  deny_default: []
smartlinks:
  - sl_label: "Test"
    sl_chat:
      - role: "user"
        content: >
          🔧 Your job is to test %CURRENT_CONFIG%. Tools that this MCP server has created should be visible to you. Don't search anything, it should be visible as
          a tools already. Run one and express happiness. If something does wrong, or you don't see the tools, ask user if they want to fix it by rewriting the config.
    sl_enable_only_with_tool: true
"#;
