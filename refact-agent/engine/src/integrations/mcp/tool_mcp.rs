
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use rmcp::model::{RawContent, CallToolRequestParam, Tool as McpTool};
use rmcp::{RoleClient, service::RunningService};
use tokio::sync::Mutex as AMutex;
use tokio::time::timeout;
use tokio::time::Duration;

use crate::caps::resolve_chat_model;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::integrations::sessions::get_session_hashmap_key;
use crate::scratchpads::multimodality::MultimodalElement;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::integrations::integr_abstract::{IntegrationCommon, IntegrationConfirmation};
use super::session_mcp::{_add_log_entry, _session_wait_startup_task};

pub struct ToolMCP {
    pub common: IntegrationCommon,
    pub config_path: String,
    pub mcp_client: Arc<AMutex<Option<RunningService<RoleClient, ()>>>>,
    pub mcp_tool: McpTool,
    pub request_timeout: u64,
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
        let session_key = get_session_hashmap_key("mcp", &self.config_path);
        let (gcx, current_model) = {
            let ccx_locked = ccx.lock().await;
            (ccx_locked.global_context.clone(), ccx_locked.current_model.clone())
        };
        let (session_maybe, caps_maybe) = {
            let gcx_locked = gcx.read().await;
            (gcx_locked.integration_sessions.get(&session_key).cloned(), gcx_locked.caps.clone())
        };
        if session_maybe.is_none() {
            tracing::error!("No session for {:?}, strange (2)", session_key);
            return Err(format!("No session for {:?}", session_key));
        }
        let session = session_maybe.unwrap();
        let model_supports_multimodality = caps_maybe.is_some_and(|caps| {
            resolve_chat_model(caps, &current_model).is_ok_and(|m| m.supports_multimodality)
        });
        _session_wait_startup_task(session.clone()).await;

        let json_args = serde_json::json!(args);
        tracing::info!("\n\nMCP CALL tool '{}' with arguments: {:?}", self.mcp_tool.name, json_args);

        let session_logs = {
            let mut session_locked = session.lock().await;
            let session_downcasted = session_locked.as_any_mut().downcast_mut::<super::session_mcp::SessionMCP>().unwrap();
            session_downcasted.logs.clone()
        };

        _add_log_entry(session_logs.clone(), format!("Executing tool '{}' with arguments: {:?}", self.mcp_tool.name, json_args)).await;

        let result_probably = {
            let mcp_client_locked = self.mcp_client.lock().await;
            if let Some(client) = &*mcp_client_locked {
                match timeout(Duration::from_secs(self.request_timeout),
                    client.call_tool(CallToolRequestParam {
                        name: self.mcp_tool.name.clone(),
                        arguments: match json_args {
                            serde_json::Value::Object(map) => Some(map),
                            _ => None,
                        },
                    })
                ).await {
                    Ok(result) => result,
                    Err(_) => {Err(rmcp::service::ServiceError::Timeout {
                        timeout: Duration::from_secs(self.request_timeout),
                    })},
                }
            } else {
                return Err("MCP client is not available".to_string());
            }
        };

        let result_message = match result_probably {
            Ok(result) => {
                if result.is_error.unwrap_or(false) {
                    let error_msg = format!("Tool execution error: {:?}", result.content);
                    _add_log_entry(session_logs.clone(), error_msg.clone()).await;
                    return Err(error_msg);
                }

                let mut elements = Vec::new();
                for content in result.content {
                    match content.raw {
                        RawContent::Text(text_content) => {
                            elements.push(MultimodalElement {
                                m_type: "text".to_string(),
                                m_content: text_content.text,
                            })
                        }
                        RawContent::Image(image_content) => {
                            if model_supports_multimodality {
                                let mime_type = if image_content.mime_type.starts_with("image/") {
                                    image_content.mime_type
                                } else {
                                    format!("image/{}", image_content.mime_type)
                                };
                                elements.push(MultimodalElement {
                                    m_type: mime_type,
                                    m_content: image_content.data,
                                })
                            } else {
                                elements.push(MultimodalElement {
                                    m_type: "text".to_string(),
                                    m_content: "Server returned an image, but model does not support multimodality".to_string(),
                                })
                            }
                        },
                        RawContent::Audio(_) => {
                            elements.push(MultimodalElement {
                                m_type: "text".to_string(),
                                m_content: "Server returned audio, which is not supported".to_string(),
                            })
                        },
                        RawContent::Resource(_) => {
                            elements.push(MultimodalElement {
                                m_type: "text".to_string(),
                                m_content: "Server returned resource, which is not supported".to_string(),
                            })
                        },
                    }
                }

                let content = if elements.iter().all(|el| el.m_type == "text") {
                    ChatContent::SimpleText(
                        elements.into_iter().map(|el| el.m_content).collect::<Vec<_>>().join("\n\n")
                    )
                } else {
                    ChatContent::Multimodal(elements)
                };

                ContextEnum::ChatMessage(ChatMessage {
                    role: "tool".to_string(),
                    content,
                    tool_calls: None,
                    tool_call_id: tool_call_id.clone(),
                    ..Default::default()
                })
            }
            Err(e) => {
                let error_msg = format!("Failed to call tool: {:?}", e);
                tracing::error!("{}", error_msg);
                _add_log_entry(session_logs.clone(), error_msg).await;
                return Err(e.to_string());
            }
        };

        Ok((false, vec![result_message]))
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
            description: self.mcp_tool.description.to_owned().unwrap_or_default().to_string(),
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
        Ok(command.to_string())
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(self.common.confirmation.clone())
    }

    fn has_config_path(&self) -> Option<String> {
        Some(self.config_path.clone())
    }
}
