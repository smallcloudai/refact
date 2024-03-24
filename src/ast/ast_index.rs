use std::collections::HashMap;
use std::sync::Arc;

use fst::{Set, set, Streamer};
use itertools::Itertools;
use rayon::prelude::*;
use ropey::Rope;
use sorted_vec::SortedVec;
use strsim::jaro_winkler;
use tracing::{info};
use tree_sitter::Range;
use url::Url;
use crate::ast::comments_wrapper::get_language_id_by_filename;

use crate::ast::fst_extra_automation::Substring;
use crate::ast::structs::{FileASTMarkup, RowMarkup, SymbolsSearchResultStruct};
use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstance, FunctionCall, SymbolInformation};
use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::{get_new_parser_by_filename};
use crate::ast::treesitter::structs::SymbolType;
use crate::files_in_workspace::DocumentInfo;

#[derive(Debug)]
pub struct AstIndex {
    symbols_by_name: HashMap<String, Vec<Arc<dyn AstSymbolInstance>>>,
    symbols_by_guid: HashMap<String, Arc<dyn AstSymbolInstance>>,
    path_by_symbols: HashMap<Url, Vec<Arc<dyn AstSymbolInstance>>>,
    symbols_search_index: HashMap<Url, Set<Vec<u8>>>,
}


#[derive(Debug, Clone, Copy)]
pub(crate) enum RequestSymbolType {
    Declaration,
    Usage,
    All
}


fn make_a_query(
    nodes_indexes: &HashMap<Url, Set<Vec<u8>>>,
    query_str: &str,
    exception_doc: Option<DocumentInfo>,
) -> Vec<String> {
    let matcher = Substring::new(query_str, true);
    let mut stream_builder = set::OpBuilder::new();

    for (doc, set) in nodes_indexes {
        if let Some(ref exception) = exception_doc {
            if *doc == exception.uri {
                continue;
            }
        }
        stream_builder = stream_builder.add(set.search(matcher.clone()));
    }

    let mut stream = stream_builder.union();
    let mut found_keys = Vec::new();
    while let Some(key) = stream.next() {
        if let Ok(key_str) = String::from_utf8(key.to_vec()) {
            found_keys.push(key_str);
        }
    }
    found_keys
}

impl AstIndex {
    pub fn init() -> AstIndex {
        AstIndex {
            symbols_by_name: HashMap::new(),
            symbols_by_guid: HashMap::new(),
            path_by_symbols: HashMap::new(),
            symbols_search_index: HashMap::new(),
        }
    }

    pub fn parse(doc: &DocumentInfo) -> Result<Vec<Arc<dyn AstSymbolInstance>>, String> {
        let mut parser = match get_new_parser_by_filename(&doc.get_path()) {
            Ok(parser) => parser,
            Err(err) => {
                return Err(err.message);
            }
        };
        let text = match doc.read_file_blocked() {
            Ok(s) => s,
            Err(e) => return Err(e.to_string())
        };

        let t_ = std::time::Instant::now();
        let symbol_instances = parser.parse(text.as_str(), &doc.uri);
        let t_elapsed = t_.elapsed();

        info!(
            "parsed {}, {} symbols, took {:.3}s to parse",
            crate::nicer_logs::last_n_chars(&doc.uri.to_string(), 30),
            symbol_instances.len(), t_elapsed.as_secs_f32()
        );
        Ok(symbol_instances)
    }

