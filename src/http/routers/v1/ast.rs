use std::path::PathBuf;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::fs::read_to_string;
use tree_sitter::Point;

use crate::custom_error::ScratchError;
use crate::files_in_workspace::DocumentInfo;
use crate::global_context::SharedGlobalContext;

#[derive(Serialize, Deserialize, Clone)]
struct AstCursorSearchPost {
    filename: String,
    row: usize,
    column: usize,
    top_n: usize,
}

#[derive(Serialize, Deserialize, Clone)]
struct AstQuerySearchPost {
    query: String,
    top_n: usize,
}


#[derive(Serialize, Deserialize, Clone)]
struct AstReferencesPost {
    filename: String
}

pub async fn handle_v1_ast_cursor_search(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<AstCursorSearchPost>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let cx_locked = global_context.read().await;
    let search_res = match *cx_locked.ast_module.lock().await {
        Some(ref ast) => {
            let doc = match DocumentInfo::from(&PathBuf::from(&post.filename)).ok() {
                Some(doc) => doc,
                None => return Err(ScratchError::new(StatusCode::BAD_REQUEST, format!("Filename could not be parsed: {}", post.filename))),
            };
            let code = match doc.read_file().await {
                Ok(s) => s,
                Err(e) => { return Err(ScratchError::new(StatusCode::BAD_REQUEST, e.to_string())); }
            };
            ast.search_by_cursor(
                &doc, code.as_str(), Point::new(post.row, post.column), post.top_n,
            ).await
        }
        None => {
            return Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, "Ast module is not available".to_string(),
            ));
        }
    };
    match search_res {
        Ok(search_res) => {
            let json_string = serde_json::to_string_pretty(&search_res).map_err(|e| {
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("JSON serialization problem: {}", e))
            })?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(json_string))
                .unwrap())
        }
        Err(e) => {
            Err(ScratchError::new(StatusCode::BAD_REQUEST, e))
        }
    }
}

pub async fn handle_v1_ast_query_search(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<AstQuerySearchPost>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let cx_locked = global_context.read().await;
    let search_res = match *cx_locked.ast_module.lock().await {
        Some(ref ast) => {
            ast.search_by_symbol_path(
                post.query,
                post.top_n,
            ).await
        }
        None => {
            return Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, "Ast module is not available".to_string(),
            ));
        }
    };

    match search_res {
        Ok(search_res) => {
            let json_string = serde_json::to_string_pretty(&search_res).map_err(|e| {
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("JSON serialization problem: {}", e))
            })?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(json_string))
                .unwrap())
        }
        Err(e) => {
            Err(ScratchError::new(StatusCode::BAD_REQUEST, e))
        }
    }
}

pub async fn handle_v1_ast_file_symbols(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<AstReferencesPost>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let doc = match DocumentInfo::from(&PathBuf::from(&post.filename)).ok() {
        Some(doc) => doc,
        None => return Err(ScratchError::new(StatusCode::BAD_REQUEST, format!("Filename could not be parsed: {}", post.filename))),
    };

    let cx_locked = global_context.read().await;
    let search_res = match *cx_locked.ast_module.lock().await {
        Some(ref ast) => {
            ast.get_file_symbols(&doc).await
        }
        None => {
            return Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, "Ast module is not available".to_string(),
            ));
        }
    };

    match search_res {
        Ok(search_res) => {
            let json_string = serde_json::to_string_pretty(&search_res).map_err(|e| {
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("JSON serialization problem: {}", e))
            })?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(json_string))
                .unwrap())
        }
        Err(e) => {
            Err(ScratchError::new(StatusCode::BAD_REQUEST, e))
        }
    }
}
