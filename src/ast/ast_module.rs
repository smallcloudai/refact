use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;
use tracing::info;
use tree_sitter::Point;
use uuid::Uuid;

use crate::ast::ast_index::{AstIndex, RequestSymbolType};
use crate::ast::ast_index_service::{AstEvent, AstIndexService, AstEventType};
use crate::ast::structs::{AstCursorSearchResult, AstQuerySearchResult, FileASTMarkup, FileReferencesResult, SymbolsSearchResultStruct};
use crate::ast::treesitter::ast_instance_structs::read_symbol;
// use crate::files_in_jsonl::docs_in_jsonl;
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;

pub struct AstModule {
    ast_index_service: Arc<AMutex<AstIndexService>>,
    ast_index: Arc<ARwLock<AstIndex>>,
    // cmdline -- take from command line what's needed, don't store a copy
}

#[derive(Debug, Serialize)]
pub struct VecDbCaps {
    functions: Vec<String>,
}


impl AstModule {
    pub async fn ast_indexer_init(
    ) -> Result<AstModule, String> {
        let ast_index = Arc::new(ARwLock::new(AstIndex::init()));
        let ast_index_service = Arc::new(AMutex::new(AstIndexService::init(ast_index.clone())));
        let me = AstModule {
            ast_index_service,
            ast_index,
        };
        Ok(me)
    }

    pub async fn ast_start_background_tasks(&self, gcx: Arc<ARwLock<GlobalContext>>) -> Vec<JoinHandle<()>> {
        return self.ast_index_service.lock().await.ast_start_background_tasks(gcx).await;
    }

    pub async fn ast_indexer_enqueue_files(&self, documents: &Vec<Document>, force: bool) {
        let mut documents_chunked = documents.chunks(16);
        while let Some(chunk) = documents_chunked.next() {
            self.ast_index_service.lock().await.ast_indexer_enqueue_files(AstEvent::add_docs(chunk.to_vec()), force).await;
        }
    }

    pub async fn ast_add_file_no_queue(&mut self, document: &Document, make_dirty: bool) -> Result<(), String> {
        self.ast_index.write().await.add_or_update(&document, make_dirty).await
    }

    pub async fn ast_force_reindex(&mut self) {
        self.ast_index.write().await.force_reindex().await
    }

    pub async fn ast_reset_index(&self, force: bool)
    {
        self.ast_index_service.lock().await.ast_indexer_enqueue_files(
            AstEvent { docs: vec![], typ: AstEventType::AstReset, posted_ts: std::time::SystemTime::now() },
            force
        ).await;
    }

    pub async fn ast_remove_file(&mut self, path: &PathBuf) {
        // TODO: will not work if the same file is in the indexer queue
        let _ = self.ast_index.write().await.remove(&Document::new(path));
    }

    pub async fn clear_index(&mut self) {
        self.ast_index.write().await.clear_index();
    }