    pub fn add_or_update_symbols_index(
        &mut self,
        doc: &DocumentInfo,
        symbols: &Vec<Arc<dyn AstSymbolInstance>>,
    ) -> Result<(), String> {
        match self.remove(&doc) {
            Ok(()) => (),
            Err(e) => return Err(format!("Error removing {}: {}", doc.uri, e)),
        }

        let mut symbol_names: SortedVec<String> = SortedVec::new();
        for symbol in symbols.iter() {
            self.symbols_by_name.entry(symbol.name().to_string()).or_insert_with(Vec::new).push(symbol.clone());
            self.symbols_by_guid.insert(symbol.guid().to_string(), symbol.clone());
            self.path_by_symbols.entry(doc.uri.clone()).or_insert_with(Vec::new).push(symbol.clone());
            symbol_names.push(symbol.name().to_string());
        }
        let meta_names_set = match Set::from_iter(symbol_names.iter()) {
            Ok(set) => set,
            Err(e) => return Err(format!("Error creating set: {}", e)),
        };
        self.symbols_search_index.insert(doc.uri.clone(), meta_names_set);

        Ok(())
    }

    pub fn add_or_update(&mut self, doc: &DocumentInfo) -> Result<(), String> {
        let symbols = AstIndex::parse(doc)?;
        self.add_or_update_symbols_index(doc, &symbols)
    }

    pub fn remove(&mut self, doc: &DocumentInfo) -> Result<(), String> {
        self.symbols_search_index.remove(&doc.uri);
        for symbol in self.path_by_symbols
            .remove(&doc.uri)
            .unwrap_or_default()
            .iter() {
            self.symbols_by_name.remove(symbol.name());
            self.symbols_by_guid.remove(symbol.guid());
        }
        Ok(())
    }

    pub fn clear_index(&mut self) {
        self.symbols_by_name.clear();
        self.symbols_by_guid.clear();
        self.path_by_symbols.clear();
        self.symbols_search_index.clear();
    }

    pub fn search_by_name(
        &self,
        query: &str,
        request_symbol_type: RequestSymbolType,
        exception_doc: Option<DocumentInfo>,
        language: Option<LanguageId>
    ) -> Result<Vec<SymbolsSearchResultStruct>, String> {
        fn exact_search(
            symbols_by_name: &HashMap<String, Vec<Arc<dyn AstSymbolInstance>>>,
            query: &str,
            request_symbol_type: &RequestSymbolType
        ) -> Vec<Arc<dyn AstSymbolInstance>> {
            symbols_by_name
                .get(query)
                .map(|x| x.clone())
                .unwrap_or_default()
                .iter()
                .cloned()
                .filter(|s| match request_symbol_type {
                    RequestSymbolType::Declaration => s.is_declaration(),
                    RequestSymbolType::Usage => !s.is_declaration(),
                    RequestSymbolType::All => true,
                })
                .collect()
        }

        fn fuzzy_search(
            search_index: &HashMap<Url, Set<Vec<u8>>>,
            symbols_by_name: &HashMap<String, Vec<Arc<dyn AstSymbolInstance>>>,
            query: &str,
            exception_doc: Option<DocumentInfo>,
            request_symbol_type: &RequestSymbolType
        ) -> Vec<Arc<dyn AstSymbolInstance>> {
            make_a_query(search_index, query, exception_doc)
                .iter()
                .map(|name| symbols_by_name
                    .get(name)
                    .map(|x| x.clone())
                    .unwrap_or_default())
                .flatten()
                .filter(|s| match request_symbol_type {
                    RequestSymbolType::Declaration => s.is_declaration(),
                    RequestSymbolType::Usage => !s.is_declaration(),
                    RequestSymbolType::All => true,
                })
                .collect()
        }

        let mut symbols = exact_search(&self.symbols_by_name, query, &request_symbol_type);
        if symbols.is_empty() {
            symbols = fuzzy_search(
                &self.symbols_search_index, &self.symbols_by_name,
                query, exception_doc, &request_symbol_type
            );
        }

        let mut filtered_search_results = symbols
            .iter()
            .filter(|s| s.language() == language.unwrap_or(s.language()))
            .map(|s| {
                (s.symbol_info_struct(), (jaro_winkler(query, s.name()) as f32).max(f32::MIN_POSITIVE))
            })
            .collect::<Vec<_>>();

        filtered_search_results
            .sort_by(|(_, dist_1), (_, dist_2)|
                dist_1.partial_cmp(dist_2).unwrap_or(std::cmp::Ordering::Equal)
            );

        let mut search_results: Vec<SymbolsSearchResultStruct> = vec![];
        for (key, dist) in filtered_search_results
            .iter()
            .rev() {
            let content = match key.get_content_blocked() {
                Ok(content) => content,
                Err(err) => {
                    info!("Error opening the file {:?}: {}", key.file_url, err);
                    continue;
                }
            };
            search_results.push(SymbolsSearchResultStruct {
                symbol_declaration: key.clone(),
                content: content,
                sim_to_query: dist.clone()
            });
        }
        Ok(search_results)
    }

