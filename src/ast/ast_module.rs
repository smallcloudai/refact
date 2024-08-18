use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use itertools::Itertools;
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
use crate::ast::structs::{AstCursorSearchResult, AstDeclarationSearchResult, AstQuerySearchResult, AstReferencesSearchResult, FileASTMarkup, FileReferencesResult, SymbolsSearchResultStruct};
use crate::ast::treesitter::ast_instance_structs::AstSymbolInstanceRc;
use crate::ast::treesitter::structs::SymbolType;
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstIndexStatus {
    pub files_unparsed: usize,
    pub files_total: usize,
    pub ast_index_files_total: usize,
    pub ast_index_symbols_total: usize,
    pub state: String,
    pub ast_max_files_hit: bool,  // ast_index_files_total >= limit
}


pub struct AstModule {
    pub ast_index_service: Arc<AMutex<AstIndexService>>,
    ast_index: Arc<AMutex<AstIndex>>,
    status: Arc<AMutex<AstIndexStatus>>,
}


fn full_path_score(path: &str, query: &str) -> f32 {
    if jaro_winkler(&path, &query) <= 0.0 {
        return 0.0;
    }

    let mut score = 1.0;
    for query_comp in query.split("::") {
        for (idx, p) in path.split("::").collect::<Vec<_>>().into_iter().rev().enumerate() {
            let current_score = jaro_winkler(&query_comp, &p) as f32;
            // preliminary exit if we have a full match in the name
            if current_score >= 0.99 {
                return score;
            }
            score *= current_score * (1.0 / (idx + 1) as f32);
        }
    }
    score
}

fn symbol_to_search_res_struct(
    ast_ref: &MutexGuard<AstIndex>,
    query: &String,
    s: &AstSymbolInstanceRc,
) -> Option<SymbolsSearchResultStruct> {
    let mut info_struct = s.borrow().symbol_info_struct();
    info_struct.symbol_path = ast_ref.get_symbol_full_path(s);
    let name = info_struct.name.clone();
    let content = info_struct.get_content_from_file_blocked().ok()?;
    Some(SymbolsSearchResultStruct {
        symbol_declaration: info_struct,
        content,
        usefulness: jaro_winkler(&query, &name) as f32 * 100.0,
    })
}

impl AstModule {
    pub async fn ast_indexer_init(
        ast_max_files: usize,
        shutdown_flag: Arc<AtomicBool>,
        ast_light_mode: bool,
    ) -> Result<AstModule, String> {
        let status = Arc::new(AMutex::new(AstIndexStatus {
            files_unparsed: 0,
            files_total: 0,
            ast_index_files_total: 0,
            ast_index_symbols_total: 0,
            state: "starting".to_string(),
            ast_max_files_hit: false,
        }));
        let ast_index = Arc::new(AMutex::new(AstIndex::init(
            ast_max_files, shutdown_flag, ast_light_mode,
        )));
        let ast_index_service = Arc::new(AMutex::new(AstIndexService::init(
            ast_index.clone(),
            status.clone(),
        )));
        let me = AstModule {
            ast_index_service,
            ast_index,
            status,
        };
        Ok(me)
    }

    pub async fn ast_start_background_tasks(
        &self, gcx: Arc<ARwLock<GlobalContext>>,
    ) -> Vec<JoinHandle<()>> {
        return self.ast_index_service.lock().await.ast_start_background_tasks(gcx).await;
    }

    pub async fn ast_indexer_enqueue_files(&self, documents: &Vec<Document>, force: bool) {
        let mut documents_chunked = documents.chunks(16);
        while let Some(chunk) = documents_chunked.next() {
            self.ast_index_service.lock().await.ast_indexer_enqueue_files(AstEvent::add_docs(chunk.to_vec()), force).await;
        }
        if documents.is_empty() {
            self.ast_index_service.lock().await.ast_indexer_enqueue_files(
                AstEvent { docs: vec![], typ: AstEventType::AddDummy, posted_ts: std::time::SystemTime::now() },
                force,
            ).await;
        }
    }

    pub async fn ast_add_file_no_queue(&mut self, document: &Document, make_dirty: bool) -> Result<usize, String> {
        let mut ast_ref = match self.write_ast(Duration::from_secs(3)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        if ast_ref.is_overflowed() {
            let mut locked_status = self.status.lock().await;
            locked_status.ast_max_files_hit = true;
        }
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

    pub async fn read_ast(&self, duration: Duration) -> Result<MutexGuard<'_, AstIndex>, Elapsed> {
        timeout(duration, self.ast_index.lock()).await
    }

    async fn write_ast(&self, duration: Duration) -> Result<MutexGuard<'_, AstIndex>, Elapsed> {
        timeout(duration, self.ast_index.lock()).await
    }

