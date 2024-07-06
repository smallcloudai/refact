use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};

use crate::ast::ast_module::AstIndexStatus;
use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::vecdb::vdb_structs::VecDbStatus;


#[derive(Serialize, Deserialize, Clone)]
struct RagStatus {
    ast: Option<AstIndexStatus>,
    ast_alive: String,
    vecdb: Option<VecDbStatus>,
    vecdb_alive: String,
    vec_db_error: String,
}

pub async fn handle_v1_rag_status(
    Extension(gcx): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let (vec_db_module, vec_db_error, ast_module) = {
        let gcx_locked = gcx.write().await;
        (gcx_locked.vec_db.clone(), gcx_locked.vec_db_error.clone(), gcx_locked.ast_module.clone())
    };

    let (maybe_vecdb_status, vecdb_message) = match crate::vecdb::vdb_highlev::get_status(vec_db_module).await {
        Ok(Some(status)) => (Some(status), "working".to_string()),
        Ok(None) => (None, "turned_off".to_string()),
        Err(err) => (None, err.to_string()),
    };

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
        vec_db_error: vec_db_error,
    };
    let json_string = serde_json::to_string_pretty(&status).map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("JSON serialization problem: {}", e))
    })?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(json_string))
        .unwrap())
}
