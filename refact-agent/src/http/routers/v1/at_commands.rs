use axum::response::Result;
use axum::Extension;
use hyper::{Body, Response, StatusCode};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use serde_json::json;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use strsim::jaro_winkler;
use itertools::Itertools;
use tokenizers::Tokenizer;
use tracing::info;

use crate::at_commands::execute_at::run_at_commands_locally;
use crate::cached_tokenizers;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::execute_at::{execute_at_commands_in_query, parse_words_from_line};
use crate::call_validation::{PostprocessSettings, SubchatParameters};
use crate::custom_error::ScratchError;
use crate::global_context::try_load_caps_quickly_if_not_present;
use crate::global_context::GlobalContext;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::at_commands::at_commands::filter_only_context_file_from_context_tool;
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::scratchpads::scratchpad_utils::max_tokens_for_rag_chat;
use crate::scratchpads::scratchpad_utils::HasRagResults;


#[derive(Serialize, Deserialize, Clone)]
struct CommandCompletionPost {
    query: String,
    cursor: i64,
    top_n: usize,
}
#[derive(Serialize, Deserialize, Clone)]
struct CommandCompletionResponse {
    completions: Vec<String>,
    replace: (i64, i64),
    is_cmd_executable: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct CommandPreviewPost {
    query: String,
    #[serde(default)]
    model: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Highlight {
    kind: String,
    pos1: i64,
    pos2: i64,
    ok: bool,
    reason: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CommandExecutePost {
    pub messages: Vec<ChatMessage>,
    pub n_ctx: usize,
    pub maxgen: usize,
    pub subchat_tool_parameters: IndexMap<String, SubchatParameters>, // tool_name: {model, allowed_context, temperature}
    pub postprocess_parameters: PostprocessSettings,
    pub model_name: String,
    pub chat_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommandExecuteResponse {
    pub messages: Vec<ChatMessage>,
    pub undroppable_msg_number: usize,
    pub any_context_produced: bool,
    pub messages_to_stream_back: Vec<serde_json::Value>,
}

pub async fn handle_v1_command_completion(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<CommandCompletionPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    let top_n = post.top_n;

    let fake_n_ctx = 4096;
    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(AtCommandsContext::new(
        global_context.clone(),
        fake_n_ctx,
        top_n,
        true,
        vec![],
        "".to_string(),
        false,
    ).await));

    let at_commands = {
        ccx.lock().await.at_commands.clone()
    };
    let at_command_names = at_commands.keys().map(|x|x.clone()).collect::<Vec<_>>();

    let mut completions: Vec<String> = vec![];
    let mut pos1 = -1; let mut pos2 = -1;
    let mut is_cmd_executable = false;

    if let Ok((query_line_val, cursor_rel, cursor_line_start)) = get_line_with_cursor(&post.query, post.cursor) {
        let query_line_val = query_line_val.chars().take(cursor_rel as usize).collect::<String>();
        let args = query_line_args(&query_line_val, cursor_rel, cursor_line_start, &at_command_names);
        info!("args: {:?}", args);
        (completions, is_cmd_executable, pos1, pos2) = command_completion(ccx.clone(), args,  post.cursor).await;
    }
    let completions: Vec<_> = completions.into_iter().unique().map(|x|format!("{} ", x)).collect();

    let response = CommandCompletionResponse {
        completions,
        replace: (pos1, pos2),
        is_cmd_executable,
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(serde_json::to_string(&response).unwrap()))
        .unwrap())
}

pub async fn handle_v1_command_preview(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<CommandPreviewPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    let mut query = post.query.clone();

    let caps = crate::global_context::try_load_caps_quickly_if_not_present(global_context.clone(), 0).await?;
    let (model_name, recommended_model_record) = {
        let caps_locked = caps.read().unwrap();
        let tmp = crate::caps::which_model_to_use(
                &caps_locked.code_chat_models,
                &post.model,
                &caps_locked.code_chat_default_model,
            );
        match tmp {
            Ok(x) => (x.0, x.1.clone()),
            Err(e) => {
                tracing::warn!("can't find model: {}", e);
                return Err(ScratchError::new(StatusCode::BAD_REQUEST, format!("can't find model: {}", e)))?;
            }
        }
    };
    let tokenizer_arc: Arc<StdRwLock<Tokenizer>> = match cached_tokenizers::cached_tokenizer(caps.clone(), global_context.clone(), model_name.clone()).await {
        Ok(x) => x,
        Err(e) => {
            tracing::warn!("can't load tokenizer for preview: {}", e);
            return Err(ScratchError::new(StatusCode::BAD_REQUEST, format!("can't load tokenizer for preview: {}", e)))?;
        }
    };

    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(AtCommandsContext::new(
        global_context.clone(),
        recommended_model_record.n_ctx,
        crate::http::routers::v1::chat::CHAT_TOP_N,
        true,
        vec![],
        "".to_string(),
        false,
    ).await));

    let (messages_for_postprocessing, vec_highlights) = execute_at_commands_in_query(
        ccx.clone(),
        &mut query
    ).await;

    let rag_n_ctx = max_tokens_for_rag_chat(recommended_model_record.n_ctx, 512);  // real maxgen may be different -- comes from request

    let mut preview: Vec<ChatMessage> = vec![];
    for exec_result in messages_for_postprocessing.iter() {
        // at commands exec() can produce both role="user" and role="assistant" messages
        if let ContextEnum::ChatMessage(raw_msg) = exec_result {
            preview.push(raw_msg.clone());
        }
    }

    let mut pp_settings = {
        let ccx_locked = ccx.lock().await;
        ccx_locked.postprocess_parameters.clone()
    };
    if pp_settings.max_files_n == 0 {
        pp_settings.max_files_n = crate::http::routers::v1::chat::CHAT_TOP_N;
    }

    let cf = postprocess_context_files(
        global_context.clone(),
        &mut filter_only_context_file_from_context_tool(&messages_for_postprocessing),
        tokenizer_arc.clone(),
        rag_n_ctx,
        false,
        &pp_settings,
    ).await;

    if !cf.is_empty() {
        let message = ChatMessage {
            role: "context_file".to_string(),
            content: ChatContent::SimpleText(serde_json::to_string(&cf).unwrap()),
            tool_calls: None,
            tool_call_id: "".to_string(),
            ..Default::default()
        };
        preview.push(message.clone());
    }

    let mut highlights = vec![];
    for h in vec_highlights {
        highlights.push(Highlight {
            kind: h.kind.clone(),
            pos1: h.pos1 as i64,
            pos2: h.pos2 as i64,
            ok: h.ok,
            reason: h.reason.unwrap_or_default(),
        })
    }
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(serde_json::to_string_pretty(
            &json!({"messages": preview, "model": model_name, "highlight": highlights})
        ).unwrap()))
        .unwrap())
}

