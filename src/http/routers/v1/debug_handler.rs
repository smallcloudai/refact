use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::call_validation::{ChatMessage, ContextFile};
use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;


#[derive(Serialize, Deserialize, Clone)]
struct CommandCompletionPost {
    file_uri: String,
}


pub async fn debug_fim_data(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    // let debug_data = global_context.read().await.debug_handler_data.lock().await.clone();

    let post = serde_json::from_slice::<CommandCompletionPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let mut vector_of_context_file: Vec<ContextFile> = vec![];
    vector_of_context_file.push(ContextFile {
        file_name: post.file_uri.replace("file://", ""),
        file_content: "test".to_string(),
        line1: 1,
        line2: 1,
        usefulness: 100.0,
    });
    vector_of_context_file.push(ContextFile {
        file_name: "/Users/valaises/RustroverProjects/refact-lsp/src/scratchpad_abstract.rs".to_string(),
        file_content: "test\ntest".to_string(),
        line1: 1,
        line2: 2,
        usefulness: 73.0,
    });
    vector_of_context_file.push(ContextFile {
        file_name: "/Users/valaises/RustroverProjects/refact-lsp/src/at_commands/utils.rs".to_string(),
        file_content: "test\ntest\ntest".to_string(),
        line1: 3,
        line2: 6,
        usefulness: 99.0,
    });

    let chat_message_mb = serde_json::to_string_pretty(&ChatMessage{
        role: "context_file".to_string(),
        content: json!(&vector_of_context_file).to_string(),
    });

    let body = match chat_message_mb{
        Ok(body) => body,
        Err(err) => {
            return Err(ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY ,format!("Error serializing data: {}", err)))
        },
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(body))
        .unwrap())
}