    pub fn search_by_content(
        &self,
        query: &str,
        request_symbol_type: RequestSymbolType,
        exception_doc: Option<DocumentInfo>,
        language: Option<LanguageId>
    ) -> Result<Vec<SymbolsSearchResultStruct>, String> {
        let search_results = self.path_by_symbols
            .iter()
            .filter(|(path, symbols)| {
                let file_path = match path.to_file_path() {
                    Ok(fp) => fp,
                    Err(_) => return false,
                };
                let language_id = match get_language_id_by_filename(&file_path) {
                    Some(lid) => lid,
                    None => return false,
                };
                let correct_language = language.map_or(true, |l| l == language_id);
                let correct_doc = exception_doc.clone().map_or(true, |doc| doc.uri == **path);
                correct_doc && correct_language
            })
            .collect::<Vec<_>>()
            .par_iter()
            .filter_map(|(path, symbols)| {
                let mut found_symbols = vec![];
                let file_path = match path.to_file_path() {
                    Ok(path) => path,
                    Err(_) => return None
                };
                let file_content = match std::fs::read_to_string(&file_path) {
                    Ok(content) => content,
                    Err(err) => {
                        info!("Error opening the file {:?}: {}", &file_path, err);
                        return None;
                    }
                };
                let text_rope = Rope::from_str(file_content.as_str());
                for symbol in symbols.iter() {
                    let symbol_content = text_rope
                        .slice(text_rope.line_to_char(symbol.full_range().start_point.row)..
                            text_rope.line_to_char(symbol.full_range().end_point.row))
                        .to_string();
                    match symbol_content.find(query) {
                        Some(_) => found_symbols.push(symbol.clone()),
                        None => { continue }
                    }
                }
                Some(found_symbols)
            })
            .flatten()
            .filter(|s| match request_symbol_type {
                RequestSymbolType::Declaration => s.is_declaration(),
                RequestSymbolType::Usage =>!s.is_declaration(),
                RequestSymbolType::All => true,
            })
            .filter_map(|s| {
                let info_struct = s.symbol_info_struct();
                let content = info_struct.get_content_blocked().ok()?;
                Some(SymbolsSearchResultStruct {
                    symbol_declaration: info_struct,
                    content: content,
                    sim_to_query: -1.0
                })
            })
           .collect::<Vec<_>>();

        Ok(search_results)
    }

    pub fn search_related_declarations(&self, guid: &str) -> Result<Vec<SymbolsSearchResultStruct>, String> {
        unimplemented!()
    }

    pub fn search_usages_by_declarations(
        &self,
        declaration_guid: &str,
        exception_doc: Option<DocumentInfo>
    ) -> Result<Vec<SymbolsSearchResultStruct>, String> {
        unimplemented!()
    }

