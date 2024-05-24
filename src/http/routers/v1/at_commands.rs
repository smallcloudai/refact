use axum::response::Result;
use axum::Extension;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use serde_json::json;
use tokio::sync::RwLock as ARwLock;
use strsim::jaro_winkler;
use itertools::Itertools;
use tokenizers::Tokenizer;
use tracing::info;

use crate::cached_tokenizers;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::query::{query_line_args, QueryLineArg};
use crate::at_commands::execute::execute_at_commands_in_query;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::call_validation::ChatMessage;
use crate::scratchpads::chat_utils_rag::{max_tokens_for_rag_chat, postprocess_at_results2};
use crate::at_commands::at_commands::filter_only_context_file_from_context_tool;
use crate::at_commands::at_commands_dict::at_commands_dicts;


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

pub async fn handle_v1_tools_available(
    Extension(_global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    _: hyper::body::Bytes,
)  -> Result<Response<Body>, ScratchError> {
    let at_dict = at_commands_dicts().map_err(|e| {
            tracing::warn!("can't load at_commands_dicts: {}", e);
            return ScratchError::new(StatusCode::NOT_FOUND, format!("can't load at_commands_dicts: {}", e));
        })?;
    let body = serde_json::to_string_pretty(
        &at_dict.iter().map(|x|x.clone().into_openai_style()).collect::<Vec<_>>()
    ).map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(body))
        .unwrap()
    )
}

pub async fn handle_v1_command_completion(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let context = AtCommandsContext::new(global_context.clone()).await;
    let at_command_names = context.at_commands.keys().map(|x|x.clone()).collect::<Vec<_>>();
    let post = serde_json::from_slice::<CommandCompletionPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let mut completions: Vec<String> = vec![];
    let mut pos1 = -1; let mut pos2 = -1;
    let mut is_cmd_executable = false;

    if let Ok((query_line_val, cursor_rel, cursor_line_start)) = get_line_with_cursor(&post.query, post.cursor) {
        let query_line_val = query_line_val.chars().take(cursor_rel as usize).collect::<String>();
        let args = query_line_args(&query_line_val, cursor_rel, cursor_line_start, &at_command_names);
        info!("args: {:?}", args);
        (completions, is_cmd_executable, pos1, pos2) = command_completion(args, &context, post.cursor, post.top_n).await;
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
                return Err(ScratchError::new(StatusCode::EXPECTATION_FAILED, format!("can't find model: {}", e)))?;
            }
        }
    };
    let tokenizer_arc: Arc<StdRwLock<Tokenizer>> = match cached_tokenizers::cached_tokenizer(caps.clone(), global_context.clone(), model_name.clone()).await {
        Ok(x) => x,
        Err(e) => {
            tracing::warn!("can't load tokenizer for preview: {}", e);
            return Err(ScratchError::new(StatusCode::EXPECTATION_FAILED, format!("can't load tokenizer for preview: {}", e)))?;
        }
    };

    let top_n = 10;  // sync with top_n in chats

    let at_context = AtCommandsContext::new(global_context.clone()).await;

    let (messages_for_postprocessing, vec_highlights) = execute_at_commands_in_query(&mut query, &at_context, false, top_n).await;

    let rag_n_ctx = max_tokens_for_rag_chat(recommended_model_record.n_ctx, 512);  // real maxgen may be different -- comes from request
    let processed = postprocess_at_results2(
        global_context.clone(),
        &filter_only_context_file_from_context_tool(&messages_for_postprocessing),
        tokenizer_arc.clone(),
        rag_n_ctx,
        false,
    ).await;
    let mut preview: Vec<ChatMessage> = vec![];
    if processed.len() > 0 {
        for p in processed {
            let message = ChatMessage::new(
                "tool".to_string(),
                serde_json::to_string(&p).unwrap()
            );
            preview.push(message.clone());
        }
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
        .body(Body::from(serde_json::to_string(
            &json!({"messages": preview, "model": model_name, "highlight": highlights})
        ).unwrap()))
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
    args: Vec<QueryLineArg>,
    context: &AtCommandsContext,
    cursor_abs: i64,
    top_n: usize,
) -> (Vec<String>, bool, i64, i64) {    // returns ([possible, completions], good_as_it_is)
    let mut args = args;
    let at_command_names = context.at_commands.keys().map(|x|x.clone()).collect::<Vec<_>>();

    let q_cmd_with_index = args.iter().enumerate().find_map(|(index, x)| {
        x.value.starts_with("@").then(|| (x, index))
    });
    let (q_cmd, q_cmd_idx) = match q_cmd_with_index {
        Some((x, idx)) => (x.clone(), idx),
        None => return (vec![], false, -1, -1),
    };

    let cmd = match at_command_names.iter().find(|x|x == &&q_cmd.value).and_then(|x|context.at_commands.get(x)) {
        Some(x) => x,
        None => {
            return if !q_cmd.focused {
                (vec![], false, -1, -1)
            } else {
                (command_completion_options(&q_cmd.value, &context, top_n).await, false, q_cmd.pos1, q_cmd.pos2)
            }
        }
    };
    args = args.iter().skip(q_cmd_idx + 1).map(|x|x.clone()).collect::<Vec<_>>();
    let cmd_params_cnt = cmd.lock().await.params().len();
    args.truncate(cmd_params_cnt);

    let can_execute = args.len() == cmd.lock().await.params().len();

    for (arg, param) in args.iter().zip(cmd.lock().await.params()) {
        let param_locked = param.lock().await;
        let is_valid = param_locked.is_value_valid(&arg.value, context).await;
        if !is_valid {
            return if arg.focused {
                (param_locked.complete(&arg.value, context, top_n).await, can_execute, arg.pos1, arg.pos2)
            } else {
                (vec![], false, -1, -1)
            }
        }
        if is_valid && arg.focused && param_locked.complete_if_valid() {
            return (param_locked.complete(&arg.value, context, top_n).await, can_execute, arg.pos1, arg.pos2);
        }
    }

    if can_execute {
        return (vec![], true, -1, -1);
    }

    // if command is not focused, and the argument is empty we should make suggestions
    if !q_cmd.focused {
        match cmd.lock().await.params().get(args.len()) {
            Some(param) => {
                return (param.lock().await.complete(&"".to_string(), context, top_n).await, false, cursor_abs, cursor_abs);
            },
            None => {}
        }
    }

    (vec![], false, -1, -1)
}

async fn command_completion_options(
    q_cmd: &String,
    context: &AtCommandsContext,
    top_n: usize,
) -> Vec<String> {
    let (ast_on, vecdb_on) = {
        let gcx = context.global_context.read().await;
        let vecdb = gcx.vec_db.lock().await;
        (gcx.ast_module.is_some(), vecdb.is_some())
    };
    let mut filtered_commands = vec![];
    for (name, cmd) in context.at_commands.iter() {
        let satisfied = cmd.lock().await.depends_on().iter().all(|d| {
            d == "ast" && ast_on || d == "vecdb" && vecdb_on
        });
        if satisfied {
            filtered_commands.push((name.clone(), cmd.clone()));
        }
    }
    let at_commands_names = filtered_commands.iter().map(|(name, _cmd)| name.clone()).collect::<Vec<String>>();
    at_commands_names
        .iter()
        .filter(|command| command.starts_with(q_cmd))
        .map(|command| {
            (command, jaro_winkler(&command, q_cmd))
        })
        .sorted_by(|(_, dist1), (_, dist2)| dist1.partial_cmp(dist2).unwrap())
        .rev()
        .take(top_n)
        .map(|(command, _)| command.clone())
        .collect()
}
