use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use std::cell::RefCell;
use std::path::PathBuf;

use fst::{Set, set, Streamer};
use itertools::Itertools;
use rayon::prelude::*;
use ropey::Rope;
use sorted_vec::SortedVec;
use strsim::jaro_winkler;
use tracing::info;
use tree_sitter::Point;

use crate::ast::comments_wrapper::get_language_id_by_filename;
use crate::ast::fst_extra_automation::Substring;
use crate::ast::structs::{FileASTMarkup, SymbolsSearchResultStruct};
use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstanceArc, SymbolInformation};
use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::get_ast_parser_by_filename;
use crate::ast::treesitter::structs::SymbolType;
use crate::ast::usages_declarations_merger::{FilePathIterator, find_decl_by_name, find_decl_by_caller_guid};
use crate::files_in_workspace::{Document, read_file_from_disk};


#[derive(Debug)]
pub struct AstIndex {
    symbols_by_name: HashMap<String, Vec<AstSymbolInstanceArc>>,
    symbols_by_guid: HashMap<String, AstSymbolInstanceArc>,
    path_by_symbols: HashMap<PathBuf, Vec<AstSymbolInstanceArc>>,
    symbols_search_index: HashMap<PathBuf, Set<Vec<u8>>>,
    type_guid_to_dependand_guids: HashMap<String, Vec<String>>,
    has_changes: bool,
}


#[derive(Debug, Clone, Copy)]
pub(crate) enum RequestSymbolType {
    Declaration,
    Usage,
    All,
}

#[derive(Debug)]
pub(crate) struct IndexingStats {
    pub(crate) found: usize,
    pub(crate) non_found: usize,
}


