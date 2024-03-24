use std::sync::Arc;
use itertools::Itertools;

use serde::Serialize;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;
use tracing::info;
use tree_sitter::Point;

use crate::global_context::GlobalContext;
use crate::ast::ast_index::{AstIndex, RequestSymbolType};
use crate::ast::ast_index_service::AstIndexService;
use crate::ast::comments_wrapper::get_language_id_by_filename;
use crate::ast::structs::{AstCursorSearchResult, AstQuerySearchResult, CursorUsagesResult, FileASTMarkup, FileReferencesResult, SymbolsSearchResultStruct, UsageSearchResultStruct};
use crate::ast::treesitter::parsers::get_parser_by_filename;
use crate::files_in_workspace::DocumentInfo;
use url::Url;
use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstance, SymbolInformation};
use crate::ast::treesitter::structs::SymbolInfo;
use crate::files_in_jsonl::files_in_jsonl;

pub struct AstModule {
    ast_index_service: Arc<AMutex<AstIndexService>>,
    ast_index: Arc<AMutex<AstIndex>>,
    // cmdline -- take from command line what's needed, don't store a copy
}

#[derive(Debug, Serialize)]
pub struct VecDbCaps {
    functions: Vec<String>,
}


impl AstModule {
    pub async fn ast_indexer_init(
        global_context: Arc<ARwLock<GlobalContext>>,
    ) -> Result<AstModule, String> {
        let ast_index = Arc::new(AMutex::new(AstIndex::init()));
        let ast_index_service = Arc::new(AMutex::new(AstIndexService::init(ast_index.clone())));

        let documents = files_in_jsonl(global_context.clone()).await;
        let me = AstModule {
            ast_index_service,
            ast_index,
        };
        me.ast_indexer_enqueue_files(&documents, true).await;
        Ok(me)
    }

    pub async fn ast_start_background_tasks(&self) -> Vec<JoinHandle<()>> {
        return self.ast_index_service.lock().await.ast_start_background_tasks().await;
    }

    pub async fn ast_indexer_enqueue_files(&self, documents: &Vec<DocumentInfo>, force: bool) {
        self.ast_index_service.lock().await.ast_indexer_enqueue_files(documents, force).await;
    }

    pub async fn ast_add_file_no_queue(&self, document: &DocumentInfo) -> Result<(), String> {
        self.ast_index.lock().await.add_or_update(&document)
    }

    pub async fn remove_file(&self, doc: &DocumentInfo) {
        // TODO: will not work if the same file is in the indexer queue
        let _ = self.ast_index.lock().await.remove(doc);
    }

    pub async fn clear_index(&self) {
        self.ast_index.lock().await.clear_index();
    }

    pub async fn search_by_name(
        &self,
        query: String,
        request_symbol_type: RequestSymbolType
    ) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        let ast_index = self.ast_index.clone();
        let ast_index_locked = ast_index.lock().await;
        match ast_index_locked.search_by_name(query.as_str(), request_symbol_type, None, None) {
            Ok(results) => {
                for r in results.iter() {
                    let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                    info!("def-distance {:.3}, found {last_30_chars}", r.sim_to_query);
                }
                info!("ast search_by_name time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), results.len());
                Ok(
                    AstQuerySearchResult {
                        query_text: query,
                        search_results: results,
                    }
                )
            }
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn search_by_content(
        &self,
        query: String,
        request_symbol_type: RequestSymbolType
    ) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        let ast_index = self.ast_index.clone();
        let ast_index_locked = ast_index.lock().await;
        match ast_index_locked.search_by_content(query.as_str(), request_symbol_type, None, None) {
            Ok(results) => {
                for r in results.iter() {
                    let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                    info!("def-distance {:.3}, found {last_30_chars}", r.sim_to_query);
                }
                info!("ast search_by_content time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), results.len());
                Ok(
                    AstQuerySearchResult {
                        query_text: query,
                        search_results: results,
                    }
                )
            }
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn search_usages_of_declarations_by_cursor(
        &mut self,
        doc: &DocumentInfo,
        code: &str,
        cursor: Point,
        top_n: usize,
        filter_by_language: bool
    ) -> Result<AstCursorSearchResult, String> {
        unimplemented!()
    }

    pub async fn search_references_by_cursor(
        &mut self,
        doc: &DocumentInfo,
        code: &str,
        cursor: Point,
        top_n: usize,
        filter_by_language: bool,
    ) -> Result<AstCursorSearchResult, String> {
        unimplemented!()
    }

    pub async fn file_markup(
        &self,
        doc: &DocumentInfo,
    ) -> Result<FileASTMarkup, String> {
        let t0 = std::time::Instant::now();
        let ast_index = self.ast_index.clone();
        let ast_index_locked = ast_index.lock().await;
        match ast_index_locked.file_markup(doc).await {
            Ok(markup) => {
                info!("ast file_markup time {:.3}s", t0.elapsed().as_secs_f32());
                Ok(markup)
            }
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn get_file_symbols(&self, request_symbol_type: RequestSymbolType, doc: &DocumentInfo) -> Result<FileReferencesResult, String> {
        let ast_index = self.ast_index.clone();
        let ast_index_locked = ast_index.lock().await;
        let symbols = match ast_index_locked.get_by_file_path(request_symbol_type, &doc) {
            Ok(s) => s,
            Err(err) => { return Err(format!("Error: {}", err)); }
        };
        Ok(FileReferencesResult {
            file_path: doc.get_path(),
            symbols,
        })
    }

    pub async fn get_all_symbols(&self, request_symbol_type: RequestSymbolType) -> Vec<SymbolInformation> {
        let ast_index = self.ast_index.clone();
        let ast_index_locked = ast_index.lock().await;
        ast_index_locked.get_all_symbols(request_symbol_type)
    }

    pub async fn get_file_paths(&self) -> Vec<Url> {
        let ast_index = self.ast_index.clone();
        let ast_index_locked = ast_index.lock().await;
        ast_index_locked.get_file_paths()
    }
}
