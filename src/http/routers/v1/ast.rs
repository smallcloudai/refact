use std::collections::HashSet;
use std::path::PathBuf;
use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};
use url::Url;
use serde_json::json;
use uuid::Uuid;

use crate::ast::ast_index::RequestSymbolType;
use crate::custom_error::ScratchError;
use crate::files_in_workspace::{Document, get_file_text_from_memory_or_disk};
use crate::global_context::SharedGlobalContext;
use crate::scratchpads::chat_utils_rag::{context_msgs_from_paths, postprocess_rag_load_ast_markup};

#[derive(Serialize, Deserialize, Clone)]
struct AstQuerySearchBy {
    query: String,
    is_declaration: bool,
    use_fuzzy_search: bool,
    top_n: usize
}

#[derive(Serialize, Deserialize, Clone)]
struct AstQuerySearchByGuid {
    guid: Uuid,
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
            ast.read().await.search_by_name(post.query, symbol_type, post.use_fuzzy_search, post.top_n).await
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
            ast.read().await.search_by_content(post.query, symbol_type, post.top_n).await
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
    let post = serde_json::from_slice::<FileNameOnlyPost>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let corrected = crate::files_correction::correct_to_nearest_filename(
        global_context.clone(),
        &post.file_name,
        false,
        1,
    ).await;
    if corrected.len() == 0 {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(serde_json::to_string_pretty(&json!({"detail": "File not found"})).unwrap()))
            .unwrap());
    }

    let search_res = {
        let ast_module = global_context.read().await.ast_module.clone();
        let x = match &ast_module {
            Some(ast) => {
                // corrected is already canonical path, so skip it here
                let mut doc = Document::new(&PathBuf::from(&corrected[0]));
                let text = get_file_text_from_memory_or_disk(global_context.clone(), &doc.path).await.unwrap_or_default(); // FIXME unwrap
                doc.update_text(&text);

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
    let corrected = crate::files_correction::correct_to_nearest_filename(
            global_context.clone(),
            &post.file_name,
            false,
            1,
        ).await;
    if corrected.len() == 0 {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(serde_json::to_string_pretty(&json!({"detail": "File not found"})).unwrap()))
            .unwrap());
    }
    let mut files_set: HashSet<String> = HashSet::new();
    files_set.insert(corrected[0].clone());
    let messages = context_msgs_from_paths(global_context.clone(), files_set).await;
    let files_markup = postprocess_rag_load_ast_markup(global_context.clone(), &messages).await;
    let mut settings = crate::scratchpads::chat_utils_rag::PostprocessSettings::new();
    settings.close_small_gaps = false;
    let (lines_in_files, _) = crate::scratchpads::chat_utils_rag::postprocess_rag_stage_3_6(
            global_context.clone(),
            vec![],
            &files_markup,
            &settings,
        ).await;
    let mut result = "".to_string();
    for linevec in lines_in_files.values() {
        for lineref in linevec {
            result.push_str(format!("{}:{:04} {:<43} {:>7.3} {}\n",
                crate::nicer_logs::last_n_chars(&lineref.fref.cpath.to_string_lossy().to_string(), 30),
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
    let cpath = crate::files_correction::canonical_path(&post.file_url.to_file_path().unwrap_or_default().to_string_lossy().to_string());
    let mut doc = Document::new(&cpath);
    let file_text = get_file_text_from_memory_or_disk(global_context.clone(), &cpath).await.unwrap_or_default(); // FIXME unwrap
    doc.update_text(&file_text);

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
    let cpath = crate::files_correction::canonical_path(&post.file_url.to_file_path().unwrap_or_default().to_string_lossy().to_string());
    let mut doc = Document::new(&cpath);
    let text = get_file_text_from_memory_or_disk(global_context.clone(), &doc.path).await.unwrap_or_default(); // FIXME unwrap
    doc.update_text(&text);

    let ast_module = global_context.read().await.ast_module.clone();
    let add_res = match &ast_module {
        Some(ast) => {
            ast.write().await.ast_add_file_no_queue(&doc, false).await
        }
        None => {
            return Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, "Ast module is not available".to_string(),
            ));
        }
    };

    match add_res {
        Ok(_) => {
            Ok(Response::builder().status(StatusCode::OK)
                .body(Body::from("{}"))
                .unwrap())
        }
        Err(e) => {
            Err(ScratchError::new(StatusCode::BAD_REQUEST, e))
        }
    }
}

pub async fn handle_v1_ast_force_reindex(
    Extension(global_context): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let ast_module = global_context.read().await.ast_module.clone();
    match &ast_module {
        Some(ast) => {
            match ast.write().await.ast_force_reindex().await {
                Ok(_) => {
                    Ok(Response::builder().status(StatusCode::OK)
                        .body(Body::from("{}"))
                        .unwrap())
                }
                Err(err) => {
                    Err(ScratchError::new(
                        StatusCode::INTERNAL_SERVER_ERROR, err,
                    ))
                }
            }
        }
        None => {
            Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, "Ast module is not available".to_string(),
            ))
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
            match ast.write().await.clear_index().await {
                Ok(_) => {
                    Ok(Response::builder().status(StatusCode::OK)
                        .body(Body::from("{}"))
                        .unwrap())
                }
                Err(err) => {
                    return Err(ScratchError::new(
                        StatusCode::INTERNAL_SERVER_ERROR, err,
                    ));
                }
            }
        }
        None => {
            return Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, "Ast module is not available".to_string(),
            ));
        }
    };
    x
}


pub async fn handle_v1_ast_status(
    Extension(global_context): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let ast_module = global_context.read().await.ast_module.clone();
    match &ast_module {
        Some(ast) => {
            let status = ast.write().await.ast_index_status().await;
            let json_string = serde_json::to_string_pretty(&status).map_err(|e| {
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("JSON serialization problem: {}", e))
            })?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(json_string))
                .unwrap())
        }
        None => {
            Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, "Ast module is not available".to_string(),
            ))
        }
    }
}