pub async fn handle_v1_at_command_execute(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<CommandExecutePost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let caps = try_load_caps_quickly_if_not_present(global_context.clone(), 0).await?;
    let tokenizer = cached_tokenizers::cached_tokenizer(caps, global_context.clone(), post.model_name.clone()).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error loading tokenizer: {}", e)))?;

    let mut ccx = AtCommandsContext::new(
        global_context.clone(),
        post.n_ctx,
        crate::http::routers::v1::chat::CHAT_TOP_N,
        true,
        vec![],
        "".to_string(),
        false,
    ).await;
    ccx.subchat_tool_parameters = post.subchat_tool_parameters.clone();
    ccx.postprocess_parameters = post.postprocess_parameters.clone();
    let ccx_arc = Arc::new(AMutex::new(ccx));

    let mut has_rag_results = HasRagResults::new();
    let (messages, undroppable_msg_number, any_context_produced) = run_at_commands_locally(
        ccx_arc.clone(), tokenizer.clone(), post.maxgen, &post.messages, &mut has_rag_results).await;
    let messages_to_stream_back = has_rag_results.in_json;

    let response = CommandExecuteResponse {
        messages, messages_to_stream_back, undroppable_msg_number, any_context_produced };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string_pretty(&response).unwrap()))
        .unwrap())
}

fn get_line_with_cursor(query: &String, cursor: i64) -> Result<(String, i64, i64), ScratchError> {
    let mut cursor_rel = cursor;
    for line in query.lines() {
        let line_length = line.len() as i64;
        if cursor_rel <= line_length {
            return Ok((line.to_string(), cursor_rel, cursor - cursor_rel));
        }
        cursor_rel -= line_length + 1; // +1 to account for the newline character
    }
    return Err(ScratchError::new(StatusCode::EXPECTATION_FAILED, "incorrect cursor provided".to_string()));
}

