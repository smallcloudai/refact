use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
// use jsonrpc_client::{
//     Request,
//     Response,
//     SendRequest,
//     Error,
//     JsonRpcError,
// };

// use mcp_client_rs::dotenv::dotenv;
// use mcp_client_rs::{Protocol, ClientError};
// use mcp_rust_sdk::client::Client;
// use mcp_rust_sdk::transport::websocket::WebSocketTransport;
// use mcp_rust_sdk::transport::stdio::StdioTransport;
// use mcp_rust_sdk::client::ClientBuilder;
use mcp_client_rs::client::ClientBuilder;


use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::integrations::integr_abstract::{IntegrationTrait, IntegrationCommon, IntegrationConfirmation};


#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ConfigMCP {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

pub struct ToolMCP {
    pub common: IntegrationCommon,
    pub config_path: String,
    pub mcp_client: Arc<AMutex<mcp_client_rs::client::Client>>,
    pub mcp_tool: mcp_client_rs::Tool,
}

#[derive(Default)]
pub struct IntegrationMCP {
    pub common: IntegrationCommon,
    pub cfg: ConfigMCP,
    pub config_path: String,
    pub mcp_client: Option<Arc<AMutex<mcp_client_rs::client::Client>>>,
    pub mcp_tools: Vec<mcp_client_rs::Tool>,
}

#[async_trait]
impl IntegrationTrait for IntegrationMCP {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn integr_settings_apply(&mut self, value: &serde_json::Value, config_path: String) -> Result<(), String> {
        match serde_json::from_value::<ConfigMCP>(value.clone()) {
            Ok(x) => self.cfg = x,
            Err(e) => {
                tracing::error!("Failed to apply settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        match serde_json::from_value::<IntegrationCommon>(value.clone()) {
            Ok(x) => self.common = x,
            Err(e) => {
                tracing::error!("Failed to apply common settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        self.config_path = config_path;

        // let mut envs = HashMap::new();
        // envs.insert(
        //     "GITHUB_PERSONAL_ACCESS_TOKEN".to_string(),
        //     std::env::var("GITHUB_PERSONAL_ACCESS_TOKEN").unwrap_or_default(),
        // );

        tracing::info!("AAA GEEEEE {:?}", self.config_path);
        let mut client_builder = ClientBuilder::new(self.cfg.command.as_str());
        for arg in &self.cfg.args {
            client_builder = client_builder.arg(arg);
        }
        let client_maybe = client_builder.spawn_and_initialize().await;

        if let Err(client_error) = client_maybe {
            tracing::error!("Failed to initialize protocol: {:?}", client_error);
            return Err(client_error.to_string());
        }

        let client = client_maybe.unwrap();
        let tool_result_maybe: Result<mcp_client_rs::ListToolsResult, mcp_client_rs::Error> = client.list_tools().await;
        if let Ok(tools_result) = tool_result_maybe {
            tracing::info!("AAA TOOLS {:?}", tools_result);
            self.mcp_tools = tools_result.tools;
        }
        self.mcp_client = Some(Arc::new(AMutex::new(client)));

        Ok(())
    }

    fn integr_settings_as_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    fn integr_tools(&self, integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        if self.mcp_client.is_none() {
            return vec![];
        }
        // self.mcp_tools is ListToolsResult { tools: [Tool { name: "add", description: "Add two numbers", input_schema: Object {"properties": Object {"a": Object {"title": String("A"), "type": String("integer")}, "b": Object {"title": String("B"), "type": String("integer")}}, "required": Array [String("a"), String("b")], "title": String("addArguments"), "type": String("object")} }] }
        let mut result: Vec<Box<dyn crate::tools::tools_description::Tool + Send>> = vec![];
        tracing::info!("AAA integr_tools {:?}", self.mcp_tools);
        for tool in self.mcp_tools.iter() {
            tracing::info!("AAA      {:?}", tool.name);
            tracing::info!("AAA      {:?}", tool.description);
            tracing::info!("AAA      {:?}", tool.input_schema);
            result.push(Box::new(ToolMCP {
                common: self.common.clone(),
                config_path: self.config_path.clone(),
                mcp_client: self.mcp_client.clone().unwrap(),
                mcp_tool: tool.clone(),
            }));
        }
        // tracing::info!("AAAAA RETURN {:?}", result);
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
        tracing::info!("\nCALL\n\n\n");

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
                        tracing::error!("Tool execution error: {:?}", result.content);
                        return Err("Tool execution error".to_string());
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
