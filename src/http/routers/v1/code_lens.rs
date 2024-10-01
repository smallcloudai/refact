use serde::{Deserialize, Serialize};
use std::sync::Arc;
use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use url::Url;

use crate::global_context::SharedGlobalContext;
use crate::ast::ast_structs::AstDefinition;
use crate::custom_error::ScratchError;


#[derive(Deserialize)]
pub struct CodeLensPost {
    pub uri: Url,
}

#[derive(Serialize)]
struct CodeLensResponse {
    success: u8,
    code_lens: Vec<CodeLensOutput>,
}

#[derive(Serialize)]
struct CodeLensOutput {
    spath: String,
    line1: usize,
    line2: usize,
}

pub async fn handle_v1_code_lens(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<CodeLensPost>(&body_bytes).map_err(|e| {
        tracing::info!("chat handler cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let cpath = crate::files_correction::canonical_path(&post.uri.to_file_path().unwrap_or_default().to_string_lossy().to_string());
    let cpath_str = cpath.to_string_lossy().to_string();

    let ast_service_opt = global_context.read().await.ast_service.clone();
    let defs: Vec<Arc<AstDefinition>> = if let Some(ast_service) = ast_service_opt {
        crate::ast::ast_indexer_thread::ast_indexer_block_until_finished(ast_service.clone(), 300, true).await;
        let ast_index = ast_service.lock().await.ast_index.clone();
        crate::ast::ast_db::doc_defs(ast_index, &cpath_str).await
    } else {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(serde_json::json!({"detail": "AST turned off"}).to_string()))
            .unwrap())
    };

    let mut output: Vec<CodeLensOutput> = Vec::new();
    for def in defs.iter() {
        let line1 = def.full_line1();
        let line2 = def.full_line2();
        if line2 > line1 {
            output.push(CodeLensOutput {
                spath: def.path_drop0(),
                line1,
                line2,
            });
        }
    }

    let response = CodeLensResponse {
        success: 1,
        code_lens: output,
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(serde_json::to_string(&response).unwrap()))
        .unwrap())
}