async fn command_completion(
    ccx: Arc<AMutex<AtCommandsContext>>,
    args: Vec<QueryLineArg>,
    cursor_abs: i64,
) -> (Vec<String>, bool, i64, i64) {    // returns ([possible, completions], good_as_it_is)
    let mut args = args;
    let at_commands = {
        ccx.lock().await.at_commands.clone()
    };
    let at_command_names = at_commands.keys().map(|x|x.clone()).collect::<Vec<_>>();

    let q_cmd_with_index = args.iter().enumerate().find_map(|(index, x)| {
        x.value.starts_with("@").then(|| (x, index))
    });
    let (q_cmd, q_cmd_idx) = match q_cmd_with_index {
        Some((x, idx)) => (x.clone(), idx),
        None => return (vec![], false, -1, -1),
    };

    let cmd = match at_command_names.iter().find(|x|x == &&q_cmd.value).and_then(|x| at_commands.get(x)) {
        Some(x) => x,
        None => {
            return if !q_cmd.focused {
                (vec![], false, -1, -1)
            } else {
                (command_completion_options(ccx.clone(), &q_cmd.value).await, false, q_cmd.pos1, q_cmd.pos2)
            }
        }
    };
    args = args.iter().skip(q_cmd_idx + 1).map(|x|x.clone()).collect::<Vec<_>>();
    let cmd_params_cnt = cmd.lock().await.params().len();
    args.truncate(cmd_params_cnt);

    let can_execute = args.len() == cmd.lock().await.params().len();

    for (arg, param) in args.iter().zip(cmd.lock().await.params()) {
        let param_locked = param.lock().await;
        let is_valid = param_locked.is_value_valid(ccx.clone(), &arg.value).await;
        if !is_valid {
            return if arg.focused {
                (param_locked.param_completion(ccx.clone(), &arg.value).await, can_execute, arg.pos1, arg.pos2)
            } else {
                (vec![], false, -1, -1)
            }
        }
        if is_valid && arg.focused && param_locked.param_completion_valid() {
            return (param_locked.param_completion(ccx.clone(), &arg.value).await, can_execute, arg.pos1, arg.pos2);
        }
    }

    if can_execute {
        return (vec![], true, -1, -1);
    }

    // if command is not focused, and the argument is empty we should make suggestions
    if !q_cmd.focused {
        match cmd.lock().await.params().get(args.len()) {
            Some(param) => {
                return (param.lock().await.param_completion(ccx.clone(), &"".to_string()).await, false, cursor_abs, cursor_abs);
            },
            None => {}
        }
    }

    (vec![], false, -1, -1)
}

async fn command_completion_options(
    ccx: Arc<AMutex<AtCommandsContext>>,
    q_cmd: &String,
) -> Vec<String> {
    let at_commands = {
        ccx.lock().await.at_commands.clone()
    };
    let at_command_names = at_commands.keys().map(|x|x.clone()).collect::<Vec<_>>();
    at_command_names
        .iter()
        .filter(|command| command.starts_with(q_cmd))
        .map(|command| {
            (command, jaro_winkler(&command, q_cmd))
        })
        .sorted_by(|(_, dist1), (_, dist2)| dist1.partial_cmp(dist2).unwrap())
        .rev()
        .take(5)
        .map(|(command, _)| command.clone())
        .collect()
}

pub fn query_line_args(line: &String, cursor_rel: i64, cursor_line_start: i64, at_command_names: &Vec<String>) -> Vec<QueryLineArg> {
    let mut args: Vec<QueryLineArg> = vec![];
    for (text, pos1, pos2) in parse_words_from_line(line).iter().rev().cloned() {
        if at_command_names.contains(&text) && args.iter().any(|x|(x.value.contains("@") && x.focused) || at_command_names.contains(&x.value)) {
            break;
        }
        let mut x = QueryLineArg {
            value: text.clone(),
            pos1: pos1 as i64, pos2: pos2 as i64,
            focused: false,
        };
        x.focused = cursor_rel >= x.pos1 && cursor_rel <= x.pos2;
        x.pos1 += cursor_line_start;
        x.pos2 += cursor_line_start;
        args.push(x)
    }
    args.iter().rev().cloned().collect::<Vec<_>>()
}

#[derive(Debug, Clone)]
pub struct QueryLineArg {
    pub value: String,
    pub pos1: i64,
    pub pos2: i64,
    pub focused: bool,
}
