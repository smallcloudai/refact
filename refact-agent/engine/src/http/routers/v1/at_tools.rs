use std::collections::HashMap;
use std::sync::Arc;
use axum::{Extension, Json};
use axum::http::{Response, StatusCode};
use hyper::Body;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatToolCall, PostprocessSettings, SubchatParameters};
use crate::caps::resolve_chat_model;
use crate::indexing_utils::wait_for_indexing_if_needed;
use crate::tools::tools_description::{set_tool_config, MatchConfirmDenyResult, ToolConfig, ToolDesc, ToolGroupCategory, ToolSource};
use crate::tools::tools_list::{get_available_tool_groups, get_available_tools};
use crate::custom_error::ScratchError;
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::tools::tools_execute::run_tools;


#[derive(Serialize, Deserialize, Clone)]
struct ToolsPermissionCheckPost {
    pub tool_calls: Vec<ChatToolCall>,
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum PauseReasonType {
    Confirmation,
    Denial,
}

#[derive(Serialize)]
struct PauseReason {
    #[serde(rename = "type")]
    reason_type: PauseReasonType,
    command: String,
    rule: String,
    tool_call_id: String,
    integr_config_path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ToolsExecutePost {
    pub messages: Vec<ChatMessage>,
    pub n_ctx: usize,
    pub maxgen: usize,
    pub subchat_tool_parameters: IndexMap<String, SubchatParameters>, // tool_name: {model, allowed_context, temperature}
    pub postprocess_parameters: PostprocessSettings,
    pub model_name: String,
    pub chat_id: String,
    pub style: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolExecuteResponse {
    pub messages: Vec<ChatMessage>,
    pub tools_ran: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ToolResponse {
    pub spec: ToolDesc,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ToolGroupResponse {
    pub name: String,
    pub description: String,
    pub category: ToolGroupCategory,
    pub tools: Vec<ToolResponse>,
}

pub async fn handle_v1_get_tools(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
) -> Json<Vec<ToolGroupResponse>> {
    let tool_groups = get_available_tool_groups(gcx.clone()).await;

    let tool_groups: Vec<ToolGroupResponse> = tool_groups.into_iter().filter_map(|tool_group| {
        if tool_group.tools.is_empty() {
            return None;
        }

        let tools: Vec<ToolResponse> = tool_group.tools.into_iter().map(|tool| {
            let spec = tool.tool_description();
            ToolResponse {
                spec,
                enabled: tool.config().unwrap_or_default().enabled,
            }
        }).collect();

        Some(ToolGroupResponse {
            name: tool_group.name,
            description: tool_group.description,
            category: tool_group.category,
            tools,
        })
    }).collect();

    Json(tool_groups)
}

#[derive(Deserialize)]
pub struct ToolPost {
    name: String,
    source: ToolSource,
    enabled: bool,
}

#[derive(Deserialize)]
pub struct ToolPostReq {
    tools: Vec<ToolPost>,
}

#[derive(Serialize)]
pub struct ToolPostResponse {
    success: bool,
}

pub async fn handle_v1_post_tools(
    body_bytes: hyper::body::Bytes,
) -> Result<Json<ToolPostResponse>, ScratchError> {
    let tools = serde_json::from_slice::<ToolPostReq>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?
        .tools;

    for tool in tools {
        set_tool_config(
            tool.source.config_path, 
            tool.name,
            ToolConfig {
                enabled: tool.enabled,
            }
        ).await.map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error setting tool config: {}", e)))?;
    }

    Ok(Json(ToolPostResponse {
        success: true,
    }))
}

pub async fn handle_v1_tools_check_if_confirmation_needed(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    fn reply(pause: bool, pause_reasons: &Vec<PauseReason>) -> Response<Body> {
        let body = serde_json::json!({
            "pause": pause,
            "pause_reasons": pause_reasons
        }).to_string();
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from(body))
            .unwrap()
    }

    let post = serde_json::from_slice::<ToolsPermissionCheckPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let ccx = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        1000,
        1,
        false,
        post.messages.clone(),
        "".to_string(),
        false
    ).await)); // used only for should_confirm