    pub async fn search_declarations(&self, query: String) -> Result<AstDeclarationSearchResult, String> {
        fn log(t0: &std::time::Instant, res: &AstDeclarationSearchResult) {
            for r in res.exact_matches.iter() {
                let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                info!("exact_matches def-distance {:.3}, found {last_30_chars}", r.usefulness);
            }
            for r in res.fuzzy_matches.iter() {
                let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                info!("fuzzy_matches def-distance {:.3}, found {last_30_chars}", r.usefulness);
            }
            info!(
                "ast search_declarations time {:.3}s, found {} exact_matches, {} fuzzy_matches",
                t0.elapsed().as_secs_f32(),
                res.exact_matches.len(),
                res.fuzzy_matches.len(),
            );
        }

        let t0 = std::time::Instant::now();
        let ast_ref = match self.read_ast(Duration::from_millis(25)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };

        let results = ast_ref.search_by_fullpath(
            query.as_str(), RequestSymbolType::Declaration, None, None,
            false, false,
        );
        match results {
            Ok(symbols) => {
                if !symbols.is_empty() {
                    let res = AstDeclarationSearchResult {
                        query_text: query.clone(),
                        exact_matches: symbols
                            .iter()
                            .filter_map(|s| symbol_to_search_res_struct(&ast_ref, &query, &s))
                            .collect::<Vec<_>>(),
                        fuzzy_matches: vec![],
                    };
                    log(&t0, &res);
                    return Ok(res);
                }
            }
            Err(err) => {
                return Err(err.to_string());
            }
        }

        let query_lower = query.to_lowercase();
        let sorted_decl_symbols = ast_ref.get_symbols_paths(RequestSymbolType::Declaration)
            .iter()
            .filter(|x| x.to_lowercase().contains(&query_lower) && !x.is_empty())
            .map(|f| (f, full_path_score(&f, &query.to_string())))
            .sorted_by(|(_, dist1), (_, dist2)| dist1.partial_cmp(dist2).unwrap())
            .rev()
            .map(|(f, _)| {
                ast_ref.search_by_fullpath(
                    f.as_str(), RequestSymbolType::Declaration, None, None, false, false,
                ).unwrap_or_default()
            })
            .flatten()
            .collect::<Vec<_>>();
        let exact_matches = sorted_decl_symbols
            .iter()
            .filter(|s| s.borrow().name() == query)
            .filter_map(|s| symbol_to_search_res_struct(&ast_ref, &query, s))
            .collect::<Vec<_>>();

        let mut fuzzy_matches = sorted_decl_symbols
            .iter()
            .filter(|s| s.borrow().name() != query)
            .filter_map(|s| symbol_to_search_res_struct(&ast_ref, &query, s))
            .collect::<Vec<_>>();
        fuzzy_matches.sort_by(|a, b| b.usefulness.partial_cmp(&a.usefulness).unwrap());
        fuzzy_matches.truncate(10);
        let res = AstDeclarationSearchResult {
            query_text: query,
            exact_matches,
            fuzzy_matches,
        };
        log(&t0, &res);
        Ok(res)
    }

