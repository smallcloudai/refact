use std::sync::Arc;

use crate::call_validation::DiffChunk;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use axum::http::{Response, StatusCode};
use axum::Extension;
use hyper::Body;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock as ARwLock;

#[derive(Deserialize)]
pub struct FileEditDryRunPost {
    pub tool_name: String,
    pub tool_args: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
pub struct FileEditDryRunResponse {
    file_before: String,
    file_after: String,
    chunks: Vec<DiffChunk>,
}

pub async fn handle_v1_file_edit_tool_dry_run(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<FileEditDryRunPost>(&body_bytes).map_err(|e| {
        ScratchError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("JSON problem: {}", e),
        )
    })?;
    let (file_before, file_after, chunks) = match post.tool_name.as_str() {
        "create_textdoc" => {
            crate::tools::file_edit::tool_create_textdoc::tool_create_text_doc_exec(
                global_context.clone(),
                &post.tool_args,
                true,
            )
            .await
            .map_err(|x| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, x))?
        }
        "replace_textdoc" => {
            crate::tools::file_edit::tool_replace_textdoc::tool_replace_text_doc_exec(
                global_context.clone(),
                &post.tool_args,
                true,
            )
            .await
            .map_err(|x| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, x))?
        }
        "update_textdoc" => {
            crate::tools::file_edit::tool_update_textdoc::tool_update_text_doc_exec(
                global_context.clone(),
                &post.tool_args,
                true,
            )
            .await
            .map_err(|x| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, x))?
        }
        "update_textdoc_regex" => {
            crate::tools::file_edit::tool_update_textdoc_regex::tool_update_text_doc_regex_exec(
                global_context.clone(),
                &post.tool_args,
                true,
            )
            .await
            .map_err(|x| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, x))?
        }
        _ => {
            return Err(ScratchError::new(
                StatusCode::BAD_REQUEST,
                format!("Unknown tool name: {}", post.tool_name),
            ))
        }
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::to_string_pretty(&FileEditDryRunResponse {
                file_before,
                file_after,
                chunks,
            })
            .unwrap(),
        ))
        .unwrap())
}
