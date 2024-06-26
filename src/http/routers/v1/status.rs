use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};

use crate::ast::ast_module::AstIndexStatus;
use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::vecdb::structs::VecDbStatus;

#[derive(Serialize, Deserialize, Clone)]
struct RagStatus {
    ast: Option<AstIndexStatus>,
    ast_alive: String,
    vecdb: Option<VecDbStatus>,
    vecdb_alive: String,
    vec_db_error: String,
}

pub async fn handle_v1_rag_status(
    Extension(global_context): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let cx_locked = global_context.read().await;
    let (maybe_vecdb_status, vecdb_message) = match *cx_locked.vec_db.lock().await {
        Some(ref db) => match db.get_status().await {
            Ok(status) => (Some(status), "working".to_string()),
            Err(err) => (None, err)
        },
        None => (None, "turned_off".to_string())
    };
    let ast_module = cx_locked.ast_module.clone();
    let (maybe_ast_status, ast_message) = match &ast_module {
        Some(ast) => {
            let status = ast.read().await.ast_index_status().await;
            (Some(status), "working".to_string())
        }
        None => (None, "turned_off".to_string())
    };

    let status = RagStatus {
        ast: maybe_ast_status,
        ast_alive: ast_message,
        vecdb: maybe_vecdb_status,
        vecdb_alive: vecdb_message,
        vec_db_error: cx_locked.vec_db_error.clone()
    };
    let json_string = serde_json::to_string_pretty(&status).map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("JSON serialization problem: {}", e))
    })?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(json_string))
        .unwrap())
}