    pub async fn search_references(&self, query: String) -> Result<AstReferencesSearchResult, String> {
        fn log(t0: &std::time::Instant, res: &AstReferencesSearchResult) {
            for r in res.declaration_exact_matches.iter() {
                let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                info!("declaration_exact_matches def-distance {:.3}, found {last_30_chars}", r.usefulness);
            }
            for r in res.declaration_fuzzy_matches.iter() {
                let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                info!("declaration_fuzzy_matches def-distance {:.3}, found {last_30_chars}", r.usefulness);
            }
            for r in res.references_for_exact_matches.iter() {
                let last_30_chars = crate::nicer_logs::last_n_chars(&r.symbol_declaration.name, 30);
                info!("references_for_exact_matches def-distance {:.3}, found {last_30_chars}", r.usefulness);
            }
            info!(
                "ast search_references time {:.3}s, found {} decl_exact_matches, {} decl_fuzzy_matches, {} ref for decl_exact_matches",
                t0.elapsed().as_secs_f32(),
                res.declaration_exact_matches.len(),
                res.declaration_fuzzy_matches.len(),
                res.references_for_exact_matches.len(),
            );
        }

        let t0 = std::time::Instant::now();
        let declarations = match self.search_declarations(query.clone()).await {
            Ok(res) => res,
            Err(err) => {
                return Err(err.to_string());
            }
        };
        let ast_ref = match self.read_ast(Duration::from_millis(25)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        if declarations.exact_matches.is_empty() {
            let res = AstReferencesSearchResult {
                query_text: query,
                declaration_exact_matches: declarations.exact_matches,
                declaration_fuzzy_matches: declarations.fuzzy_matches,
                references_for_exact_matches: vec![],
            };
            log(&t0, &res);
            return Ok(res);
        }
        let func_calls_matched_by_name = declarations
            .exact_matches
            .iter()
            .filter(|s| s.symbol_declaration.symbol_type == SymbolType::FunctionDeclaration)
            .map(|s| {
                ast_ref.search_by_name(
                    &s.symbol_declaration.name, RequestSymbolType::Usage, None,
                    Some(s.symbol_declaration.language), false, false
                ).unwrap_or_default()
            })
            .flatten()
            .filter(|s| {
                s.borrow().symbol_type() == SymbolType::FunctionCall
            })
            .unique_by(|s| s.borrow().guid().clone())
            .collect::<Vec<_>>();
        let usages = declarations
            .exact_matches
            .iter()
            .map(|s| {
                let symbols_by_declarations = ast_ref.search_usages_with_this_declaration(
                    &s.symbol_declaration.guid, None
                ).unwrap_or_default();
                symbols_by_declarations
                    .iter()
                    .sorted_unstable_by_key(|s| {
                        match s.borrow().symbol_type() {
                            SymbolType::ClassFieldDeclaration => 1,
                            SymbolType::VariableDefinition => 1,
                            SymbolType::FunctionDeclaration => 1,
                            SymbolType::FunctionCall => 2,
                            SymbolType::VariableUsage => 2,
                            _ => 0,
                        }
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .flatten()
            .chain(func_calls_matched_by_name)
            .unique_by(|s| s.borrow().guid().clone())
            .collect::<Vec<_>>();
        let res = AstReferencesSearchResult {
            query_text: query.clone(),
            declaration_exact_matches: declarations.exact_matches,
            declaration_fuzzy_matches: declarations.fuzzy_matches,
            references_for_exact_matches: usages
                .iter()
                .filter_map(|s| symbol_to_search_res_struct(&ast_ref, &query, s))
                .collect::<Vec<_>>(),
        };
        log(&t0, &res);
        Ok(res)
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
                        let mut info_struct = s.borrow().symbol_info_struct();
                        info_struct.symbol_path = ast_ref.get_symbol_full_path(s);
                        let name = info_struct.name.clone();
                        let content = info_struct.get_content_from_file_blocked().ok()?;
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
                        refs_n: results.len(),
                    }
                )
            }
            Err(e) => Err(e.to_string())
        }
    }

    pub async fn search_by_fullpath(
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
        match ast_ref.search_by_fullpath(query.as_str(), request_symbol_type, None, None, try_fuzzy_if_not_found, true) {
            Ok(results) => {
                let symbol_structs = results
                    .iter()
                    .take(top_n)
                    .filter_map(|s| {
                        let mut info_struct = s.borrow().symbol_info_struct();
                        info_struct.symbol_path = ast_ref.get_symbol_full_path(s);
                        let name = info_struct.name.clone();
                        let content = info_struct.get_content_from_file_blocked().ok()?;
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
                        refs_n: results.len(),
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
                        let content = info_struct.get_content_from_file_blocked().ok()?;
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
                        refs_n: results.len(),
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
                        let content = info_struct.get_content_from_file_blocked().ok()?;
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
                info!("ast search_related_declarations time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), results.len());
                Ok(
                    AstQuerySearchResult {
                        query_text: guid.to_string(),
                        search_results: symbol_structs,
                        refs_n: results.len(),
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
                        let content = info_struct.get_content_from_file_blocked().ok()?;
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
                info!("ast search_usages_by_declarations time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), results.len());
                Ok(
                    AstQuerySearchResult {
                        query_text: declaration_guid.to_string(),
                        search_results: symbol_structs,
                        refs_n: results.len(),
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
            let content = if symbol_declaration.file_path == doc.path {
                symbol_declaration.get_content(&code.to_string()).unwrap_or_default()
            } else {
                symbol_declaration.get_content_from_file_blocked().unwrap_or_default()
            };
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

    pub async fn decl_symbols_from_imports_by_file_path(
        &self,
        doc: &Document,
        imports_depth: usize,
    ) -> Result<AstQuerySearchResult, String> {
        let t0 = std::time::Instant::now();
        let ast_ref = match self.read_ast(Duration::from_millis(25)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        let results = ast_ref.decl_symbols_from_imports_by_file_path(&doc, imports_depth);
        let symbol_structs = results
            .iter()
            .filter_map(|s| {
                let info_struct = s.borrow().symbol_info_struct();
                let content = info_struct.get_content_from_file_blocked().ok()?;
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
        info!("ast decl_symbols_from_imports_by_file_path time {:.3}s, found {} results", t0.elapsed().as_secs_f32(), results.len());
        Ok(
            AstQuerySearchResult {
                query_text: "".to_string(),
                search_results: symbol_structs,
                refs_n: results.len(),
            }
        )
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

    pub async fn get_symbols_paths(&self, request_symbol_type: RequestSymbolType) -> Result<Vec<String>, String> {
        let ast_ref = match self.read_ast(Duration::from_millis(25)).await {
            Ok(ast) => ast,
            Err(_) => {
                return Err("ast timeout".to_string());
            }
        };
        Ok(ast_ref.get_symbols_paths(request_symbol_type))
    }

    pub async fn ast_index_status(&self) -> AstIndexStatus {
        let mut locked_status = self.status.lock().await;
        match self.read_ast(Duration::from_millis(25)).await {
            Ok(ast) => {
                locked_status.ast_index_files_total = ast.total_files();
                locked_status.ast_index_symbols_total = ast.total_symbols();
            }
            Err(_) => {}
        };
        locked_status.clone()
    }
}
