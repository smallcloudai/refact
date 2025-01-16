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
use mcp_client_rs::{Protocol, ClientError};


use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
// use crate::tools::tools_description::{Tool, ToolDesc, ToolParam};
// use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::integrations::integr_abstract::{IntegrationTrait, IntegrationCommon, IntegrationConfirmation};


#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ConfigMCP {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Default)]
pub struct IntegrationMCP {
    pub common: IntegrationCommon,
    pub cfg: ConfigMCP,
    pub config_path: String,
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

        let mut envs = HashMap::new();
        // envs.insert(
        //     "GITHUB_PERSONAL_ACCESS_TOKEN".to_string(),
        //     std::env::var("GITHUB_PERSONAL_ACCESS_TOKEN").unwrap_or_default(),
        // );

        tracing::info!("AAA GEEEEE {:?}", self.config_path);

        let protocol_maybe: Result<Protocol, ClientError> = Protocol::new(
            "0",
            self.cfg.command.as_str(),
            self.cfg.args.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
            envs,
        ).await;

        if let Err(client_error) = protocol_maybe {
            tracing::error!("Failed to initialize protocol: {:?}", client_error);
            return Err(client_error.to_string());
        }

        let client = Arc::new(protocol_maybe.unwrap());

        Ok(())
    }

    fn integr_settings_as_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    fn integr_tools(&self, integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        vec![]
        // vec![Box::new(IntegrationMCP {
        //     common: self.common.clone(),
        //     name: integr_name.to_string(),
        //     // cfg: self.cfg.clone(),
        //     // config_path: self.config_path.clone(),
        // })]
    }

    fn integr_schema(&self) -> &str {
        MCP_INTEGRATION_SCHEMA
    }
}

// #[async_trait]
// impl Tool for ToolMcp {
//     fn as_any(&self) -> &dyn std::any::Any {
//         self
//     }

//     async fn tool_execute(
//         &mut self,
//         ccx: Arc<AMutex<AtCommandsContext>>,
//         tool_call_id: &String,
//         args: &HashMap<String, serde_json::Value>,
//     ) -> Result<(bool, Vec<ContextEnum>), String> {
//         // Initialize JSON-RPC request
//         let request = jsonrpc_core::Request {
//             jsonrpc: Some(String::from("2.0")),
//             method: self.name.clone(),
//             params: Params::Map(serde_json::Map::from_iter(args.clone().into_iter())),
//             id: jsonrpc_core::Id::Num(1),
//         };

//         // Execute the request
//         let response = match self.io_handler.handle_request(&serde_json::to_string(&request).unwrap()).await {
//             Ok(response) => response.unwrap_or_default(),
//             Err(e) => return Err(format!("RPC error: {}", e)),
//         };

//         // Format the response
//         let result = vec![ContextEnum::ChatMessage(ChatMessage {
//             role: "tool".to_string(),
//             content: ChatContent::SimpleText(response),
//             tool_calls: None,
//             tool_call_id: tool_call_id.clone(),
//             ..Default::default()
//         })];

//         Ok((false, result))
//     }

//     fn tool_depends_on(&self) -> Vec<String> {
//         vec![]
//     }

//     fn tool_description(&self) -> ToolDesc {
//         let parameters_required = self.cfg.parameters_required.clone().unwrap_or_else(|| {
//             self.cfg.parameters.iter().map(|param| param.name.clone()).collect()
//         });
//         ToolDesc {
//             name: self.name.clone(),
//             agentic: true,
//             experimental: false,
//             description: self.cfg.description.clone(),
//             parameters: self.cfg.parameters.clone(),
//             parameters_required,
//         }
//     }

//     fn command_to_match_against_confirm_deny(
//         &self,
//         _args: &HashMap<String, serde_json::Value>,
//     ) -> Result<String, String> {
//         Ok(self.name.clone())
//     }

//     fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
//         Some(self.integr_common().confirmation)
//     }

//     fn has_config_path(&self) -> Option<String> {
//         Some(self.config_path.clone())
//     }
// }

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
