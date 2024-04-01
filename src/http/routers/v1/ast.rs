use std::collections::HashSet;
use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::ast::ast_index::RequestSymbolType;
use crate::custom_error::ScratchError;
use crate::files_in_workspace::DocumentInfo;
use crate::global_context::SharedGlobalContext;

#[derive(Serialize, Deserialize, Clone)]
struct AstQuerySearchBy {
    query: String,
    is_declaration: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct AstQuerySearchByGuid {
    guid: String,
}


#[derive(Serialize, Deserialize, Clone)]
struct AstFileUrlPost {
    file_url: Url,
}

#[derive(Serialize, Deserialize, Clone)]
struct FileNameOnlyPost {
    file_name: String,
}


pub async fn handle_v1_ast_search_by_name(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<AstQuerySearchBy>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let ast_module = global_context.read().await.ast_module.clone();
    let search_res = match &ast_module {
        Some(ast) => {
            let symbol_type = if post.is_declaration {
                RequestSymbolType::Declaration
            } else {
                RequestSymbolType::Usage
            };
            ast.read().await.search_by_name(post.query, symbol_type).await
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

pub async fn handle_v1_ast_search_by_content(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<AstQuerySearchBy>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let ast_module = global_context.read().await.ast_module.clone();
    let search_res = match &ast_module {
        Some(ast) => {
            let symbol_type = if post.is_declaration {
                RequestSymbolType::Declaration
            } else {
                RequestSymbolType::Usage
            };
            ast.read().await.search_by_content(post.query, symbol_type).await
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

pub async fn handle_v1_ast_search_related_declarations(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<AstQuerySearchByGuid>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let ast_module = global_context.read().await.ast_module.clone();
    let search_res = match &ast_module {
        Some(ast) => {
            ast.read().await.search_related_declarations(&post.guid).await
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

pub async fn handle_v1_ast_search_usages_by_declarations(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<AstQuerySearchByGuid>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let ast_module = global_context.read().await.ast_module.clone();
    let search_res = match &ast_module {
        Some(ast) => {
            ast.read().await.search_usages_by_declarations(&post.guid).await
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

pub async fn handle_v1_ast_file_markup(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<AstFileUrlPost>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let search_res = {
        let ast_module = global_context.read().await.ast_module.clone();
        let x = match &ast_module {
            Some(ast) => {
                let doc = DocumentInfo { uri: post.file_url, document: None };
                ast.read().await.file_markup(&doc).await
            }
            None => {
                return Err(ScratchError::new(
                    StatusCode::INTERNAL_SERVER_ERROR, "Ast module is not available".to_string(),
                ));
            }
        };
        x
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


pub async fn handle_v1_ast_file_dump(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<FileNameOnlyPost>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let mut files_set: HashSet<String> = HashSet::new();
    files_set.insert(post.file_name);
    let (lines_in_files, _) = crate::scratchpads::chat_utils_rag::postprocess_rag_stage1(global_context, vec![], files_set).await;
    let mut result = "".to_string();
    for linevec in lines_in_files.values() {
        for lineref in linevec {
            result.push_str(format!("{}:{:04} {:<43} {:>7.3} {}\n",
                crate::nicer_logs::last_n_chars(&lineref.fref.file_name, 40),
                lineref.line_n,
                crate::nicer_logs::first_n_chars(&lineref.line_content, 40),
                lineref.useful,
                lineref.color,
            ).as_str());
        }
    }
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(result))
        .unwrap())
}

pub async fn handle_v1_ast_file_symbols(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<AstFileUrlPost>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let doc = DocumentInfo { uri: post.file_url, document: None };
    let ast_module = global_context.read().await.ast_module.clone();
    let search_res = match &ast_module {
        Some(ast) => {
            ast.read().await.get_file_symbols(RequestSymbolType::All, &doc).await
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

pub async fn handle_v1_ast_index_file(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<AstFileUrlPost>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let doc = DocumentInfo { uri: post.file_url, document: None };
    let ast_module = global_context.read().await.ast_module.clone();
    let add_res = match &ast_module {
        Some(ast) => {
            ast.write().await.ast_add_file_no_queue(&doc).await
        }
        None => {
            return Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, "Ast module is not available".to_string(),
            ));
        }
    };

    match add_res {
        Ok(()) => {
            Ok(Response::builder().status(StatusCode::OK)
                .body(Body::from("{}"))
                .unwrap())
        }
        Err(e) => {
            Err(ScratchError::new(StatusCode::BAD_REQUEST, e))
        }
    }
}

pub async fn handle_v1_ast_clear_index(
    Extension(global_context): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let ast_module = global_context.read().await.ast_module.clone();
    let x = match &ast_module {
        Some(ast) => {
            ast.write().await.clear_index().await;
            Ok(Response::builder().status(StatusCode::OK)
                .body(Body::from("{}"))
                .unwrap())
        }
        None => {
            return Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, "Ast module is not available".to_string(),
            ));
        }
    };
    x
}
