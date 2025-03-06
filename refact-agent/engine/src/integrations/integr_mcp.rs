use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Weak;
use std::future::Future;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;

use mcp_client_rs::client::ClientBuilder;

use crate::global_context::GlobalContext;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::integrations::integr_abstract::{IntegrationTrait, IntegrationCommon, IntegrationConfirmation};
use crate::integrations::sessions::IntegrationSession;


#[derive(Deserialize, Serialize, Clone, Default, PartialEq)]
pub struct SettingsMCP {
    #[serde(rename = "command")]
    pub mcp_command: String,
    #[serde(default, rename = "env")]
    pub mcp_env: HashMap<String, String>,
}

pub struct ToolMCP {
    pub common: IntegrationCommon,
    pub config_path: String,
    pub mcp_client: Arc<AMutex<mcp_client_rs::client::Client>>,
    pub mcp_tool: mcp_client_rs::Tool,
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
    pub mcp_client: Option<Arc<AMutex<mcp_client_rs::client::Client>>>,
    pub mcp_tools: Vec<mcp_client_rs::Tool>,
    pub launched_coroutines: Vec<tokio::task::JoinHandle<()>>,
    pub logs: Vec<String>,          // Store log messages
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
            _session_wait_coroutines(self_arc.clone()).await;

            let mut session_locked = self_arc.lock().await;
            let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
            _session_kill_process(session_downcasted).await;

            "".to_string()
        })
    }
}

fn _add_log_entry(session: &mut SessionMCP, entry: String) {
    let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
    let log_entry = format!("[{}] {}", timestamp, entry);
    
    session.logs.extend(log_entry.lines().into_iter().map(|s| s.to_string()));

    if session.logs.len() > 100 {
        let excess = session.logs.len() - 100;
        session.logs.drain(0..excess);
    }
}

async fn _session_kill_process(session: &mut SessionMCP) {
    let debug_name = session.debug_name.clone();
    tracing::info!("Stopping MCP Server for {}", debug_name);
    _add_log_entry(session, "Stopping MCP Server".to_string());
    
    if let Some(mcp_client) = &session.mcp_client {
        let client_result = {
            let mut mcp_client_locked = mcp_client.lock().await;
            mcp_client_locked.shutdown().await
        };
        
        if let Err(e) = client_result {
            let error_msg = format!("Failed to stop MCP: {:?}", e);
            tracing::error!("{} for {}", error_msg, debug_name);
            _add_log_entry(session, error_msg);
        } else {
            let success_msg = "MCP server stopped".to_string();
            tracing::info!("{} for {}", success_msg, debug_name);
            _add_log_entry(session, success_msg);
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
                mcp_tools: vec![],
                launched_coroutines: vec![],
                logs: vec![],
            })));
            tracing::info!("MCP START SESSION {:?}", session_key);
            gcx_write.integration_sessions.insert(session_key.clone(), new_session.clone());
            new_session
        } else {
            session.unwrap()
        }
    };

    let session_key_clone = session_key.clone();
    let new_cfg_clone = new_cfg.clone();
    let session_arc_clone = session_arc.clone();

    {
        let mut session_locked = session_arc.lock().await;
        let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();

        let coroutine = tokio::spawn(async move {
            // tracing::info!("MCP START SESSION LOCK {:?}", session_key_clone);
            let mut session_locked = session_arc_clone.lock().await;
            let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
            // tracing::info!("MCP START SESSION /LOCK {:?}", session_key_clone);

            if session_downcasted.mcp_client.is_some() && new_cfg == session_downcasted.launched_cfg {
                // tracing::info!("MCP NO UPDATE NEEDED {:?}", session_key);
                return;
            }
            
            _add_log_entry(session_downcasted, "Applying new settings".to_string());

            _session_kill_process(session_downcasted).await;

            let parsed_args = match shell_words::split(&new_cfg_clone.mcp_command) {
                Ok(args) => {
                    if args.is_empty() {
                        let error_msg = "Empty command".to_string();
                        tracing::info!("{error_msg}");
                        _add_log_entry(session_downcasted, error_msg);
                        return;
                    }
                    args
                }
                Err(e) => {
                    let error_msg = format!("Failed to parse command: {}", e);
                    tracing::info!("{}", error_msg);
                    _add_log_entry(session_downcasted, error_msg);
                    return;
                }
            };

            let mut client_builder = ClientBuilder::new(&parsed_args[0]);
            for arg in parsed_args.iter().skip(1) {
                client_builder = client_builder.arg(arg);
            }
            for (key, value) in &new_cfg_clone.mcp_env {
                client_builder = client_builder.env(key, value);
            }

            let (mut client, imp, caps) = match client_builder.spawn().await {
                Ok(r) => r,
                Err(e) => {
                    let err_msg = format!("Failed to init process: {}", e);
                    tracing::error!("{err_msg} for {session_key_clone}");
                    _add_log_entry(session_downcasted, err_msg);
                    return;
                }
            };
            if let Err(e) = client.initialize(imp, caps).await {
                let err_msg = format!("Failed to init server: {}", e);
                tracing::error!("{err_msg} for {session_key_clone}");
                _add_log_entry(session_downcasted, err_msg);
                if let Ok(error_log) = client.get_stderr(None).await {
                    _add_log_entry(session_downcasted, error_log);
                }
                return;
            }

            // let set_result = client.request(
            //     "logging/setLevel",
            //     Some(serde_json::json!({ "level": "debug" })),
            // ).await;
            // match set_result {
            //     Ok(_) => {
            //         tracing::info!("MCP START SESSION (2) set log level success");
            //     }
            //     Err(e) => {
            //         tracing::info!("MCP START SESSION (2) failed to set log level: {:?}", e);
            //     }
            // }

            tracing::info!("MCP START SESSION (2) {:?}", session_key_clone);
            _add_log_entry(session_downcasted, "Listing tools".to_string());
            
            let tools_result = match client.list_tools().await {
                Ok(result) => {
                    let success_msg = format!("Successfully listed {} tools", result.tools.len());
                    tracing::info!("{} for {}", success_msg, session_key_clone);
                    result
                },
                Err(tools_error) => {
                    let err_msg = format!("Failed to list tools: {:?}", tools_error);
                    tracing::error!("{} for {}", err_msg, session_key_clone);
                    _add_log_entry(session_downcasted, err_msg);
                    if let Ok(error_log) = client.get_stderr(None).await {
                        _add_log_entry(session_downcasted, error_log);
                    }
                    return;
                }
            };

            tracing::info!("MCP START SESSION (3) {:?}", session_key_clone);
            let mcp_client = Arc::new(AMutex::new(client));
            session_downcasted.mcp_client = Some(mcp_client.clone());
            session_downcasted.mcp_tools = tools_result.tools.clone();
            session_downcasted.launched_cfg = new_cfg_clone.clone();
            
            let setup_msg = format!("MCP session setup complete with {} tools", 
                session_downcasted.mcp_tools.len());
            tracing::info!("{} for {}", setup_msg, session_key_clone);
            _add_log_entry(session_downcasted, setup_msg);
        });

        session_downcasted.launched_coroutines.push(coroutine);
    }
}

