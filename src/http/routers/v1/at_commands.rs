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

use crate::cached_tokenizers;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::query::QueryLine;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::call_validation::ChatMessage;


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

pub async fn handle_v1_command_completion(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let context = AtCommandsContext::new(global_context.clone()).await;
    let post = serde_json::from_slice::<CommandCompletionPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let mut completions: Vec<String> = vec![];
    let mut pos1 = -1; let mut pos2 = -1;
    let mut is_cmd_executable = false;

    if let Ok((query_line_val, cursor_rel, cursor_line_start)) = get_line_with_cursor(&post.query, post.cursor) {
        let query_line_val = query_line_val.chars().take(cursor_rel as usize).collect::<String>();
        let query_line = QueryLine::new(query_line_val, cursor_rel, cursor_line_start);
        (completions, is_cmd_executable, pos1, pos2) = command_completion(&query_line, &context, post.cursor, post.top_n).await;
    }

    let response = CommandCompletionResponse {
        completions: completions.clone(),
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

    let mut messages_for_postprocessing = vec![];
    let top_n = 5;
    let at_context = AtCommandsContext::new(global_context.clone()).await;
    let valid_commands = crate::at_commands::utils::find_valid_at_commands_in_query(&mut query, &at_context).await;
    for cmd in valid_commands {
        match cmd.command.lock().await.execute(&query, &cmd.args, top_n, &at_context).await {
            Ok(msg) => {
                messages_for_postprocessing.push(msg);
            },
            Err(e) => {
                tracing::warn!("can't execute command that indicated it can execute: {}", e);
            }
        }
    }
    let processed = crate::scratchpads::chat_utils_rag::postprocess_at_results2(
        global_context.clone(),
        messages_for_postprocessing,
        tokenizer_arc.clone(),
        recommended_model_record.n_ctx,
    ).await;
    let mut preview: Vec<ChatMessage> = vec![];
    if processed.len() > 0 {
        let message = ChatMessage {
            role: "context_file".to_string(),
            content: serde_json::to_string(&processed).unwrap(),
        };
        preview.push(message.clone());
    }
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(serde_json::to_string(
            &json!({"messages": preview, "model": model_name})
        ).unwrap()))
        .unwrap())
}

fn get_line_with_cursor(query: &String, cursor: i64) -> Result<(String, i64, i64), ScratchError> {
    let mut cursor_rel = cursor;
    for line in query.lines() {
        let line_length = line.len() as i64;
        if cursor_rel <= line_length {
            if !line.starts_with("@") {
                return Err(ScratchError::new(StatusCode::OK, "no command provided".to_string()));
            }
            return Ok((line.to_string(), cursor_rel, cursor - cursor_rel));
        }
        cursor_rel -= line_length + 1; // +1 to account for the newline character
    }
    return Err(ScratchError::new(StatusCode::EXPECTATION_FAILED, "incorrect cursor provided".to_string()));
}

async fn command_completion(
    query_line: &QueryLine,
    context: &AtCommandsContext,
    cursor_abs: i64,
    top_n: usize,
) -> (Vec<String>, bool, i64, i64) {    // returns ([possible, completions], good_as_it_is)
    let q_cmd = match query_line.command() {
        Some(x) => x,
        None => { return (vec![], false, -1, -1)}
    };

    let (_, cmd) = match context.at_commands.iter().find(|&(k, _v)| k == &q_cmd.value) {
        Some(x) => x,
        None => {
            return if !q_cmd.focused {
                (vec![], false, -1, -1)
            } else {
                (command_completion_options(&q_cmd.value, &context, top_n).await, false, q_cmd.pos1, q_cmd.pos2)
            }
        }
    };

    let can_execute = cmd.lock().await.can_execute(&query_line.get_args().iter().map(|x|x.value.clone()).collect(), context).await;

    for (arg, param) in query_line.get_args().iter().zip(cmd.lock().await.params()) {
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
        match cmd.lock().await.params().get(query_line.get_args().len()) {
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
    let at_commands_names = context.at_commands.iter().map(|(name, _cmd)| name.clone()).collect::<Vec<String>>();
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
