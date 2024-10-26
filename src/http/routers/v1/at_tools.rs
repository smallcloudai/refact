use std::collections::HashMap;
use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::ChatToolCall;
use crate::tools::tools_description::{commands_require_confirmation_rules_from_integrations_yaml, tool_description_list_from_yaml, tools_merged_and_filtered};
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::tools::tools_execute::{command_should_be_confirmed_by_user, command_should_be_denied};


#[derive(Serialize, Deserialize, Clone)]
struct ToolsPermissionCheckPost {
    pub tool_calls: Vec<ChatToolCall>,
}


pub async fn handle_v1_tools(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let all_tools = match tools_merged_and_filtered(gcx.clone()).await {
        Ok(tools) => tools,
        Err(e) => {
            let error_body = serde_json::json!({ "detail": e }).to_string();
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Body::from(error_body))
                .unwrap());
        }
    };

    let turned_on = all_tools.keys().cloned().collect::<Vec<_>>();
    let allow_experimental = gcx.read().await.cmdline.experimental;

    let tool_desclist = tool_description_list_from_yaml(all_tools, &turned_on, allow_experimental).await.unwrap_or_else(|e| {
        tracing::error!("Error loading compiled_in_tools: {:?}", e);
        vec![]
    });

    let tools_openai_stype = tool_desclist.into_iter().map(|x| x.into_openai_style()).collect::<Vec<_>>();

    let body = serde_json::to_string_pretty(&tools_openai_stype).map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap())
}

pub async fn handle_v1_tools_check_if_confirmation_needed(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<ToolsPermissionCheckPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let all_tools = match tools_merged_and_filtered(gcx.clone()).await {
        Ok(tools) => tools,
        Err(e) => {
            let error_body = serde_json::json!({ "detail": e }).to_string();
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Body::from(error_body))
                .unwrap());
        }
    };

    let mut result_messages = vec![];
    let mut confirmation_rules = None;
    for tool_call in &post.tool_calls {
        let tool = match all_tools.get(&tool_call.function.name) {
            Some(x) => x,
            None => {
                return Err(ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("Unknown tool: {}", tool_call.function.name)))
            }
        };

        let args = match serde_json::from_str::<HashMap<String, Value>>(&tool_call.function.arguments) {
            Ok(args) => args,
            Err(e) => {
                return Err(ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)));
            }
        };

        let command_to_match = {
            let tool_locked = tool.lock().await;
            tool_locked.command_to_match_against_confirm_deny(&args)
        }.map_err(|e| {
            ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("Error getting tool command to match: {}", e))
        })?;

        if !command_to_match.is_empty() {
            if confirmation_rules.is_none() {
                confirmation_rules = Some(commands_require_confirmation_rules_from_integrations_yaml(gcx.clone()).await.map_err(|e| {
                    ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error loading generic tool config: {}", e))
                })?);
            }

            if let Some(rules) = &confirmation_rules {
                let (is_denied, deny_reason) = command_should_be_denied(&command_to_match, &rules.commands_deny, true);
                if is_denied {
                    result_messages.push(deny_reason);
                }
                let (needs_confirmation, confirmation_reason) = command_should_be_confirmed_by_user(&command_to_match, &rules.commands_need_confirmation);
                if needs_confirmation {
                    result_messages.push(confirmation_reason);
                }
            }
        }
    }

    let body = serde_json::json!({
        "pause": !result_messages.is_empty(),
        "pause_reasons": result_messages,
    }).to_string();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap())
}