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
use crate::ast::treesitter::structs::SymbolType;


#[derive(Deserialize)]
pub struct CodeLensPost {
    pub uri: Url,
    #[serde(default)]
    pub debug: bool,
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
    debug_string: Option<String>,
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
        let defs = crate::ast::ast_db::doc_defs(ast_index, &cpath_str);
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
        if let Some(last) = def.official_path.last() {
            if last == "root" {
                continue;
            }
        }
        if !post.debug {
            let line1 = def.full_line1();
            let line2 = def.full_line2();
            if line2 > line1 {
                output.push(CodeLensOutput {
                    spath: def.path_drop0(),
                    line1,
                    line2,
                    debug_string: None,
                });
            }
        } else {
            let line1 = def.full_line1();
            let line2 = def.full_line2();
            let mut entity_char = 'D';
            if def.symbol_type == SymbolType::VariableDefinition {
                entity_char = '📦';
            } else if def.symbol_type == SymbolType::StructDeclaration {
                entity_char = '📂';
            } else if def.symbol_type == SymbolType::FunctionDeclaration {
                entity_char = '⭐';
            }
            output.push(CodeLensOutput {
                spath: "".to_string(),
                line1,
                line2,
                debug_string: Some(format!("{entity_char}({})", def.path_drop0()))
            });
            for u in def.usages.iter() {
                let resolved = u.resolved_as.rsplit("::").take(2).collect::<Vec<&str>>().iter().rev().cloned().collect::<Vec<&str>>().join("::");
                let txt = if resolved != "" {
                    format!("↗{}", resolved)
                } else {
                    format!("❌{}", u.targets_for_guesswork.get(0).unwrap_or(&"".to_string()))
                };
                output.push(CodeLensOutput {
                    spath: "".to_string(),
                    line1: u.uline + 1,
                    line2: u.uline + 1,
                    debug_string: Some(txt)
                });
            }
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