    pub async fn file_markup(
        &self,
        doc: &DocumentInfo
    ) -> Result<FileASTMarkup, String> {
        fn within_range(
            decl_range: &Range,
            line_idx: usize,
        ) -> bool {
            decl_range.start_point.row <= line_idx
                && decl_range.end_point.row >= line_idx
        }

        fn sorted_candidates_within_line(
            symbols: &Vec<Arc<dyn AstSymbolInstance>>,
            line_idx: usize,
        ) -> (Vec<Arc<dyn AstSymbolInstance>>, bool) {
            let filtered_symbols = symbols
                .iter()
                .filter(|s| within_range(&s.full_range(), line_idx))
                .sorted_by_key(
                    |s|
                        s.full_range().end_point.row - s.full_range().start_point.row
                )
                .rev()
                .cloned()
                .collect::<Vec<_>>();
            let is_signature = symbols
                .iter()
                .map(|s| within_range(&s.declaration_range(), line_idx))
                .any(|x| x);
            (filtered_symbols, is_signature)
        }

        let symbols: Vec<Arc<dyn AstSymbolInstance>> = self.path_by_symbols
            .get(&doc.uri)
            .map(|symbols| {
                symbols
                    .iter()
                    .filter(|s| s.is_declaration())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let file_content = match doc.read_file().await {
            Ok(content) => content,
            Err(e) => return Err(e.to_string())
        };

        let mut file_ast_markup = FileASTMarkup {
            file_url: doc.uri.clone(),
            file_content: file_content.clone(),
            symbols: symbols
                .iter()
                .map(|s| (s.guid().to_string(), s.symbol_info_struct()))
                .collect::<HashMap<String, SymbolInformation>>(),
            rows_markup: HashMap::new()
        };
        for (idx, line) in file_content.lines().enumerate() {
            let (candidate_symbols, is_signature) = sorted_candidates_within_line(&symbols, idx);
            file_ast_markup.rows_markup.insert(idx, RowMarkup {
                symbols_guid: candidate_symbols.iter().map(|s| s.guid().to_string()).collect(),
                line_content: line.to_string(),
                is_signature: is_signature
            });
        }

        Ok(file_ast_markup)
    }

    pub fn get_by_file_path(
        &self,
        request_symbol_type: RequestSymbolType,
        doc: &DocumentInfo
    ) -> Result<Vec<SymbolInformation>, String> {
        let symbols = self.path_by_symbols
            .get(&doc.uri)
            .map(|symbols| {
                symbols
                    .iter()
                    .filter(|s| match request_symbol_type {
                        RequestSymbolType::Declaration => s.is_declaration(),
                        RequestSymbolType::Usage => !s.is_declaration(),
                        RequestSymbolType::All => true,
                    })
                    .map(|s| s.symbol_info_struct())
                    .collect()
            })
            .unwrap_or_default();
        Ok(symbols)
    }

    pub fn get_file_paths(&self) -> Vec<Url> {
        self.symbols_search_index.iter().map(|(path, _)| path.clone()).collect()
    }

    pub fn get_all_symbols(
        &self,
        request_symbol_type: RequestSymbolType,
    ) -> Vec<SymbolInformation> {
        self.symbols_by_guid
            .iter()
            .filter(|(guid, s)| match request_symbol_type {
                RequestSymbolType::Declaration => s.is_declaration(),
                RequestSymbolType::Usage => !s.is_declaration(),
                RequestSymbolType::All => true,
            })
            .map(|(guid, s)| s.symbol_info_struct())
            .collect()
    }


    // fn merge_usages_to_declarations(&mut self) {
    //     for usage_symbol in self.symbols_by_guid
    //         .iter()
    //         .filter(|(guid, s)| !s.is_declaration())
    //         .map(|(guid, s)| s.clone())
    //         .collect::<Vec<_>>() {
    //
    //         match usage_symbol.symbol_type() {
    //             SymbolType::FunctionCall => {
    //                 let function_call_symbol = match usage_symbol
    //                     .as_any()
    //                     .downcast_ref::<FunctionCall>() {
    //                     Some(obj) => { obj }
    //                     None => continue,
    //                 };
    //                 function_call_symbol.
    //             }
    //             SymbolType::VariableUsage => {
    //
    //             }
    //             _ => {}
    //         }
    //     }
    // }

    // fn resolve_types(&mut self) {
    //
    // }
}