fn make_a_query(
    nodes_indexes: &HashMap<PathBuf, Set<Vec<u8>>>,
    query_str: &str,
    exception_doc: Option<Document>,
) -> Vec<String> {
    let matcher = Substring::new(query_str, true);
    let mut stream_builder = set::OpBuilder::new();

    for (doc_path, set) in nodes_indexes {
        if let Some(ref exception) = exception_doc {
            if *doc_path == exception.path {
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
            type_guid_to_dependand_guids: HashMap::new(),
            has_changes: false,
        }
    }

    pub(crate) fn parse(doc: &Document) -> Result<Vec<AstSymbolInstanceArc>, String> {
        let mut parser = match get_ast_parser_by_filename(&doc.path) {
            Ok(parser) => parser,
            Err(err) => {
                return Err(err.message);
            }
        };
        let text = doc.text.clone().unwrap_or_default().to_string();
        let t_ = std::time::Instant::now();
        let symbol_instances = parser.parse(&text, &doc.path);
        let t_elapsed = t_.elapsed();

        info!(
            "parsed {}, {} symbols, took {:.3}s to parse",
            crate::nicer_logs::last_n_chars(&doc.path.display().to_string(), 30),
            symbol_instances.len(), t_elapsed.as_secs_f32()
        );
        Ok(symbol_instances)
    }

    pub async fn add_or_update_symbols_index(&mut self, doc: &Document, symbols: &Vec<AstSymbolInstanceArc>) -> Result<(), String> {
        let has_removed = self.remove(&doc);
        if has_removed {
            self.resolve_types(symbols).await;
            self.merge_usages_to_declarations(symbols).await;
            self.create_extra_indexes(symbols);
            self.has_changes = false;
        } else {
            // TODO: we don't want to update the whole index for a single file
            // even if we might miss some new cross-references
            // later we should think about some kind of force update, ie once in a while self.has_changes=false
            self.has_changes = true;
        }

        let mut symbol_names: SortedVec<String> = SortedVec::new();
        for symbol in symbols.iter() {
            let symbol_ref = symbol.read().expect("the data might be broken");
            self.symbols_by_name.entry(symbol_ref.name().to_string()).or_insert_with(Vec::new).push(symbol.clone());
            self.symbols_by_guid.insert(symbol_ref.guid().to_string(), symbol.clone());
            self.path_by_symbols.entry(doc.path.clone()).or_insert_with(Vec::new).push(symbol.clone());
            symbol_names.push(symbol_ref.name().to_string());
        }
        let meta_names_set = match Set::from_iter(symbol_names.iter()) {
            Ok(set) => set,
            Err(e) => return Err(format!("Error creating set: {}", e)),
        };
        self.symbols_search_index.insert(doc.path.clone(), meta_names_set);

        Ok(())
    }

    pub async fn add_or_update(&mut self, doc: &Document) -> Result<(), String> {
        let symbols = AstIndex::parse(doc)?;
        self.add_or_update_symbols_index(doc, &symbols).await
    }

    pub fn remove(&mut self, doc: &Document) -> bool {
        let has_removed = self.symbols_search_index.remove(&doc.path).is_some();
        if !has_removed {
            return false;
        }
        let mut removed_guids = HashSet::new();
        for symbol in self.path_by_symbols
            .remove(&doc.path)
            .unwrap_or_default()
            .iter() {
            let (name, guid) = {
                let symbol_ref = symbol.read().expect("the data might be broken");
                (symbol_ref.name().to_string(), symbol_ref.guid().to_string())
            };
            self.symbols_by_name
                .entry(name)
                .and_modify(|v| {
                    let indices_to_remove = v
                        .iter()
                        .enumerate()
                        .filter(|(_idx, s)| s.read().expect("the data might be broken").guid() == guid)
                        .map(|(idx, _s)| idx)
                        .collect::<Vec<_>>();
                    indices_to_remove.iter().for_each(|i| { v.remove(*i); });
                });

            self.symbols_by_guid.remove(&guid);
            if self.type_guid_to_dependand_guids.contains_key(&guid) {
                // TODO: we should do the removing more precisely,
                // some leftovers still are in the values, but it doesn't break the overall thing for now
                self.type_guid_to_dependand_guids.remove(&guid);
            }
            removed_guids.insert(guid);
        }
        for symbol in self.symbols_by_guid.values_mut() {
            symbol.write().expect("the data might be broken").remove_linked_guids(&removed_guids);
        }
        self.has_changes = true;
        has_removed
    }

    pub fn clear_index(&mut self) {
        self.symbols_by_name.clear();
        self.symbols_by_guid.clear();
        self.path_by_symbols.clear();
        self.symbols_search_index.clear();
        self.has_changes = true;
    }

    pub fn search_by_name(
        &self,
        query: &str,
        request_symbol_type: RequestSymbolType,
        exception_doc: Option<Document>,
        language: Option<LanguageId>,
        try_fuzzy_if_not_found: bool
    ) -> Result<Vec<SymbolsSearchResultStruct>, String> {
        fn exact_search(
            symbols_by_name: &HashMap<String, Vec<AstSymbolInstanceArc>>,
            query: &str,
            request_symbol_type: &RequestSymbolType,
        ) -> Vec<AstSymbolInstanceArc> {
            symbols_by_name
                .get(query)
                .map(|x| x.clone())
                .unwrap_or_default()
                .iter()
                .cloned()
                .filter(|s| {
                    let s_ref = s.read().expect("the data might be broken");
                    match request_symbol_type {
                        RequestSymbolType::Declaration => s_ref.is_declaration(),
                        RequestSymbolType::Usage => !s_ref.is_declaration(),
                        RequestSymbolType::All => true,
                    }
                })
                .collect()
        }

        fn fuzzy_search(
            search_index: &HashMap<PathBuf, Set<Vec<u8>>>,
            symbols_by_name: &HashMap<String, Vec<AstSymbolInstanceArc>>,
            query: &str,
            exception_doc: Option<Document>,
            request_symbol_type: &RequestSymbolType,
        ) -> Vec<AstSymbolInstanceArc> {
            make_a_query(search_index, query, exception_doc)
                .iter()
                .map(|name| symbols_by_name
                    .get(name)
                    .map(|x| x.clone())
                    .unwrap_or_default())
                .flatten()
                .filter(|s| {
                    let s_ref = s.read().expect("the data might be broken");
                    match request_symbol_type {
                        RequestSymbolType::Declaration => s_ref.is_declaration(),
                        RequestSymbolType::Usage => !s_ref.is_declaration(),
                        RequestSymbolType::All => true,
                    }
                })
                .collect()
        }

        let mut symbols = exact_search(&self.symbols_by_name, query, &request_symbol_type);
        if try_fuzzy_if_not_found && symbols.is_empty() {
            symbols = fuzzy_search(
                &self.symbols_search_index, &self.symbols_by_name,
                query, exception_doc, &request_symbol_type,
            );
        }

        let mut filtered_search_results = symbols
            .iter()
            .filter(|s| {
                let s_ref = s.read().expect("the data might be broken");
                *s_ref.language() == language.unwrap_or(*s_ref.language())
            })
            .map(|s| {
                let s_ref = s.read().expect("the data might be broken");
                (s_ref.symbol_info_struct(), (jaro_winkler(query, s_ref.name()) as f32).max(f32::MIN_POSITIVE))
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
                    info!("Error opening the file {:?}: {}", key.file_path, err);
                    continue;
                }
            };
            search_results.push(SymbolsSearchResultStruct {
                symbol_declaration: key.clone(),
                content,
                sim_to_query: dist.clone(),
            });
        }
        Ok(search_results)
    }

    pub async fn search_by_content(
        &self,
        query: &str,
        request_symbol_type: RequestSymbolType,
        exception_doc: Option<Document>,
        language: Option<LanguageId>,
    ) -> Result<Vec<SymbolsSearchResultStruct>, String> {
        let search_results = self.path_by_symbols
            .iter()
            .filter(|(path, _symbols)| {
                let language_id = match get_language_id_by_filename(path) {
                    Some(lid) => lid,
                    None => return false,
                };
                let correct_language = language.map_or(true, |l| l == language_id);
                let correct_doc = exception_doc.clone().map_or(true, |doc| doc.path != **path);
                correct_doc && correct_language
            })
            .collect::<Vec<_>>()
            .par_iter()
            .filter_map(|(path, symbols)| {
                let mut found_symbols = vec![];
                let file_content = match read_file_from_disk_block(path) {
                    Ok(content) => content,
                    Err(err) => {
                        info!("Error opening the file {:?}: {}", path, err);
                        return None;
                    }
                };
                let text_rope = Rope::from_str(file_content.as_str());
                for symbol in symbols.iter() {
                    let s_ref = symbol.read().expect("the data might be broken");
                    let symbol_content = text_rope
                        .slice(text_rope.line_to_char(s_ref.full_range().start_point.row)..
                            text_rope.line_to_char(s_ref.full_range().end_point.row))
                        .to_string();
                    match symbol_content.find(query) {
                        Some(_) => found_symbols.push(symbol.clone()),
                        None => { continue; }
                    }
                }
                Some(found_symbols)
            })
            .flatten()
            .filter(|s| {
                let s_ref = s.read().expect("the data might be broken");
                match request_symbol_type {
                    RequestSymbolType::Declaration => s_ref.is_declaration(),
                    RequestSymbolType::Usage => !s_ref.is_declaration(),
                    RequestSymbolType::All => true,
                }
            })
            .filter_map(|s| {
                let info_struct = s.read().expect("the data might be broken").symbol_info_struct();
                let content = info_struct.get_content_blocked().ok()?;
                Some(SymbolsSearchResultStruct {
                    symbol_declaration: info_struct,
                    content: content,
                    sim_to_query: -1.0,
                })
            })
            .collect::<Vec<_>>();

        Ok(search_results)
    }

    pub fn search_related_declarations(&self, guid: &str) -> Result<Vec<SymbolsSearchResultStruct>, String> {
        match self.symbols_by_guid.get(guid) {
            Some(symbol) => {
                Ok(symbol
                    .read().expect("the data might be broken")
                    .types()
                    .iter()
                    .filter_map(|t| t.guid.clone())
                    .filter_map(|g| self.symbols_by_guid.get(&g))
                    .filter_map(|s| {
                        let info_struct = s.read().expect("the data might be broken").symbol_info_struct();
                        let content = info_struct.get_content_blocked().ok()?;
                        Some(SymbolsSearchResultStruct {
                            symbol_declaration: info_struct,
                            content,
                            sim_to_query: -1.0,
                        })
                    })
                    .collect::<Vec<_>>())
            }
            _ => Ok(vec![])
        }
    }

    pub fn search_symbols_by_declarations_usage(
        &self,
        declaration_guid: &str,
        exception_doc: Option<Document>,
    ) -> Result<Vec<SymbolsSearchResultStruct>, String> {
        Ok(self.type_guid_to_dependand_guids
            .get(declaration_guid)
            .map(|x| x.clone())
            .unwrap_or_default()
            .iter()
            .filter_map(|guid| self.symbols_by_guid.get(guid))
            .filter(|s| {
                let s_ref = s.read().expect("the data might be broken");
                exception_doc.clone().map_or(true, |doc| doc.path != *s_ref.file_path())
            })
            .filter_map(|s| {
                let info_struct = s.read().expect("the data might be broken").symbol_info_struct();
                let content = info_struct.get_content_blocked().ok()?;
                Some(SymbolsSearchResultStruct {
                    symbol_declaration: info_struct,
                    content,
                    sim_to_query: -1.0,
                })
            })
            .collect::<Vec<_>>())
    }

    pub async fn retrieve_cursor_symbols_by_declarations(
        &self,
        doc: &Document,
        code: &str,
        cursor: Point,
        top_n_near_cursor: usize,
        top_n_usage_for_each_decl: usize,
    ) -> (Vec<SymbolInformation>, Vec<SymbolInformation>, Vec<SymbolInformation>) {
        let file_symbols = self.parse_single_file(doc, code).await;
        let language = get_language_id_by_filename(&doc.path);

        let unfiltered_cursor_symbols = file_symbols
            .iter()
            .unique_by(|s| {
                let s_ref = s.read().expect("the data might be broken");
                (s_ref.guid().to_string(), s_ref.name().to_string())
            })
            .filter(|s| !s.read().expect("the data might be broken").name().is_empty())
            .sorted_by_key(|a| a.read().expect("the data might be broken").distance_to_cursor(&cursor))
            .collect::<Vec<_>>();

        let cursor_symbols_with_types = unfiltered_cursor_symbols
            .iter()
            .cloned()
            .filter_map(|s| {
                let s_ref = s.read().expect("the data might be broken");
                if s_ref.is_declaration() {
                    Some(s)
                } else {
                    s_ref.get_linked_decl_guid()
                        .clone()
                        .map(|guid| self.symbols_by_guid.get(&guid))
                        .flatten()
                }
            })
            .unique_by(|s| s.read().expect("the data might be broken").guid().to_string())
            .cloned()
            .collect::<Vec<_>>();
        let declarations_matched_by_name = unfiltered_cursor_symbols
            .iter()
            .cloned()
            .map(|s| {
                let s_ref = s.read().expect("the data might be broken");
                let use_fuzzy_search = s_ref.full_range().start_point.row == cursor.row;
                self.search_by_name(&s_ref.name(), RequestSymbolType::Declaration, Some(doc.clone()), language.clone(), use_fuzzy_search)
                    .unwrap_or_else(|_| vec![])
            })
            .flatten()
            .filter(|s| {
                s.symbol_declaration.symbol_type == SymbolType::StructDeclaration
                    || s.symbol_declaration.symbol_type == SymbolType::TypeAlias
                    || s.symbol_declaration.symbol_type == SymbolType::FunctionDeclaration
            })
            .map(|s| s.symbol_declaration)
            .unique_by(|s| (s.guid.clone(), s.name.clone()))
            .take(top_n_near_cursor)
            .collect::<Vec<_>>();

        let mut declarations = cursor_symbols_with_types
            .iter()
            .filter(|s| !s.read().expect("the data might be broken").types().is_empty())
            .map(|s| {
                s.read().expect("the data might be broken")
                    .types()
                    .iter()
                    .filter_map(|t| t.guid.clone())
                    .filter_map(|g| self.symbols_by_guid.get(&g))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .flatten()
            .filter(|s| {
                let s_ref = s.read().expect("the data might be broken");
                *s_ref.language() == language.unwrap_or(*s_ref.language())
            })
            .map(|s| s.read().expect("the data might be broken").symbol_info_struct())
            .unique_by(|s| (s.guid.clone(), s.name.clone()))
            .take(top_n_near_cursor)
            .collect::<Vec<_>>();
        declarations.extend(declarations_matched_by_name);

        let mut usages = declarations
            .iter()
            .map(|s| {
                self.search_symbols_by_declarations_usage(&s.guid, Some(doc.clone()))
                    .unwrap_or_default()
                    .iter()
                    .map(|x| x.symbol_declaration.clone())
                    .sorted_by_key(|s| {
                        match s.symbol_type {
                            SymbolType::ClassFieldDeclaration => 1,
                            SymbolType::VariableDefinition => 1,
                            SymbolType::FunctionDeclaration => 1,
                            SymbolType::FunctionCall => 2,
                            SymbolType::VariableUsage => 2,
                            _ => 0,
                        }
                    })
                    .take(top_n_usage_for_each_decl)
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();

        let func_usages_matched_by_name = declarations
            .iter()
            .filter(|s| s.symbol_type == SymbolType::FunctionDeclaration)
            .map(|s| {
                let use_fuzzy_search = s.full_range.start_point.row == cursor.row;
                self.search_by_name(&s.name, RequestSymbolType::Usage, Some(doc.clone()), language.clone(), use_fuzzy_search)
                    .unwrap_or_else(|_| vec![])
            })
           .flatten()
           .filter(|s| {
                s.symbol_declaration.symbol_type == SymbolType::FunctionCall
            })
           .map(|s| s.symbol_declaration)
           .unique_by(|s| (s.guid.clone(), s.name.clone()))
           .take(top_n_usage_for_each_decl)
           .collect::<Vec<_>>();
        usages.extend(func_usages_matched_by_name);

        (
            unfiltered_cursor_symbols
                .iter()
                .unique_by(|s| s.read().expect("the data might be broken").name().to_string())
                .map(|s| s.read().expect("the data might be broken").symbol_info_struct())
                .collect(),
            declarations
                .iter()
                .filter(|s| doc.path != s.file_path)
                .unique_by(|s| (s.guid.clone(), s.name.clone()))
                .cloned()
                .collect::<Vec<_>>(),
            usages
                .iter()
                .filter(|s| doc.path != s.file_path)
                .unique_by(|s| (s.guid.clone(), s.name.clone()))
                .cloned()
                .collect::<Vec<_>>(),
        )
    }

    pub async fn file_markup(
        &self,
        doc: &Document,
    ) -> Result<FileASTMarkup, String> {
        let symbols: Vec<AstSymbolInstanceArc> = self.path_by_symbols
            .get(&doc.path)
            .map(|symbols| {
                symbols
                    .iter()
                    .filter(|s| s.read().expect("the data might be broken").is_declaration())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let file_content = match read_file_from_disk(&doc.path).await {
            Ok(content) => content.to_string(),
            Err(e) => return Err(e.to_string())
        };

        let mut symbols4export: Vec<Arc<RefCell<SymbolInformation>>> = symbols.iter().map(|s| {
            let s_ref = s.read().expect("the data might be broken");
            Arc::new(RefCell::new(s_ref.symbol_info_struct()))
        }).collect();
        let guid_to_symbol: HashMap<String, Arc<RefCell<SymbolInformation>>> = symbols4export.iter().map(
                |s| (s.borrow().guid.to_string(), s.clone())
            ).collect();
        fn recursive_path_of_guid(guid_to_symbol: &HashMap<String, Arc<RefCell<SymbolInformation>>>, guid: &String) -> String
        {
            return match guid_to_symbol.get(guid) {
                Some(x) => {
                    let pname = if !x.borrow().name.is_empty() { x.borrow().name.clone() } else { x.borrow().guid[..8].to_string() };
                    let pp = recursive_path_of_guid(&guid_to_symbol, &x.borrow().parent_guid);
                    format!("{}::{}", pp, pname)
                },
                None => {
                    // FIXME:
                    // info!("parent_guid {} not found, maybe outside of this file", guid);
                    "UNK".to_string()
                }
            };
        }
        for s in symbols4export.iter_mut() {
            let symbol_path = recursive_path_of_guid(&guid_to_symbol, &s.borrow().guid);
            s.borrow_mut().symbol_path = symbol_path.clone();
        }
        // longer symbol path at the bottom => parent always higher than children
        symbols4export.sort_by(|a, b| {
            a.borrow().symbol_path.len().cmp(&b.borrow().symbol_path.len())
        });
        Ok(FileASTMarkup {
            file_path: doc.path.clone(),
            file_content,
            // convert to a simple Vec<SymbolInformation>
            symbols_sorted_by_path_len: symbols4export.iter().map(|s| {
                s.borrow().clone()
            }).collect()
        })
    }

    pub fn get_by_file_path(
        &self,
        request_symbol_type: RequestSymbolType,
        doc: &Document,
    ) -> Result<Vec<SymbolInformation>, String> {
        let symbols = self.path_by_symbols
            .get(&doc.path)
            .map(|symbols| {
                symbols
                    .iter()
                    .filter(|s| {
                        let s_ref = s.read().expect("the data might be broken");
                        match request_symbol_type {
                            RequestSymbolType::Declaration => s_ref.is_declaration(),
                            RequestSymbolType::Usage => !s_ref.is_declaration(),
                            RequestSymbolType::All => true,
                        }
                    })
                    .map(|s| s.read().expect("the data might be broken").symbol_info_struct())
                    .collect()
            })
            .unwrap_or_default();
        Ok(symbols)
    }

    #[allow(unused)]
    pub fn get_file_paths(&self) -> Vec<PathBuf> {
        self.symbols_search_index.iter().map(|(path, _)| path.clone()).collect()
    }

    pub fn get_symbols_names(
        &self,
        request_symbol_type: RequestSymbolType,
    ) -> Vec<String> {
        self.symbols_by_guid
            .iter()
            .filter(|(_guid, s)| {
                let s_ref = s.read().expect("the data might be broken");
                match request_symbol_type {
                    RequestSymbolType::Declaration => s_ref.is_declaration(),
                    RequestSymbolType::Usage => !s_ref.is_declaration(),
                    RequestSymbolType::All => true,
                }
            })
            .map(|(_guid, s)| s.read().expect("the data might be broken").name().to_string())
            .collect()
    }

    pub(crate) fn symbols_by_guid(&self) -> &HashMap<String, AstSymbolInstanceArc> {
        &self.symbols_by_guid
    }

    pub(crate) fn need_update(&self) -> bool {
        self.has_changes
    }

    pub(crate) fn set_updated(&mut self) {
        self.has_changes = false;
    }

    pub(crate) async fn resolve_types(&self, symbols: &Vec<AstSymbolInstanceArc>) -> IndexingStats {
        let mut stats = IndexingStats { found: 0, non_found: 0 };
        for symbol in symbols {
            tokio::task::yield_now().await;
            let (type_names, symb_type, symb_path) = {
                let s_ref = symbol.read().expect("the data might be broken");
                (s_ref.types(), s_ref.symbol_type(), s_ref.file_path().clone())
            };
            if symb_type == SymbolType::ImportDeclaration
                || symb_type == SymbolType::CommentDefinition
                || symb_type == SymbolType::FunctionCall
                || symb_type == SymbolType::VariableUsage {
                continue;
            }

            let mut new_guids = vec![];
            for (_, t) in type_names.iter().enumerate() {
                // TODO: make a type inference by `inference_info`
                if t.is_pod || t.guid.is_some() || t.name.is_none() {
                    stats.non_found += 1;
                    new_guids.push(t.guid.clone());
                    continue;
                }

                stats.found += 1;
                let name = t.name.clone().expect("filter has invalid condition");
                let maybe_guid = match self.symbols_by_name.get(&name) {
                    Some(symbols) => {
                        symbols
                            .iter()
                            .filter(|s| s.read().expect("the data might be broken").is_type())
                            .sorted_by(|a, b| {
                                let path_a = a.read().expect("the data might be broken").file_path().clone();
                                let path_b = b.read().expect("the data might be broken").file_path().clone();
                                FilePathIterator::compare_paths(&symb_path, &path_a, &path_b)
                            })
                            .next()
                            .map(|s| s.read().expect("the data might be broken").guid().to_string())
                    }
                    None => {
                        stats.non_found += 1;
                        new_guids.push(None);
                        continue;
                    }
                };

                match maybe_guid {
                    Some(guid) => {
                        stats.found += 1;
                        new_guids.push(Some(guid));
                    }
                    None => {
                        stats.non_found += 1;
                        new_guids.push(None);
                    }
                }
            }
            assert_eq!(new_guids.len(), type_names.len());
            symbol
                .write().expect("the data might be broken")
                .set_guids_to_types(&new_guids);
        }
        stats
    }

    pub(crate) async fn merge_usages_to_declarations(&self, symbols: &Vec<AstSymbolInstanceArc>) -> IndexingStats {
        fn get_caller_depth(
            symbol: &AstSymbolInstanceArc,
            guid_by_symbols: &HashMap<String, AstSymbolInstanceArc>,
            current_depth: usize,
        ) -> usize {
            let caller_guid = match symbol
                .read().expect("the data might be broken")
                .get_caller_guid()
                .clone() {
                Some(g) => g,
                None => return current_depth,
            };
            match guid_by_symbols.get(&caller_guid) {
                Some(s) => get_caller_depth(
                    s, guid_by_symbols, current_depth + 1,
                ),
                None => current_depth,
            }
        }

        let mut stats = IndexingStats { found: 0, non_found: 0 };
        let extra_index: HashMap<(String, String, String), AstSymbolInstanceArc> = symbols
            .iter()
            .map(|x| {
                let x_ref = x.read().expect("the data might be broken");
                ((x_ref.name().to_string(),
                  x_ref.parent_guid().clone().unwrap_or_default(),
                  x_ref.file_path().to_str().unwrap_or_default().to_string()),
                 x.clone())
            })
            .collect();
        let mut depth: usize = 0;
        loop {
            let symbols_to_process = symbols
                .iter()
                .filter(|symbol| {
                    symbol.read().expect("the data might be broken").get_linked_decl_guid().is_none()
                })
                .filter(|symbol| {
                    let s_ref = symbol.read().expect("the data might be broken");
                    let valid_depth = get_caller_depth(symbol, &self.symbols_by_guid, 0) == depth;
                    valid_depth && (s_ref.symbol_type() == SymbolType::FunctionCall
                        || s_ref.symbol_type() == SymbolType::VariableUsage)
                })
                .collect::<Vec<_>>();

            if symbols_to_process.is_empty() {
                break;
            }

            let mut symbols_cache: HashMap<(String, String), Option<String>> = HashMap::new();
            for (_, usage_symbol) in symbols_to_process
                .iter()
                .enumerate() {
                tokio::task::yield_now().await;
                let (name, parent_guid, caller_guid) = {
                    let s_ref = usage_symbol.read().expect("the data might be broken");
                    (s_ref.name().to_string(), s_ref.parent_guid().clone().unwrap_or_default(), s_ref.get_caller_guid().clone())
                };
                let guids_pair = (parent_guid, name);
                let decl_guid = if !symbols_cache.contains_key(&guids_pair) {
                    let decl_guid = match caller_guid {
                        Some(ref guid) => {
                            match find_decl_by_caller_guid(
                                *usage_symbol,
                                &guid,
                                &self.symbols_by_guid,
                            ) {
                                Some(decl_guid) => { Some(decl_guid) }
                                None => find_decl_by_name(
                                    *usage_symbol,
                                    &self.path_by_symbols,
                                    &self.symbols_by_guid,
                                    &extra_index,
                                    20,
                                )
                            }
                        }
                        None => find_decl_by_name(
                            *usage_symbol,
                            &self.path_by_symbols,
                            &self.symbols_by_guid,
                            &extra_index,
                            20,
                        )
                    };
                    symbols_cache.insert(guids_pair, decl_guid.clone());
                    decl_guid
                } else {
                    symbols_cache.get(&guids_pair).cloned().unwrap_or_default()
                };
                match decl_guid {
                    Some(guid) => {
                        {
                            usage_symbol
                                .write()
                                .expect("the data might be broken")
                                .set_linked_decl_guid(Some(guid))
                        }
                        stats.found += 1;
                    }
                    None => {
                        stats.non_found += 1;
                    }
                }
            }
            depth += 1;
        }
        stats
    }

    pub(crate) fn create_extra_indexes(&mut self, symbols: &Vec<AstSymbolInstanceArc>) {
        for symbol in symbols
            .iter()
            .filter(|s| !s.read().expect("the data might be broken").is_type())
            .cloned() {
            let (s_guid, mut types, is_declaration) = {
                let s_ref = symbol.read().expect("the data might be broken");
                (s_ref.guid().to_string(), s_ref.types(), s_ref.is_declaration())
            };
            types = if is_declaration {
                types
            } else {
                symbol.read().expect("the data might be broken")
                    .get_linked_decl_guid()
                    .clone()
                    .map(|guid| self.symbols_by_guid.get(&guid))
                    .flatten()
                    .map(|s| s.read().expect("the data might be broken").types())
                    .unwrap_or_default()
            };
            for guid in types
                .iter()
                .filter_map(|t| t.guid.clone()) {
                self.type_guid_to_dependand_guids.entry(guid).or_default().push(s_guid.clone());
            }
        }
    }

    pub(crate) async fn force_reindex(&mut self) {
        let symbols = self
            .symbols_by_guid()
            .values()
            .cloned()
            .collect::<Vec<_>>();
        self.resolve_types(&symbols).await;
        self.merge_usages_to_declarations(&symbols).await;
        self.create_extra_indexes(&symbols);
        self.has_changes = false;
    }

    async fn parse_single_file(
        &self,
        doc: &Document,
        code: &str,
    ) -> Vec<AstSymbolInstanceArc> {
        // This function runs to find symbols near cursor
        let mut doc = doc.clone();
        doc.update_text(&code.to_string());
        let symbols = AstIndex::parse(&doc).unwrap_or_default();
        self.resolve_types(&symbols).await;
        self.merge_usages_to_declarations(&symbols).await;
        // for s in symbols.iter() {
        //     let x = s.read().unwrap();
        //     info!("symbol {:?} {:?}", x.name(), x.symbol_type());
        // }
        symbols
    }
}

pub fn read_file_from_disk_block(path: &PathBuf) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read file from disk: {}", e))
}

