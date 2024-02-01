use axum::response::Result;
use axum::Extension;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use itertools::Itertools;
use serde_json::{json, Value};
use tokio::sync::RwLock as ARwLock;
use strsim::jaro_winkler;
use crate::at_commands::structs::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::query::QueryLine;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;

#[derive(Serialize, Deserialize, Clone)]
struct CommandCompletionPost {
    query: String,
    cursor: i64,
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
}

#[derive(Serialize, Deserialize, Clone)]
struct CommandPreviewResponse {
    messages: Vec<Value>,
}

pub async fn handle_v1_command_completion(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let context = AtCommandsContext::new(global_context.clone()).await;
    let post = serde_json::from_slice::<CommandCompletionPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let (query_line_val, cursor_rel, cursor_line_start) = get_line_with_cursor(&post.query, post.cursor.clone())?;
    let query_line_val = query_line_val.chars().take(cursor_rel as usize).collect::<String>();
    // info!(query_line_val);
    let query_line = QueryLine::new(query_line_val, cursor_rel, cursor_line_start);
    // for arg in query_line.args.iter() {
    //     info!("value: {}, focused: {}, type_name: {}; pos1: {}; pos2: {}", arg.value, arg.focused, arg.type_name, arg.pos1, arg.pos2);
    // }
    let (completions, is_cmd_executable, pos1, pos2) = command_completion(&query_line, &context).await?;

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
    let context = AtCommandsContext::new(global_context.clone()).await;
    let post = serde_json::from_slice::<CommandPreviewPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    let mut query = post.query.clone();
    let valid_commands = crate::at_commands::utils::find_valid_at_commands_in_query(&mut query, &context).await;
    if valid_commands.is_empty() {
        return Err(ScratchError::new(StatusCode::OK, "no valid commands in query".to_string()));
    }

    let mut preview_msgs = vec![];
    for cmd in valid_commands {
        match cmd.command.lock().await.execute(&post.query, &cmd.args, 5, &context).await {
            Ok(msg) => {
                preview_msgs.push(json!(msg));
            },
            Err(_) => {}
        }
    }

    let response = CommandPreviewResponse {
        messages: preview_msgs,
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(serde_json::to_string(&response).unwrap()))
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
    return Err(ScratchError::new(StatusCode::OK, "cursor is incorrect".to_string()));
}

async fn command_completion(
    query_line: &QueryLine,
    context: &AtCommandsContext,
) -> Result<(Vec<String>, bool, i64, i64), ScratchError> {    // returns ([possible, completions], good_as_it_is)
    let q_cmd = match query_line.command() {
        Some(x) => x,
        None => { return Err(ScratchError::new(StatusCode::OK, "no command given".to_string()));}
    };

    let (_, cmd) = match context.at_commands.iter().find(|&(k, _v)| k == &q_cmd.value) {
        Some(x) => x,
        None => {
            if !q_cmd.focused {
                return Err(ScratchError::new(StatusCode::OK, "incorrect command given".to_string()));
            }
            return Ok((command_completion_options(&q_cmd.value, &context, 5).await, false, q_cmd.pos1, q_cmd.pos2));
        }
    };
    if cmd.lock().await.can_execute(&query_line.get_args().iter().map(|x|x.value.clone()).collect(), context).await {
        return Ok((vec![], true, -1, -1));
    }

    for (arg, param) in query_line.get_args().iter().zip(cmd.lock().await.params()) {
        let is_valid = param.lock().await.is_value_valid(&arg.value, context).await;
        if !is_valid {
            return if arg.focused {
                Ok((param.lock().await.complete(&arg.value, context, 5).await, false, arg.pos1, arg.pos2))
            } else {
                Err(ScratchError::new(StatusCode::OK, "invalid parameter".to_string()))
            }
        }

    }

    return Ok((vec![], false, -1, -1));
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
