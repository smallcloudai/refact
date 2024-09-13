use std::collections::HashSet;
use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};
use url::Url;
use serde_json::json;
use uuid::Uuid;

use crate::custom_error::ScratchError;
use crate::files_in_workspace::{Document, get_file_text_from_memory_or_disk};
use crate::global_context::SharedGlobalContext;
use crate::postprocessing::pp_context_files::pp_color_lines;
use crate::postprocessing::pp_utils::{context_msgs_from_paths, pp_ast_markup_files};
use crate::call_validation::PostprocessSettings;


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
    let files_markup = pp_ast_markup_files(global_context.clone(), &messages).await;
    let mut settings = PostprocessSettings::new();
    settings.close_small_gaps = false;
    let lines_in_files = pp_color_lines(
            global_context.clone(),
            &vec![],
            files_markup,
            &settings,
        ).await;
    let mut result = "".to_string();
    for linevec in lines_in_files.values() {
        for lineref in linevec {
            result.push_str(format!("{}:{:04} {:<43} {:>7.3} {}\n",
                crate::nicer_logs::last_n_chars(&lineref.file_ref.cpath.to_string_lossy().to_string(), 30),
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

    let corrected = crate::files_correction::correct_to_nearest_filename(
        global_context.clone(),
        &post.file_url.to_file_path().unwrap_or_default().to_string_lossy().to_string(),
        false,
        1,
    ).await;

    if corrected.len() == 0 {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(serde_json::to_string_pretty(&json!({"detail": "File not found"})).unwrap()))
            .unwrap());
    }

    let cpath = corrected[0].clone();
    let mut doc = Document::new(&cpath.into());
    let file_text = get_file_text_from_memory_or_disk(global_context.clone(), &doc.doc_path).await.map_err(|e|
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e)
    )?;
    doc.update_text(&file_text);

    let ast_service_opt = global_context.read().await.ast_service.clone();
    let search_res = match &ast_service_opt {
        Some(ast_service) => {
            let ast_index = ast_service.lock().await.ast_index.clone();
            crate::ast::alt_db::doc_symbols(ast_index.clone(), &doc.doc_path.to_string_lossy().to_string()).await
        }
        None => {
            return Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, "Ast module is not available".to_string(),
            ));
        }
    };
    let json_string = serde_json::to_string_pretty(&search_res).map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("JSON serialization problem: {}", e))
    })?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(json_string))
        .unwrap())
}

pub async fn handle_v1_ast_status(
    Extension(global_context): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let ast_service_opt = global_context.read().await.ast_service.clone();
    match &ast_service_opt {
        Some(ast_service) => {
            let alt_status: std::sync::Arc<tokio::sync::Mutex<crate::ast::alt_minimalistic::AstStatus>> = ast_service.lock().await.alt_status.clone();
            let json_string = serde_json::to_string_pretty(&*alt_status.lock().await).map_err(|e| {
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("JSON serialization problem: {}", e))
            })?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(json_string))
                .unwrap())
        }
        None => {
            Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, "ast module is turned off".to_string(),
            ))
        }
    }
}
