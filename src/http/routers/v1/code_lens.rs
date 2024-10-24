use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
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

#[derive(Serialize, Clone)]
struct CodeLensResponse {
    success: u8,
    code_lens: Vec<CodeLensOutput>,
}

#[derive(Serialize, Clone)]
struct CodeLensOutput {
    spath: String,
    line1: usize,
    line2: usize,
}

struct CodeLensCacheEntry {
    response: CodeLensResponse,
    ts: f64,
}

#[derive(Default)]
pub struct CodeLensCache {
    store: HashMap<String, CodeLensCacheEntry>,
}

impl CodeLensCache {
    pub fn clean_up_old_entries(&mut self, now: f64) {
        self.store.retain(|_, entry| now - entry.ts <= 600.0);
    }
}

pub async fn handle_v1_code_lens(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<CodeLensPost>(&body_bytes).map_err(|e| {
        tracing::info!("chat handler cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let codelens_cache = global_context.read().await.codelens_cache.clone();

    let cpath = crate::files_correction::canonical_path(&post.uri.to_file_path().unwrap_or_default().to_string_lossy().to_string());
    let cpath_str = cpath.to_string_lossy().to_string();

    let ast_service_opt = global_context.read().await.ast_service.clone();
    let defs: Vec<Arc<AstDefinition>> = if let Some(ast_service) = ast_service_opt {
        let indexing_finished = crate::ast::ast_indexer_thread::ast_indexer_block_until_finished(ast_service.clone(), 300, true).await;
        let ast_index = ast_service.lock().await.ast_index.clone();
        let defs = crate::ast::ast_db::doc_defs(ast_index, &cpath_str).await;
        if !indexing_finished || defs.len() <= 1 {
            tracing::info!("indexing_finished={} defs.len()=={}", indexing_finished, defs.len());
            if let Some(cache_entry) = codelens_cache.lock().await.store.get(&cpath_str) {
                tracing::info!("therefore return cached {} records", cache_entry.response.code_lens.len());
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from(serde_json::to_string(&cache_entry.response).unwrap()))
                    .unwrap());
            }
        }
        defs
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

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
    codelens_cache.lock().await.store.insert(cpath_str.clone(), CodeLensCacheEntry { response: response.clone(), ts: now });
    codelens_cache.lock().await.clean_up_old_entries(now);
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(serde_json::to_string(&response).unwrap()))
        .unwrap())
}