    let all_tools = get_available_tools(gcx.clone()).await.into_iter()
        .map(|tool| {
            let spec = tool.tool_description();
            (spec.name, tool)
        }).collect::<IndexMap<_, _>>();

    let mut result_messages = vec![];
    for tool_call in &post.tool_calls {
        let tool = match all_tools.get(&tool_call.function.name) {
            Some(x) => x,
            None => {
                tracing::error!("Unknown tool: {}", tool_call.function.name);
                // Not returning error here, because we don't want to stop the chat, it will fail later
                // in `/chat` and provide error to the model
                continue;
            }
        };

        let args = match serde_json::from_str::<HashMap<String, Value>>(&tool_call.function.arguments) {
            Ok(args) => args,
            Err(e) => {
                return Ok(reply(false, &vec![
                    PauseReason {
                        reason_type: PauseReasonType::Denial,
                        command: tool_call.function.name.clone(),
                        rule: format!("tool parsing problem: {}", e),
                        tool_call_id: tool_call.id.clone(),
                        integr_config_path: tool.has_config_path(),
                    }
                ]));
            }
        };

        let should_confirm = match tool.match_against_confirm_deny(ccx.clone(), &args).await {
            Ok(should_confirm) => should_confirm,
            Err(e) => {
                tracing::error!("Error getting tool command to match: {e}");
                // Not returning error here, because we don't want to stop the chat, it will fail later
                // in `/chat` and provide error to the model
                continue;
            }
        };

        match should_confirm.result {
            MatchConfirmDenyResult::DENY => {
                result_messages.push(PauseReason {
                    reason_type: PauseReasonType::Denial,
                    command: should_confirm.command.clone(),
                    rule: should_confirm.rule.clone(),
                    tool_call_id: tool_call.id.clone(),
                    integr_config_path: tool.has_config_path(),
                });
            },
            MatchConfirmDenyResult::CONFIRMATION => {
                result_messages.push(PauseReason {
                    reason_type: PauseReasonType::Confirmation,
                    command: should_confirm.command.clone(),
                    rule: should_confirm.rule.clone(),
                    tool_call_id: tool_call.id.clone(),
                    integr_config_path: tool.has_config_path(),
                });
            },
            _ => {},
        }
    }

    Ok(reply(!result_messages.is_empty(), &result_messages))
}

pub async fn handle_v1_tools_execute(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    wait_for_indexing_if_needed(gcx.clone()).await;

    let tools_execute_post = serde_json::from_slice::<ToolsExecutePost>(&body_bytes)
      .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await?;
    let model_rec = resolve_chat_model(caps, &tools_execute_post.model_name)
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let tokenizer = crate::tokens::cached_tokenizer(gcx.clone(), &model_rec.base).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let mut ccx = AtCommandsContext::new(
        gcx.clone(),
        tools_execute_post.n_ctx,
        crate::http::routers::v1::at_commands::CHAT_TOP_N,
        false,
        tools_execute_post.messages.clone(),
        tools_execute_post.chat_id.clone(),
        false
    ).await;
    ccx.subchat_tool_parameters = tools_execute_post.subchat_tool_parameters.clone();
    ccx.postprocess_parameters = tools_execute_post.postprocess_parameters.clone();
    let ccx_arc = Arc::new(AMutex::new(ccx));

    let mut at_tools = get_available_tools(gcx.clone()).await.into_iter()
        .map(|tool| {
            let spec = tool.tool_description();
            (spec.name, tool)
        }).collect::<IndexMap<_, _>>();

    let (messages, tools_ran) = run_tools(
        ccx_arc.clone(), &mut at_tools, tokenizer.clone(), tools_execute_post.maxgen, &tools_execute_post.messages, &tools_execute_post.style
    ).await.map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error running tools: {}", e)))?;

    let response = ToolExecuteResponse {
        messages,
        tools_ran,
    };

    let response_json = serde_json::to_string(&response)
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Response JSON problem: {}", e)))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(response_json))
        .unwrap()
    )
}
