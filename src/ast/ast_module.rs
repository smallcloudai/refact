use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use strsim::jaro_winkler;
use tokio::sync::{Mutex as AMutex, MutexGuard};
use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;
use tokio::time::error::Elapsed;
use tokio::time::timeout;
use tracing::info;
use tree_sitter::Point;
use uuid::Uuid;

use crate::ast::ast_index::{AstIndex, RequestSymbolType};
use crate::ast::ast_index_service::{AstEvent, AstEventType, AstIndexService};
use crate::ast::structs::{AstCursorSearchResult, AstQuerySearchResult, FileASTMarkup, FileReferencesResult, SymbolsSearchResultStruct};
use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstanceRc};
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstIndexStatus {
    pub files_unparsed: usize,
    pub files_total: usize,
    pub ast_index_files_total: usize,
    pub ast_index_symbols_total: usize,
    pub state: String,
}


pub struct AstModule {
    ast_index_service: Arc<AMutex<AstIndexService>>,
    ast_index: Arc<AMutex<AstIndex>>,
    status: Arc<AMutex<AstIndexStatus>>
}

#[derive(Debug, Serialize)]
pub struct VecDbCaps {
    functions: Vec<String>,
}

impl AstModule {
    pub async fn ast_indexer_init(
        ast_index_max_files: usize,
        shutdown_flag: Arc<AtomicBool>
    ) -> Result<AstModule, String> {
        let status = Arc::new(AMutex::new(AstIndexStatus {
            files_unparsed: 0,
            files_total: 0,
            ast_index_files_total: 0,
            ast_index_symbols_total: 0,
            state: "starting".to_string(),
        }));
        let ast_index = Arc::new(AMutex::new(AstIndex::init(
            ast_index_max_files, shutdown_flag
        )));
        let ast_index_service = Arc::new(AMutex::new(AstIndexService::init(
            ast_index.clone(),
            status.clone()
        )));
        let me = AstModule {
            ast_index_service,
            ast_index,
            status
        };
        Ok(me)
    }

    pub async fn ast_start_background_tasks(
        &self, gcx: Arc<ARwLock<GlobalContext>>
    ) -> Vec<JoinHandle<()>> {
        return self.ast_index_service.lock().await.ast_start_background_tasks(gcx).await;
    }

    pub async fn ast_indexer_enqueue_files(&self, documents: &Vec<Document>, force: bool) {
        let mut documents_chunked = documents.chunks(16);
        while let Some(chunk) = documents_chunked.next() {
            self.ast_index_service.lock().await.ast_indexer_enqueue_files(AstEvent::add_docs(chunk.to_vec()), force).await;
        }
    }

