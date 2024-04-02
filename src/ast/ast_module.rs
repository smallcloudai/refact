use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;
use tracing::info;
use tree_sitter::Point;

use crate::ast::ast_index::{AstIndex, RequestSymbolType};
use crate::ast::ast_index_service::{AstEvent, AstIndexService};
use crate::ast::comments_wrapper::get_language_id_by_filename;
use crate::ast::structs::{AstCursorSearchResult, AstQuerySearchResult, FileASTMarkup, FileReferencesResult, SymbolsSearchResultStruct};
use crate::ast::treesitter::structs::SymbolType;
use crate::files_in_jsonl::docs_in_jsonl;
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
        global_context: Arc<ARwLock<GlobalContext>>,
    ) -> Result<AstModule, String> {
        let ast_index = Arc::new(ARwLock::new(AstIndex::init()));
        let ast_index_service = Arc::new(AMutex::new(AstIndexService::init(ast_index.clone())));

        let documents = docs_in_jsonl(global_context.clone()).await;
        let mut docs = vec![];
        for d in documents {
            docs.push(d.read().await.clone());
        }
        let me = AstModule {
            ast_index_service,
            ast_index,
        };
        me.ast_indexer_enqueue_files(&docs, true).await;
        Ok(me)
    }

    pub async fn ast_start_background_tasks(&self) -> Vec<JoinHandle<()>> {
        return self.ast_index_service.lock().await.ast_start_background_tasks().await;
    }

    pub async fn ast_indexer_enqueue_files(&self, documents: &Vec<Document>, force: bool) {
        self.ast_index_service.lock().await.ast_indexer_enqueue_files(AstEvent::add_docs(documents.clone()), force).await;
    }

    pub async fn ast_add_file_no_queue(&mut self, document: &Document) -> Result<(), String> {
        self.ast_index.write().await.add_or_update(&document).await
    }

    pub async fn ast_force_reindex(&mut self) {
        self.ast_index.write().await.force_reindex().await
    }

    pub async fn ast_reset_index(&self) {
        self.ast_index_service.lock().await.ast_indexer_enqueue_files(AstEvent::reset(), false).await;
    }

    pub async fn remove_file(&mut self, path: &PathBuf) {
        // TODO: will not work if the same file is in the indexer queue
        let _ = self.ast_index.write().await.remove(&Document::new(path, None));
    }

    pub async fn clear_index(&mut self) {
        self.ast_index.write().await.clear_index();
    }

    pub async fn search_by_name(
        &self,
        query: String,
        request_symbol_type: RequestSymbolType,
    ) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        match self.ast_index.read().await.search_by_name(query.as_str(), request_symbol_type, None, None) {
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
        request_symbol_type: RequestSymbolType,
    ) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        match self.ast_index.read().await.search_by_content(query.as_str(), request_symbol_type, None, None).await {
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

    pub async fn search_related_declarations(&self, guid: &str) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        match self.ast_index.read().await.search_related_declarations(guid) {
            Ok(results) => {
                for r in results.iter() {
                    let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                    info!("found {last_30_chars}");
                }
                info!("ast search_by_name time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), results.len());
                Ok(
                    AstQuerySearchResult {
                        query_text: guid.to_string(),
                        search_results: results,
                    }
                )
            }
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn search_usages_by_declarations(&self, declaration_guid: &str) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        match self.ast_index.read().await.search_symbols_by_declarations_usage(declaration_guid, None) {
            Ok(results) => {
                for r in results.iter() {
                    let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                    info!("found {last_30_chars}");
                }
                info!("ast search_by_name time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), results.len());
                Ok(
                    AstQuerySearchResult {
                        query_text: declaration_guid.to_string(),
                        search_results: results,
                    }
                )
            }
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn retrieve_cursor_symbols_by_declarations(
        &mut self,
        doc: &Document,
        code: &str,
        cursor: Point,
        top_n_near_cursor: usize,
        top_n_usage_for_each_decl: usize,
    ) -> Result<AstCursorSearchResult, String> {
        let t0 = std::time::Instant::now();
        let (cursor_usages, declarations, usages) = self.ast_index.read().await.retrieve_cursor_symbols_by_declarations(
            doc,
            code,
            cursor,
            top_n_near_cursor,
            top_n_usage_for_each_decl
        ).await;
        for r in declarations.iter() {
            let last_30_chars = crate::nicer_logs::last_n_chars(&r.name, 30);
            info!("found {last_30_chars}");
        }
        for r in usages.iter() {
            let last_30_chars = crate::nicer_logs::last_n_chars(&r.name, 30);
            info!("found {last_30_chars}");
        }
        let language = get_language_id_by_filename(&doc.path);
        let matched_by_name_symbols = {
            let ast_index_locked = self.ast_index.read().await;
            cursor_usages
                .iter()
                .take(top_n_near_cursor)
                .map(|s| {
                    ast_index_locked
                        .search_by_name(&s.name, RequestSymbolType::Declaration, Some(doc.clone()), language.clone())
                        .unwrap_or_else(|_| vec![])
                })
                .flatten()
                .filter(|s| {
                    s.symbol_declaration.symbol_type == SymbolType::StructDeclaration
                        || s.symbol_declaration.symbol_type == SymbolType::TypeAlias
                        || s.symbol_declaration.symbol_type == SymbolType::FunctionDeclaration
                })
                .collect::<Vec<_>>()
        };
        info!("ast retrieve_cursor_symbols_by_declarations time {:.3}s, \
            found {} declarations, {} declaration usages, {} by name",
            t0.elapsed().as_secs_f32(), declarations.len(), usages.len(), matched_by_name_symbols.len());
        Ok(
            AstCursorSearchResult {
                query_text: "".to_string(),
                file_path: doc.path.clone(),
                cursor,
                cursor_symbols: cursor_usages
                    .iter()
                    .map(|x| SymbolsSearchResultStruct {
                        symbol_declaration: x.clone(),
                        content: x.get_content_blocked().unwrap_or_default(),
                        sim_to_query: -1.0,
                    })
                    .collect::<Vec<SymbolsSearchResultStruct>>(),
                declaration_symbols: declarations
                    .iter()
                    .map(|x| SymbolsSearchResultStruct {
                        symbol_declaration: x.clone(),
                        content: x.get_content_blocked().unwrap_or_default(),
                        sim_to_query: -1.0,
                    })
                    .collect::<Vec<SymbolsSearchResultStruct>>(),
                declaration_usage_symbols: usages
                    .iter()
                    .map(|x| SymbolsSearchResultStruct {
                        symbol_declaration: x.clone(),
                        content: x.get_content_blocked().unwrap_or_default(),
                        sim_to_query: -1.0,
                    })
                    .collect::<Vec<SymbolsSearchResultStruct>>(),
                matched_by_name_symbols: matched_by_name_symbols
            }
        )
    }

    pub async fn file_markup(
        &self,
        doc: &Document,
    ) -> Result<FileASTMarkup, String> {
        let t0 = std::time::Instant::now();
        match self.ast_index.read().await.file_markup(doc).await {
            Ok(markup) => {
                info!("ast file_markup time {:.3}s", t0.elapsed().as_secs_f32());
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
