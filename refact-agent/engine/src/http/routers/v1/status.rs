use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::Serialize;

use crate::ast::ast_structs::AstStatus;
use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;

#[derive(Serialize)]
struct RagStatus {
    ast: Option<AstStatus>,
    ast_alive: String,
    vecdb: Option<crate::vecdb::vdb_structs::VecDbStatus>,
    vecdb_alive: String,
    vec_db_error: String,
}

pub async fn handle_v1_rag_status(
    Extension(gcx): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let (vec_db_error, ast_module_mb, vectorizer_service_mb) = {
        let gcx_locked = gcx.read().await;
        (
            gcx_locked.vec_db_error.clone(),
            gcx_locked.ast_service.clone(),
            gcx_locked.vectorizer_service.clone()
        )
    };

    let (maybe_vecdb_status, vecdb_message) = if let Some(vectorizer_service) = vectorizer_service_mb {
        match crate::vecdb::vdb_highlev::get_status(vectorizer_service).await {
            Ok(Some(status)) => (Some(status), "working".to_string()),
            Ok(None) => (None, "turned_off".to_string()),
            Err(err) => (None, err.to_string()),
        }
    } else {
        (None, "turned_off".to_string())
    };

    let (maybe_ast_status, ast_message) = match &ast_module_mb {
        Some(ast_service) => {
            let ast_status = ast_service.lock().await.ast_status.clone();
            let status = ast_status.lock().await.clone();
            (Some(status), "working".to_string())
        }
        None => (None, "turned_off".to_string())
    };

    let status = RagStatus {
        ast: maybe_ast_status,
        ast_alive: ast_message,
        vecdb: maybe_vecdb_status,
        vecdb_alive: vecdb_message,
        vec_db_error,
    };

    let json_string = serde_json::to_string_pretty(&status).map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("JSON serialization problem: {}", e))
    })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(json_string))
        .unwrap())
}
