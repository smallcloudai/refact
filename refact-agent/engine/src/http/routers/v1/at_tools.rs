use std::collections::HashMap;
use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::cached_tokenizers;
use crate::call_validation::{ChatMessage, ChatMeta, ChatToolCall, PostprocessSettings, SubchatParameters};
use crate::caps::resolve_chat_model;
use crate::http::http_post_json;
use crate::http::routers::v1::chat::CHAT_TOP_N;
use crate::integrations::docker::docker_container_manager::docker_container_get_host_lsp_port_to_connect;
use crate::tools::tools_description::{tool_description_list_from_yaml, tools_merged_and_filtered, MatchConfirmDenyResult};
use crate::custom_error::ScratchError;
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::tools::tools_execute::run_tools;


#[derive(Serialize, Deserialize, Clone)]
struct ToolsPermissionCheckPost {
    pub tool_calls: Vec<ChatToolCall>,
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub meta: ChatMeta,
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

pub async fn handle_v1_tools(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let all_tools = match tools_merged_and_filtered(gcx.clone(), true).await {
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

    let tool_desclist = tool_description_list_from_yaml(all_tools, Some(&turned_on), allow_experimental).await.unwrap_or_else(|e| {
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

    let is_inside_container = gcx.read().await.cmdline.inside_container;
    if post.meta.chat_remote && !is_inside_container {
        let port = docker_container_get_host_lsp_port_to_connect(gcx.clone(), &post.meta.chat_id).await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        let url = format!("http://localhost:{port}/v1/tools-check-if-confirmation-needed");
        let response: serde_json::Value = http_post_json( &url, &post).await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        return Ok(Response::builder()
           .status(StatusCode::OK)
          .header("Content-Type", "application/json")
          .body(Body::from(serde_json::to_string(&response).unwrap()))
          .unwrap());
    }

    let ccx = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(), 1000, 1, false, post.messages.clone(), "".to_string(), false
    ).await)); // used only for should_confirm

    let all_tools = match tools_merged_and_filtered(gcx.clone(), true).await {
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

        let should_confirm = tool.match_against_confirm_deny(ccx.clone(), &args).await
            .map_err(|e| { ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e)})?;

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
    let tools_execute_post = serde_json::from_slice::<ToolsExecutePost>(&body_bytes)
      .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await?;
    let model_rec = resolve_chat_model(caps, &tools_execute_post.model_name)
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let tokenizer = cached_tokenizers::cached_tokenizer(gcx.clone(), &model_rec.base).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error loading tokenizer: {}", e)))?;

    let mut ccx = AtCommandsContext::new(
        gcx.clone(),
        tools_execute_post.n_ctx,
        CHAT_TOP_N,
        false,
        tools_execute_post.messages.clone(),
        tools_execute_post.chat_id.clone(),
        false,
    ).await;
    ccx.subchat_tool_parameters = tools_execute_post.subchat_tool_parameters.clone();
    ccx.postprocess_parameters = tools_execute_post.postprocess_parameters.clone();
    let ccx_arc = Arc::new(AMutex::new(ccx));

    let mut at_tools = tools_merged_and_filtered(gcx.clone(), false).await.map_err(|e|{
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error getting at_tools: {}", e))
    })?;
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