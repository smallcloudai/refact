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
    #[serde(rename = "args")]
    pub mcp_args: Vec<String>,
}

// #[derive(PartialEq)]
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

// #[derive(Default)]
pub struct SessionMCP {
    pub debug_name: String,
    pub launched_cfg: SettingsMCP,  // a copy to compare against IntegrationMCP::cfg, to see if anything has changed
    pub mcp_client: Arc<AMutex<mcp_client_rs::client::Client>>,
    pub mcp_tools: Vec<mcp_client_rs::Tool>,
}

impl IntegrationSession for SessionMCP {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn is_expired(&self) -> bool { false }

    fn try_stop(&mut self) -> Box<dyn Future<Output = String> + Send + '_> {
        Box::new(async {
            tracing::info!("MCP STOP {}", self.debug_name);
            let mcp_client_locked = self.mcp_client.lock().await;
            let maybe_err = mcp_client_locked.shutdown().await;
            if let Err(e) = maybe_err {
                tracing::error!("Failed to stop MCP {}:\n{:?}", self.debug_name, e);
                format!("{} failed to stop", self.debug_name)
            } else {
                format!("{} stopped", self.debug_name)
            }
        })
    }
}


#[async_trait]
impl IntegrationTrait for IntegrationMCP {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn integr_settings_apply(
        &mut self,
        gcx: Arc<ARwLock<GlobalContext>>,
        config_path: String,
        value: &serde_json::Value
    ) -> Result<(), String> {
        let session_key = format!("{}", config_path);
        self.gcx_option = Some(Arc::downgrade(&gcx));

        let cfg = match serde_json::from_value::<SettingsMCP>(value.clone()) {
            Ok(x) => x,
            Err(e) => {
                tracing::error!("Failed to apply settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        };

        let mut session_option = gcx.read().await.integration_sessions.get(&session_key).cloned();
        let mut wrong_cfg = false;
        if let Some(ref session) = session_option {
            let mut session_locked = session.lock().await;
            let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
            if session_downcasted.launched_cfg != cfg {
                wrong_cfg = true;
            }
        }
        if session_option.is_none() || wrong_cfg {
            tracing::info!("MCP START SESSION (1) {:?}", session_key);
            let mut client_builder = ClientBuilder::new(cfg.mcp_command.as_str());
            for arg in &cfg.mcp_args {
                client_builder = client_builder.arg(arg);
            }
            // let mut envs = HashMap::new();
            // envs.insert(
            //     "GITHUB_PERSONAL_ACCESS_TOKEN".to_string(),
            //     std::env::var("GITHUB_PERSONAL_ACCESS_TOKEN").unwrap_or_default(),
            // );

            let client = match client_builder.spawn_and_initialize().await {
                Ok(client) => client,
                Err(client_error) => {
                    tracing::error!("Failed to initialize {}: {:?}", session_key, client_error);
                    return Err(client_error.to_string());
                }
            };

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

            tracing::info!("MCP START SESSION (3) {:?}", session_key);
            let tools_result = match client.list_tools().await {
                Ok(result) => result,
                Err(tools_error) => {
                    tracing::error!("Failed to list tools for {}: {:?}", session_key, tools_error);
                    return Err(tools_error.to_string());
                }
            };
            tracing::info!("MCP START SESSION (4) {:?}", session_key);
            let mcp_client = Arc::new(AMutex::new(client));
            session_option = Some(Arc::new(AMutex::new(Box::new(SessionMCP {
                debug_name: session_key.clone(),
                launched_cfg: cfg.clone(),
                mcp_client,
                mcp_tools: tools_result.tools.clone(),
            }))));
            gcx.write().await.integration_sessions.insert(session_key.clone(), session_option.clone().unwrap());
        }

        match serde_json::from_value::<IntegrationCommon>(value.clone()) {
            Ok(x) => self.common = x,
            Err(e) => {
                tracing::error!("Failed to apply common settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        self.config_path = config_path;

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
        let gcx = match self.gcx_option.as_ref() {
            Some(gcx) => match gcx.upgrade() {
                Some(gcx) => gcx,
                None => {
                    tracing::error!("Whoops the system is shutting down!");
                    return vec![];
                }
            },
            None => {
                tracing::error!("MCP is not set up yet");
                return vec![];
            }
        };
        let session_option = gcx.read().await.integration_sessions.get(&session_key).cloned();
        if session_option.is_none() {
            tracing::error!("No session for {:?}, strange (1)", session_key);
            return vec![];
        }

        let mut result: Vec<Box<dyn crate::tools::tools_description::Tool + Send>> = vec![];
        {
            let session = session_option.unwrap();
            let mut session_locked = session.lock().await;
            let session_downcasted: &mut SessionMCP = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
            // no long operations here
            for tool in session_downcasted.mcp_tools.iter() {
                result.push(Box::new(ToolMCP {
                    common: self.common.clone(),
                    config_path: self.config_path.clone(),
                    mcp_client: session_downcasted.mcp_client.clone(),
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
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        // let session_key = format!("{}", self.config_path);
        // let gcx = ccx.lock().await.global_context.clone();
        // let session_option = gcx.read().await.integration_sessions.get(&session_key).cloned();
        // if session_option.is_none() {
        //     tracing::error!("No session for {:?}, strange (2)", session_key);
        //     return Err(format!("No session for {:?}", session_key));
        // }

        let mut json_arguments: serde_json::Value = serde_json::json!({});
        if let serde_json::Value::Object(schema) = &self.mcp_tool.input_schema {
            if let Some(serde_json::Value::Object(properties)) = schema.get("properties") {
                for (name, prop) in properties {
                    if let Some(prop_type) = prop.get("type") {
                        match prop_type.as_str().unwrap_or("") {
                            "string" => {
                                if let Some(arg_value) = args.get(name) {
                                    json_arguments[name] = serde_json::Value::String(arg_value.as_str().unwrap_or("").to_string());
                                }
                            },
                            "integer" => {
                                if let Some(arg_value) = args.get(name) {
                                    json_arguments[name] = serde_json::Value::Number(arg_value.as_i64().unwrap_or(0).into());
                                }
                            },
                            "boolean" => {
                                if let Some(arg_value) = args.get(name) {
                                    json_arguments[name] = serde_json::Value::Bool(arg_value.as_bool().unwrap_or(false));
                                }
                            },
                            _ => {
                                tracing::warn!("Unsupported argument type: {}", prop_type);
                            }
                        }
                    }
                }
            }
            if let Some(serde_json::Value::Array(required)) = schema.get("required") {
                for req in required {
                    if let Some(req_str) = req.as_str() {
                        if !json_arguments.as_object().unwrap().contains_key(req_str) {
                            return Err(format!("Missing required argument: {}", req_str));
                        }
                    }
                }
            }
        }

        tracing::info!("\nCALL tool '{}' with arguments: {:?}\n", self.mcp_tool.name, json_arguments);

        let tool_output = {
            let mcp_client_locked = self.mcp_client.lock().await;
            let result_probably: Result<mcp_client_rs::CallToolResult, mcp_client_rs::Error> = mcp_client_locked.call_tool(self.mcp_tool.name.as_str(), json_arguments).await;

            match result_probably {
                Ok(result) => {
                    tracing::info!("BBBBB result.is_error={:?}", result.is_error);
                    tracing::info!("BBBBB result.content={:?}", result.content);
                    if result.is_error {
                        return Err(format!("Tool execution error: {:?}", result.content));
                    }
                    if let Some(mcp_client_rs::MessageContent::Text { text }) = result.content.get(0) {
                        text.clone()
                    } else {
                        tracing::error!("Unexpected tool output format: {:?}", result.content);
                        return Err("Unexpected tool output format".to_string());
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to call tool: {:?}", e);
                    return Err("Failed to call tool".to_string());
                }
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
            name: self.mcp_tool.name.clone(),
            agentic: true,
            experimental: false,
            description: self.mcp_tool.description.clone(),
            parameters,
            parameters_required,
        }
    }

    fn tool_name(&self) -> String  {
        self.mcp_tool.name.clone()
    }

    fn command_to_match_against_confirm_deny(
        &self,
        _args: &HashMap<String, serde_json::Value>,
    ) -> Result<String, String> {
        Ok(self.mcp_tool.name.clone())
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
  description:
    f_type: string_long
    f_desc: "Description of the MCP (Model Control Protocol) integration"
  parameters:
    f_type: "tool_parameters"
    f_desc: "Parameters that the model should provide when making MCP calls"
  parameters_required:
    f_type: "string_array"
    f_desc: "List of required parameters"
    f_extra: true
description: |
  MCP (Model Control Protocol) integration for JSON-RPC based communication with the model.
  This integration handles initialization, method calls, and notifications according to the protocol.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
confirmation:
  ask_user_default: []
  deny_default: []
"#;