async fn _session_wait_coroutines(
    session_arc: Arc<AMutex<Box<dyn IntegrationSession>>>,
) {
    loop {
        let handle = {
            let mut session_locked = session_arc.lock().await;
            let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
            if session_downcasted.launched_coroutines.is_empty() {
                return;
            }
            session_downcasted.launched_coroutines.remove(0)
        };
        if let Err(e) = handle.await {
            tracing::error!("Error waiting for coroutine: {:?}", e);
            return;
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

        _session_wait_coroutines(session.clone()).await;

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
        _session_wait_coroutines(session.clone()).await;

        let json_args = serde_json::json!(args);
        tracing::info!("\n\nMCP CALL tool '{}' with arguments: {:?}", self.mcp_tool.name, json_args);
        
        {
            let mut session_locked = session.lock().await;
            let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
            _add_log_entry(session_downcasted, format!("Executing tool '{}' with arguments: {:?}", self.mcp_tool.name, json_args));
        }
        
        let result_probably = {
            let mut mcp_client_locked = self.mcp_client.lock().await;
            mcp_client_locked.call_tool(self.mcp_tool.name.as_str(), json_args).await
        };
        
        let tool_output = match result_probably {
            Ok(result) => {
                if result.is_error {
                    let error_msg = format!("Tool execution error: {:?}", result.content);
                    {
                        let mut session_locked = session.lock().await;
                        let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
                        _add_log_entry(session_downcasted, error_msg.clone());
                    }
                    return Err(error_msg);
                }
                
                if let Some(mcp_client_rs::MessageContent::Text { text }) = result.content.get(0) {
                    let success_msg = format!("Tool '{}' executed successfully", self.mcp_tool.name);
                    {
                        let mut session_locked = session.lock().await;
                        let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
                        _add_log_entry(session_downcasted, success_msg);
                    }
                    text.clone()
                } else {
                    let error_msg = format!("Unexpected tool output format: {:?}", result.content);
                    tracing::error!("{}", error_msg);
                    {
                        let mut session_locked = session.lock().await;
                        let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
                        _add_log_entry(session_downcasted, error_msg.clone());
                    }
                    return Err("Unexpected tool output format".to_string());
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to call tool: {:?}", e);
                tracing::error!("{}", error_msg);
                {
                    let mut session_locked = session.lock().await;
                    let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
                    _add_log_entry(session_downcasted, error_msg.clone());
                    
                    let error_log = self.mcp_client.lock().await.get_stderr(None).await;
                    if let Ok(error_log) = error_log {
                        _add_log_entry(session_downcasted, error_log);
                    }
                }
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

        if let serde_json::Value::Object(schema) = &self.mcp_tool.input_schema {
            if let Some(serde_json::Value::Object(properties)) = schema.get("properties") {
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
            if let Some(serde_json::Value::Array(required)) = schema.get("required") {
                for req in required {
                    if let Some(req_str) = req.as_str() {
                        parameters_required.push(req_str.to_string());
                    }
                }
            }
        }

        ToolDesc {
            name: self.tool_name(),
            agentic: true,
            experimental: false,
            description: self.mcp_tool.description.clone(),
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

    fn command_to_match_against_confirm_deny(
        &self,
        _args: &HashMap<String, serde_json::Value>,
    ) -> Result<String, String> {
        let command = self.mcp_tool.name.clone();
        tracing::info!("MCP command_to_match_against_confirm_deny() returns {:?}", command);
        Ok(command)
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
  command:
    f_type: string
    f_desc: "The MCP command to execute, like `npx -y <some-mcp-server>`, `/my/path/venv/python -m <some-mcp-server>`, or `docker run -i --rm <some-mcp-image>`. On Windows, use `npx.cmd` or `npm.cmd` instead of `npx` or `npm`."
  env:
    f_type: string_to_string_map
description: |
  You can add almost any MCP (Model Context Protocol) server here! This supports local MCP servers,
  with remote servers coming up as the specificion gets updated. You can read more
  here https://www.anthropic.com/news/model-context-protocol
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