    pub async fn search_by_name(
        &self,
        query: String,
        request_symbol_type: RequestSymbolType,
        try_fuzzy_if_not_found: bool,
        top_n: usize,
    ) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        match self.ast_index.read().await.search_by_name(query.as_str(), request_symbol_type, None, None, try_fuzzy_if_not_found) {
            Ok(results) => {
                let symbol_structs = results
                    .iter()
                    .take(top_n)
                    .filter_map(|s| {
                        let info_struct = read_symbol(s).symbol_info_struct();
                        let content = info_struct.get_content_blocked().ok()?;
                        Some(SymbolsSearchResultStruct {
                            symbol_declaration: info_struct,
                            content: content,
                            sim_to_query: 1.0,
                        })
                    })
                    .collect::<Vec<_>>();
                for r in symbol_structs.iter() {
                    let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                    info!("def-distance {:.3}, found {last_30_chars}", r.sim_to_query);
                }
                info!("ast search_by_name time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), results.len());
                Ok(
                    AstQuerySearchResult {
                        query_text: query,
                        search_results: symbol_structs
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
        match self.ast_index.read().await.search_by_content(query.as_str(), request_symbol_type, None, None).await {
            Ok(results) => {
                let symbol_structs = results
                    .iter()
                    .take(top_n)
                    .filter_map(|s| {
                        let info_struct = read_symbol(s).symbol_info_struct();
                        let content = info_struct.get_content_blocked().ok()?;
                        Some(SymbolsSearchResultStruct {
                            symbol_declaration: info_struct,
                            content: content,
                            sim_to_query: -1.0,
                        })
                    })
                    .collect::<Vec<_>>();
                for r in symbol_structs.iter() {
                    let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                    info!("def-distance {:.3}, found {last_30_chars}", r.sim_to_query);
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
        match self.ast_index.read().await.search_related_declarations(guid) {
            Ok(results) => {
                let symbol_structs = results
                    .iter()
                    .filter_map(|s| {
                        let info_struct = read_symbol(s).symbol_info_struct();
                        let content = info_struct.get_content_blocked().ok()?;
                        Some(SymbolsSearchResultStruct {
                            symbol_declaration: info_struct,
                            content: content,
                            sim_to_query: -1.0,
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
                        search_results: symbol_structs
                    }
                )
            }
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn search_usages_by_declarations(&self, declaration_guid: &Uuid) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        match self.ast_index.read().await.search_usages_with_this_declaration(declaration_guid, None) {
            Ok(results) => {
                let symbol_structs = results
                    .iter()
                    .filter_map(|s| {
                        let info_struct = read_symbol(s).symbol_info_struct();
                        let content = info_struct.get_content_blocked().ok()?;
                        Some(SymbolsSearchResultStruct {
                            symbol_declaration: info_struct,
                            content: content,
                            sim_to_query: -1.0,
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
        &mut self,
        doc: &Document,
        code: &str,
        cursor: Point,
        top_n_near_cursor: usize,
        top_n_usage_for_each_decl: usize,
    ) -> Result<AstCursorSearchResult, String> {
        let t0 = std::time::Instant::now();
        info!("symbols_near_cursor_to_buckets started for {}", crate::nicer_logs::last_n_chars(&doc.path.to_string_lossy().to_string(), 30));
        let (cursor_usages, declarations, usages, bucket_high_overlap) = self.ast_index.read().await.symbols_near_cursor_to_buckets(
            doc,
            code,
            cursor,
            top_n_near_cursor,
            top_n_usage_for_each_decl,
        ).await;
        // for r in declarations.iter() {
        //     let last_30_chars = crate::nicer_logs::last_n_chars(&r.name, 30);
        //     info!("found {last_30_chars}");
        // }
        // for r in usages.iter() {
        //     let last_30_chars = crate::nicer_logs::last_n_chars(&r.name, 30);
        //     info!("found {last_30_chars}");
        // }
        let result = AstCursorSearchResult {
            query_text: "".to_string(),
            file_path: doc.path.clone(),
            cursor,
            cursor_symbols: cursor_usages
                .iter()
                .map(|x| {
                    let symbol_declaration = read_symbol(x).symbol_info_struct();
                    let content = symbol_declaration.get_content_blocked().unwrap_or_default();
                    SymbolsSearchResultStruct {
                        symbol_declaration,
                        content,
                        sim_to_query: -1.0,
                    }
                })
                .collect::<Vec<SymbolsSearchResultStruct>>(),
            bucket_declarations: declarations
                .iter()
                .map(|x| {
                    let symbol_declaration = read_symbol(x).symbol_info_struct();
                    let content = symbol_declaration.get_content_blocked().unwrap_or_default();
                    SymbolsSearchResultStruct {
                        symbol_declaration,
                        content,
                        sim_to_query: -1.0,
                    }
                })
                .collect::<Vec<SymbolsSearchResultStruct>>(),
            bucket_usage_of_same_stuff: usages
                .iter()
                .map(|x| {
                    let symbol_declaration = read_symbol(x).symbol_info_struct();
                    let content = symbol_declaration.get_content_blocked().unwrap_or_default();
                    SymbolsSearchResultStruct {
                        symbol_declaration,
                        content,
                        sim_to_query: -1.0,
                    }
                })
                .collect::<Vec<SymbolsSearchResultStruct>>(),
            bucket_high_overlap: bucket_high_overlap
                .iter()
                .map(|x| {
                    let symbol_declaration = read_symbol(x).symbol_info_struct();
                    let content = symbol_declaration.get_content_blocked().unwrap_or_default();
                    SymbolsSearchResultStruct {
                        symbol_declaration,
                        content,
                        sim_to_query: -1.0,
                    }
                })
                .collect::<Vec<SymbolsSearchResultStruct>>(),
        };
        info!("symbols_near_cursor_to_buckets {:.3}s, \
            found bucket_declarations {}, bucket_usage_of_same_stuff {}, bucket_high_overlap {}",
            t0.elapsed().as_secs_f32(),
            result.bucket_declarations.len(),
            result.bucket_usage_of_same_stuff.len(),
            result.bucket_high_overlap.len()
        );
        Ok(result)
    }

    pub async fn file_markup(
        &self,
        doc: &Document,
    ) -> Result<FileASTMarkup, String> {
        let t0 = std::time::Instant::now();
        match self.ast_index.read().await.file_markup(doc).await {
            Ok(markup) => {
                info!("ast file_markup {:.3}s for {}", t0.elapsed().as_secs_f32(), crate::nicer_logs::last_n_chars(&doc.path.to_string_lossy().to_string(), 30));
                Ok(markup)
            }
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn get_file_symbols(&self, request_symbol_type: RequestSymbolType, doc: &Document) -> Result<FileReferencesResult, String> {
        let symbols = match self.ast_index.read().await.get_by_file_path(request_symbol_type, &doc) {
            Ok(s) => s,
            Err(err) => { return Err(format!("Error: {}", err)); }
        };
        Ok(FileReferencesResult {
            file_path: doc.path.clone(),
            symbols,
        })
    }

    pub async fn get_symbols_names(&self, request_symbol_type: RequestSymbolType) -> Result<Vec<String>, String> {
        Ok(self.ast_index.read().await.get_symbols_names(request_symbol_type))
    }
}
