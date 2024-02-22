use std::path::PathBuf;
use std::sync::Arc;
use itertools::Itertools;

use serde::Serialize;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;
use tracing::{info, error};
use tree_sitter::Point;

use crate::global_context::GlobalContext;
use crate::ast::ast_index::AstIndex;
use crate::ast::ast_index_service::AstIndexService;
use crate::ast::structs::{AstCursorSearchResult, AstQuerySearchResult, CursorUsagesResult, FileReferencesResult, SymbolsSearchResultStruct, UsageSearchResultStruct};
use crate::ast::treesitter::parsers::get_parser_by_filename;
use crate::files_in_workspace::DocumentInfo;


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

        let files_jsonl_path = global_context.read().await.cmdline.files_jsonl_path.clone();
        let documents = match crate::files_in_jsonl::parse_jsonl(&files_jsonl_path).await {
            Ok(lst) => lst,
            Err(err) => {
                error!("failed to parse {}: {}", files_jsonl_path, err);
                vec![]
            }
        };
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
        self.ast_index.lock().await.add_or_update(&document).await
    }

    pub async fn remove_file(&self, doc: &DocumentInfo) {
        // TODO: will not work if the same file is in the indexer queue
        let _ = self.ast_index.lock().await.remove(doc).await;
    }

    pub async fn clear_index(&self) {
        self.ast_index.lock().await.clear_index().await;
    }

    pub async fn search_declarations_by_cursor(
        &mut self,
        doc: &DocumentInfo,
        code: &str,
        cursor: Point,
        top_n: usize
    ) -> Result<AstCursorSearchResult, String> {
        let t0 = std::time::Instant::now();

        let path = doc.get_path();
        let usage_result = match self.parse_near_cursor(doc, code, cursor, top_n).await {
            Ok(usages) => usages,
            Err(e) => {
                return Err(format!("Error parsing {}: {}", path.display(), e));
            }
        };
        let mut declarations: Vec<SymbolsSearchResultStruct> = vec![];
        {
            let ast_index = self.ast_index.clone();
            let ast_index_locked = ast_index.lock().await;
            for sym in usage_result.search_results.iter() {
                declarations.extend(
                    match ast_index_locked.search_declarations(sym.symbol_path.as_str(), 1, Some(doc.clone())).await {
                        Ok(nodes) => nodes,
                        Err(e) => {
                            info!("Error searching for {}: {}", sym.symbol_path.as_str(), e);
                            vec![]
                        }
                    }
                )
            }
        }

        for rec in declarations.iter() {
            info!("distance {:.3}, found {}, ", rec.sim_to_query, rec.symbol_declaration.meta_path);
        }
        info!("search_by_cursor time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), declarations.len());
        Ok(
            AstCursorSearchResult {
                query_text: code.to_string(),
                file_path: doc.get_path(),
                cursor: cursor,
                cursor_symbols: usage_result.search_results,
                search_results: declarations
            }
        )
    }

    pub async fn search_declarations_by_symbol_path(
        &self,
        symbol_path: String,
        top_n: usize
    ) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        let ast_index = self.ast_index.clone();
        let ast_index_locked  = ast_index.lock().await;
        match ast_index_locked.search_declarations(symbol_path.as_str(), top_n, None).await {
            Ok(results) => {
                for r in results.iter() {
                    info!("distance {:.3}, found {}, ", r.sim_to_query, r.symbol_declaration.meta_path);
                }
                info!("search_by_symbol_path time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), results.len());
                Ok(
                    AstQuerySearchResult {
                        query_text: symbol_path,
                        search_results: results,
                    }
                )
            },
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn search_references_by_cursor(
        &mut self,
        doc: &DocumentInfo,
        code: &str,
        cursor: Point,
        top_n: usize
    ) -> Result<AstCursorSearchResult, String> {
        let t0 = std::time::Instant::now();

        let path = doc.get_path();
        let usage_result = match self.parse_near_cursor(doc, code, cursor, top_n).await {
            Ok(usages) => usages,
            Err(e) => {
                return Err(format!("Error parsing {}: {}", path.display(), e));
            }
        };
        let mut declarations: Vec<SymbolsSearchResultStruct> = vec![];
        {
            let ast_index = self.ast_index.clone();
            let ast_index_locked = ast_index.lock().await;
            for sym in usage_result.search_results.iter() {
                declarations.extend(
                    match ast_index_locked.search_usages(sym.symbol_path.as_str(), 3, Some(doc.clone())).await {
                        Ok(nodes) => nodes,
                        Err(e) => {
                            info!("Error searching for {}: {}", sym.symbol_path.as_str(), e);
                            vec![]
                        }
                    }
                )
            }
        }

        for rec in declarations.iter() {
            info!("distance {:.3}, found {}, ", rec.sim_to_query, rec.symbol_declaration.meta_path);
        }
        info!("search_by_cursor time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), declarations.len());
        Ok(
            AstCursorSearchResult {
                query_text: code.to_string(),
                file_path: doc.get_path(),
                cursor: cursor,
                cursor_symbols: usage_result.search_results,
                search_results: declarations
            }
        )
    }

    pub async fn search_references_by_symbol_path(
        &self,
        symbol_path: String,
        top_n: usize
    ) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        let ast_index = self.ast_index.clone();
        let ast_index_locked  = ast_index.lock().await;
        match ast_index_locked.search_usages(symbol_path.as_str(), top_n, None).await {
            Ok(results) => {
                for r in results.iter() {
                    info!("distance {:.3}, found {}, ", r.sim_to_query, r.symbol_declaration.meta_path);
                }
                info!("search_by_symbol_path time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), results.len());
                Ok(
                    AstQuerySearchResult {
                        query_text: symbol_path,
                        search_results: results,
                    }
                )
            },
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn get_file_symbols(&self, doc: &DocumentInfo) -> Result<FileReferencesResult, String> {
        let ast_index = self.ast_index.clone();
        let ast_index_locked  = ast_index.lock().await;
        let symbols = match ast_index_locked.get_symbols_by_file_path(&doc) {
            Ok(s) => s,
            Err(err) => { return Err(format!("Error: {}", err)) }
        };
        Ok(FileReferencesResult {
            file_path: doc.get_path(),
            symbols,
        })
    }

    pub async fn get_indexed_symbol_paths(&self) -> Vec<String> {
        let ast_index = self.ast_index.clone();
        let ast_index_locked  = ast_index.lock().await;
        ast_index_locked.get_indexed_symbol_paths()
    }

    pub async fn get_indexed_references(&self) -> Vec<String> {
        let ast_index = self.ast_index.clone();
        let ast_index_locked  = ast_index.lock().await;
        ast_index_locked.get_indexed_references()
    }

    pub async fn get_indexed_file_paths(&self) -> Vec<PathBuf> {
        let ast_index = self.ast_index.clone();
        let ast_index_locked  = ast_index.lock().await;
        ast_index_locked.get_indexed_file_paths()
    }

    async fn parse_near_cursor(
        &mut self,
        doc: &DocumentInfo,
        code: &str,
        cursor: Point,
        top_n: usize,
    ) -> Result<CursorUsagesResult, String> {
        let path = doc.get_path();
        let mut parser = match get_parser_by_filename(&path) {
            Ok(parser) => parser,
            Err(err) => {
                return Err(err.message);
            }
        };
        let usages = match parser.parse_usages(code) {
            Ok(usages) => usages,
            Err(e) => {
                return Err(format!("Error parsing {}: {}", path.display(), e));
            }
        };
        let filtered_usages = usages.iter()
            .unique_by(|x| x.meta_path())
            .sorted_by(|a, b| {
                a.distance_to_cursor(&cursor).cmp(&b.distance_to_cursor(&cursor))
            })
            .take(top_n)
            .collect::<Vec<_>>();

        Ok(CursorUsagesResult {
            file_path: path,
            query_text: code.to_string(),
            cursor: cursor.clone(),
            search_results: filtered_usages
                .iter()
                .map(|x| {
                    UsageSearchResultStruct {
                        symbol_path: x.meta_path(),
                        dist_to_cursor: x.distance_to_cursor(&cursor),
                        type_str: x.type_str(),
                    }
                })
                .collect::<Vec<UsageSearchResultStruct>>(),
        })
    }
}