    pub async fn ast_add_file_no_queue(&mut self, document: &Document, make_dirty: bool) -> Result<usize, String> {
        let mut ast_ref = match self.write_ast(Duration::from_secs(3)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        ast_ref.add_or_update(&document, make_dirty)
    }

    pub async fn ast_force_reindex(&mut self) -> Result<(), String> {
        let mut ast_ref = match self.write_ast(Duration::from_secs(3)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        ast_ref.reindex();
        Ok(())
    }

    pub async fn ast_reset_index(&self, force: bool) {
        self.ast_index_service.lock().await.ast_indexer_enqueue_files(
            AstEvent { docs: vec![], typ: AstEventType::AstReset, posted_ts: std::time::SystemTime::now() },
            force,
        ).await;
    }

    pub async fn ast_remove_file(&mut self, path: &PathBuf) -> Result<(), String> {
        // TODO: will not work if the same file is in the indexer queue
        let mut ast_ref = match self.write_ast(Duration::from_secs(3)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        let _ = ast_ref.remove(&Document::new(path));
        Ok(())
    }

    pub async fn clear_index(&mut self) -> Result<(), String> {
        let mut ast_ref = match self.write_ast(Duration::from_secs(3)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        ast_ref.clear_index();
        Ok(())
    }

    async fn read_ast(&self, duration: Duration) -> Result<MutexGuard<'_, AstIndex>, Elapsed> {
        timeout(duration, self.ast_index.lock()).await
    }

    async fn write_ast(&self, duration: Duration) -> Result<MutexGuard<'_, AstIndex>, Elapsed> {
        timeout(duration, self.ast_index.lock()).await
    }

    pub async fn search_by_name(
        &self,
        query: String,
        request_symbol_type: RequestSymbolType,
        try_fuzzy_if_not_found: bool,
        top_n: usize,
    ) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        let ast_ref = match self.read_ast(Duration::from_millis(25)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        match ast_ref.search_by_name(query.as_str(), request_symbol_type, None, None, try_fuzzy_if_not_found, true) {
            Ok(results) => {
                let symbol_structs = results
                    .iter()
                    .take(top_n)
                    .filter_map(|s| {
                        let info_struct = s.borrow().symbol_info_struct();
                        let name = info_struct.name.clone();
                        let content = info_struct.get_content_blocked().ok()?;
                        Some(SymbolsSearchResultStruct {
                            symbol_declaration: info_struct,
                            content: content,
                            usefulness: jaro_winkler(&query, &name) as f32 * 100.0,
                        })
                    })
                    .collect::<Vec<_>>();
                for r in symbol_structs.iter() {
                    let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                    info!("def-distance {:.3}, found {last_30_chars}", r.usefulness);
                }
                info!("ast search_by_name time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), results.len());
                Ok(
                    AstQuerySearchResult {
                        query_text: query,
                        search_results: symbol_structs,
                    }
                )
            }
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn search_by_content(
        &self,
        query: String,
        request_symbol_type: RequestSymbolType,
        top_n: usize,
    ) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        let ast_ref = match self.read_ast(Duration::from_millis(25)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        match ast_ref.search_by_content(query.as_str(), request_symbol_type, None, None) {
            Ok(results) => {
                let symbol_structs = results
                    .iter()
                    .take(top_n)
                    .filter_map(|s| {
                        let info_struct = s.borrow().symbol_info_struct();
                        let content = info_struct.get_content_blocked().ok()?;
                        Some(SymbolsSearchResultStruct {
                            symbol_declaration: info_struct,
                            content: content,
                            usefulness: 100.0,
                        })
                    })
                    .collect::<Vec<_>>();
                for r in symbol_structs.iter() {
                    let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                    info!("def-distance {:.3}, found {last_30_chars}", r.usefulness);
                }
                info!("ast search_by_content time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), results.len());
                Ok(
                    AstQuerySearchResult {
                        query_text: query,
                        search_results: symbol_structs,
                    }
                )
            }
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn search_related_declarations(&self, guid: &Uuid) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        let ast_ref = match self.read_ast(Duration::from_millis(25)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        match ast_ref.search_related_declarations(guid) {
            Ok(results) => {
                let symbol_structs = results
                    .iter()
                    .filter_map(|s| {
                        let info_struct = s.borrow().symbol_info_struct();
                        let content = info_struct.get_content_blocked().ok()?;
                        Some(SymbolsSearchResultStruct {
                            symbol_declaration: info_struct,
                            content: content,
                            usefulness: 100.0,
                        })
                    })
                    .collect::<Vec<_>>();
                for r in symbol_structs.iter() {
                    let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                    info!("found {last_30_chars}");
                }
                info!("ast search_by_name time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), results.len());
                Ok(
                    AstQuerySearchResult {
                        query_text: guid.to_string(),
                        search_results: symbol_structs,
                    }
                )
            }
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn search_usages_by_declarations(&self, declaration_guid: &Uuid) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        let ast_ref = match self.read_ast(Duration::from_millis(25)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        match ast_ref.search_usages_with_this_declaration(declaration_guid, None) {
            Ok(results) => {
                let symbol_structs = results
                    .iter()
                    .filter_map(|s| {
                        let info_struct = s.borrow().symbol_info_struct();
                        let content = info_struct.get_content_blocked().ok()?;
                        Some(SymbolsSearchResultStruct {
                            symbol_declaration: info_struct,
                            content: content,
                            usefulness: 100.0,
                        })
                    })
                    .collect::<Vec<_>>();
                for r in symbol_structs.iter() {
                    let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                    info!("found {last_30_chars}");
                }
                info!("ast search_by_name time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), results.len());
                Ok(
                    AstQuerySearchResult {
                        query_text: declaration_guid.to_string(),
                        search_results: symbol_structs,
                    }
                )
            }
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn symbols_near_cursor_to_buckets(
        &self,
        doc: &Document,
        code: &str,
        cursor: Point,
        top_n_near_cursor: usize,
        top_n_usage_for_each_decl: usize,
    ) -> Result<AstCursorSearchResult, String> {
        let t0 = std::time::Instant::now();
        let ast_ref = match self.read_ast(Duration::from_millis(25)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };

        info!("to_buckets {}", crate::nicer_logs::last_n_chars(&doc.path.to_string_lossy().to_string(), 30));
        let (cursor_usages, declarations, usages, bucket_high_overlap, bucket_imports, guid_to_usefulness) =
            ast_ref.symbols_near_cursor_to_buckets(
                &doc,
                code,
                cursor,
                top_n_near_cursor,
                top_n_usage_for_each_decl,
                3,
            );
        let symbol_to_search_res = |x: &AstSymbolInstanceRc| {
            let symbol_declaration = x.borrow().symbol_info_struct();
            let content = symbol_declaration.get_content_blocked().unwrap_or_default();
            let usefulness = *guid_to_usefulness
                .get(&symbol_declaration.guid)
                .expect("Guid has not found in `guid_to_usefulness` dict, \
                        something is wrong with the `symbols_near_cursor_to_buckets`");
            SymbolsSearchResultStruct {
                symbol_declaration,
                content,
                usefulness,
            }
        };

        let result = AstCursorSearchResult {
            query_text: "".to_string(),
            file_path: doc.path.clone(),
            cursor,
            cursor_symbols: cursor_usages
                .iter()
                .map(symbol_to_search_res)
                .collect::<Vec<SymbolsSearchResultStruct>>(),
            bucket_declarations: declarations
                .iter()
                .map(symbol_to_search_res)
                .collect::<Vec<SymbolsSearchResultStruct>>(),
            bucket_usage_of_same_stuff: usages
                .iter()
                .map(symbol_to_search_res)
                .collect::<Vec<SymbolsSearchResultStruct>>(),
            bucket_high_overlap: bucket_high_overlap
                .iter()
                .map(symbol_to_search_res)
                .collect::<Vec<SymbolsSearchResultStruct>>(),
            bucket_imports: bucket_imports
                .iter()
                .map(symbol_to_search_res)
                .collect::<Vec<SymbolsSearchResultStruct>>(),
        };
        info!("to_buckets {:.3}s => bucket_declarations \
            {} bucket_usage_of_same_stuff {} bucket_high_overlap {} bucket_imports {}",
            t0.elapsed().as_secs_f32(),
            result.bucket_declarations.len(),
            result.bucket_usage_of_same_stuff.len(),
            result.bucket_high_overlap.len(),
            result.bucket_imports.len()
        );
        Ok(result)
    }

    pub async fn file_markup(
        &self,
        doc: &Document,
    ) -> Result<FileASTMarkup, String> {
        let ast_ref = match self.read_ast(Duration::from_millis(25)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        match ast_ref.file_markup(doc) {
            Ok(markup) => {
                Ok(markup)
            }
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn get_file_symbols(&self, request_symbol_type: RequestSymbolType, doc: &Document) -> Result<FileReferencesResult, String> {
        let ast_ref = match self.read_ast(Duration::from_millis(25)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        let symbols = match ast_ref.get_by_file_path(request_symbol_type, &doc) {
            Ok(s) => s,
            Err(err) => { return Err(format!("Error: {}", err)); }
        };
        Ok(FileReferencesResult {
            file_path: doc.path.clone(),
            symbols,
        })
    }

    pub async fn get_symbols_names(&self, request_symbol_type: RequestSymbolType) -> Result<Vec<String>, String> {
        let ast_ref = match self.read_ast(Duration::from_millis(25)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        Ok(ast_ref.get_symbols_names(request_symbol_type))
    }

    pub async fn ast_index_status(&self) -> AstIndexStatus {
        let mut locked_status = self.status.lock().await;
        match self.read_ast(Duration::from_millis(25)).await {
            Ok(ast) => {
                locked_status.ast_index_files_total = ast.total_files();
                locked_status.ast_index_symbols_total = ast.total_symbols();
            },
            Err(_) => {}
        };
        locked_status.clone()
    }
}
