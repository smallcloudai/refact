use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use itertools::Itertools;
use rayon::prelude::*;
use ropey::Rope;
use sorted_vec::SortedVec;
use strsim::jaro_winkler;
use tracing::info;
use tree_sitter::Point;
use uuid::Uuid;

use crate::ast::comments_wrapper::get_language_id_by_filename;
use crate::ast::structs::FileASTMarkup;
use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstanceArc, read_symbol, SymbolInformation};

use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::get_ast_parser_by_filename;
use crate::ast::treesitter::structs::SymbolType;
use crate::ast::usages_declarations_merger::{FilePathIterator, find_decl_by_name, find_decl_by_caller_guid};
use crate::files_in_workspace::Document;


#[derive(Debug)]
pub struct AstIndex {
    symbols_by_name: HashMap<String, Vec<AstSymbolInstanceArc>>,
    symbols_by_guid: HashMap<Uuid, AstSymbolInstanceArc>,
    path_by_symbols: HashMap<PathBuf, Vec<AstSymbolInstanceArc>>,
    type_guid_to_dependent_guids: HashMap<Uuid, HashSet<Uuid>>,
    declaration_guid_to_usage_names: HashMap<Uuid, HashSet<String>>,
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


impl AstIndex {
    pub fn init() -> AstIndex {
        AstIndex {
            symbols_by_name: HashMap::new(),
            symbols_by_guid: HashMap::new(),
            path_by_symbols: HashMap::new(),
            type_guid_to_dependent_guids: HashMap::new(),
            declaration_guid_to_usage_names: HashMap::new(),
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
        let text = doc.text.clone().unwrap().to_string();
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

    pub async fn add_or_update_symbols_index(
        &mut self,
        doc: &Document,
        symbols: &Vec<AstSymbolInstanceArc>,
        make_dirty: bool,
    ) -> Result<(), String> {
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
            self.has_changes = make_dirty;
        }

        let mut symbol_names: SortedVec<String> = SortedVec::new();
        for symbol in symbols.iter() {
            let symbol_ref = read_symbol(symbol);
            self.symbols_by_name.entry(symbol_ref.name().to_string()).or_insert_with(Vec::new).push(symbol.clone());
            self.symbols_by_guid.insert(symbol_ref.guid().clone(), symbol.clone());
            self.path_by_symbols.entry(doc.path.clone()).or_insert_with(Vec::new).push(symbol.clone());
            symbol_names.push(symbol_ref.name().to_string());
        }

        Ok(())
    }

    pub async fn add_or_update(&mut self, doc: &Document, make_dirty: bool) -> Result<(), String> {
        let symbols = AstIndex::parse(doc)?;
        self.add_or_update_symbols_index(doc, &symbols, make_dirty).await
    }

    pub fn remove(&mut self, doc: &Document) -> bool {
        let symbols = self.path_by_symbols.remove(&doc.path);
        let has_removed = symbols.is_some();
        if !has_removed {
            return false;
        }
        let mut removed_guids = HashSet::new();
        for symbol in symbols
            .unwrap_or_default()
            .iter() {
            let (name, guid) = {
                let symbol_ref = read_symbol(symbol);
                (symbol_ref.name().to_string(), symbol_ref.guid().clone())
            };
            self.symbols_by_name
                .entry(name)
                .and_modify(|v| {
                    v.retain(|s| *read_symbol(s).guid() != guid);
                });

            self.symbols_by_guid.remove(&guid);
            if self.type_guid_to_dependent_guids.contains_key(&guid) {
                // TODO: we should do the removing more precisely,
                // some leftovers still are in the values, but it doesn't break the overall thing for now
                self.type_guid_to_dependent_guids.remove(&guid);
            }
            if self.declaration_guid_to_usage_names.contains_key(&guid) {
                // TODO: we should do the removing more precisely,
                // some leftovers still are in the values, but it doesn't break the overall thing for now
                self.declaration_guid_to_usage_names.remove(&guid);
            }
            removed_guids.insert(guid.clone());
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
        self.type_guid_to_dependent_guids.clear();
        self.declaration_guid_to_usage_names.clear();
        self.has_changes = true;
    }

    pub fn search_by_name(
        &self,
        query: &str,
        request_symbol_type: RequestSymbolType,
        exception_doc: Option<Document>,
        language: Option<LanguageId>,
        try_fuzzy_if_not_found: bool,
    ) -> Result<Vec<AstSymbolInstanceArc>, String> {
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
                    let s_ref = read_symbol(s);
                    match request_symbol_type {
                        RequestSymbolType::Declaration => s_ref.is_declaration(),
                        RequestSymbolType::Usage => !s_ref.is_declaration(),
                        RequestSymbolType::All => true,
                    }
                })
                .collect()
        }

        fn fuzzy_search(
            symbols_by_name: &HashMap<String, Vec<AstSymbolInstanceArc>>,
            query: &str,
            request_symbol_type: &RequestSymbolType,
        ) -> Vec<AstSymbolInstanceArc> {
            let lower_query = query.to_lowercase();
            symbols_by_name
                .par_iter()
                .filter(|(name, _)| {
                    let lower_name = name.to_lowercase();
                    lower_name.contains(&lower_query)
                })
                .map(|(_, symbols)| symbols.clone())
                .collect::<Vec<_>>()
                .iter()
                .flatten()
                .cloned()
                .filter(|s| {
                    let s_ref = read_symbol(s);
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
            symbols = fuzzy_search(&self.symbols_by_name, query, &request_symbol_type);
        }

        Ok(symbols
            .iter()
            .filter(|s| {
                let s_ref = read_symbol(s);
                let correct_doc = exception_doc.clone().map_or(true, |doc| doc.path != *s_ref.file_path());
                let correct_language = language.map_or(true, |l| l == *s_ref.language());
                correct_doc && correct_language
            })
            .map(|s| {
                let s_ref = read_symbol(s);
                (s, (jaro_winkler(query, s_ref.name()) as f32).max(f32::MIN_POSITIVE))
            })
            .sorted_by(|(_, dist_1), (_, dist_2)|
                dist_1.partial_cmp(dist_2).unwrap_or(std::cmp::Ordering::Equal)
            )
            .rev()
            .map(|(s, _)| s.clone())
            .collect::<Vec<_>>())
    }

    pub async fn search_by_content(
        &self,
        query: &str,
        request_symbol_type: RequestSymbolType,
        exception_doc: Option<Document>,
        language: Option<LanguageId>,
    ) -> Result<Vec<AstSymbolInstanceArc>, String> {
        Ok(self.path_by_symbols
            .par_iter()
            .filter(|(path, _symbols)| {
                let language_id = match get_language_id_by_filename(path) {
                    Some(lid) => lid,
                    None => return false,
                };
                let correct_language = language.map_or(true, |l| l == language_id);
                let correct_doc = exception_doc.clone().map_or(true, |doc| doc.path != **path);
                correct_doc && correct_language
            })
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
                    let s_ref = read_symbol(symbol);
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
                let is_declaration = read_symbol(s).is_declaration();
                match request_symbol_type {
                    RequestSymbolType::Declaration => is_declaration,
                    RequestSymbolType::Usage => !is_declaration,
                    RequestSymbolType::All => true,
                }
            })
            .collect::<Vec<_>>())
    }

    pub fn search_related_declarations(&self, guid: &Uuid) -> Result<Vec<AstSymbolInstanceArc>, String> {
        match self.symbols_by_guid.get(guid) {
            Some(symbol) => {
                Ok(read_symbol(symbol)
                    .types()
                    .iter()
                    .filter_map(|t| t.guid.clone())
                    .filter_map(|g| self.symbols_by_guid.get(&g))
                    .cloned()
                    .collect::<Vec<_>>())
            }
            _ => Ok(vec![])
        }
    }

    pub fn search_symbols_by_declarations_usage(
        &self,
        declaration_guid: &Uuid,
        exception_doc: Option<Document>,
    ) -> Result<Vec<AstSymbolInstanceArc>, String> {
        Ok(self.type_guid_to_dependent_guids
            .get(declaration_guid)
            .map(|x| x.clone())
            .unwrap_or_default()
            .iter()
            .filter_map(|guid| self.symbols_by_guid.get(guid))
            .filter(|s| {
                let s_ref = read_symbol(s);
                exception_doc.clone().map_or(true, |doc| doc.path != *s_ref.file_path())
            })
            .cloned()
            .collect::<Vec<_>>())
    }

    pub async fn retrieve_cursor_symbols_by_declarations(
        &self,
        doc: &Document,
        code: &str,
        cursor: Point,
        top_n_near_cursor: usize,
        top_n_usage_for_each_decl: usize,
    ) -> (Vec<AstSymbolInstanceArc>, Vec<AstSymbolInstanceArc>, Vec<AstSymbolInstanceArc>, Vec<AstSymbolInstanceArc>) {
        let file_symbols = self.parse_single_file(doc, code).await;
        let language = get_language_id_by_filename(&doc.path);

        let unfiltered_cursor_symbols = file_symbols
            .iter()
            .unique_by(|s| {
                let s_ref = read_symbol(s);
                (s_ref.guid().clone(), s_ref.name().to_string())
            })
            .filter(|s| !read_symbol(s).name().is_empty())
            .sorted_by_key(|a| read_symbol(a).distance_to_cursor(&cursor))
            .cloned()
            .collect::<Vec<_>>();

        let cursor_symbols_with_types = unfiltered_cursor_symbols
            .iter()
            .filter_map(|s| {
                let s_ref = read_symbol(s);
                if s_ref.is_declaration() {
                    Some(s)
                } else {
                    s_ref.get_linked_decl_guid()
                        .clone()
                        .map(|guid| self.symbols_by_guid.get(&guid))
                        .flatten()
                }
            })
            .unique_by(|s| read_symbol(s).guid().clone())
            .cloned()
            .collect::<Vec<_>>();

        let declarations_matched_by_name = unfiltered_cursor_symbols
            .iter()
            .map(|s| {
                let s_ref = read_symbol(s);
                let use_fuzzy_search = s_ref.full_range().start_point.row == cursor.row && s_ref.is_error();
                self.search_by_name(&s_ref.name(), RequestSymbolType::Declaration, None, language.clone(), use_fuzzy_search)
                    .unwrap_or_else(|_| vec![])
            })
            .flatten()
            .filter(|s| {
                let symbol_type = read_symbol(s).symbol_type();
                symbol_type == SymbolType::StructDeclaration
                    || symbol_type == SymbolType::TypeAlias
                    || symbol_type == SymbolType::FunctionDeclaration
            })
            .unique_by(|s| read_symbol(s).guid().clone())
            .unique_by(|s| read_symbol(s).name().to_string())
            .take(top_n_near_cursor)
            .collect::<Vec<_>>();
        let declarations = cursor_symbols_with_types
            .iter()
            .filter(|s| read_symbol(s).types().is_empty())
            .map(|s| {
                read_symbol(s)
                    .types()
                    .iter()
                    .filter_map(|t| t.guid.clone())
                    .filter_map(|g| self.symbols_by_guid.get(&g))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .flatten()
            .filter(|s| {
                let s_ref = read_symbol(s);
                *s_ref.language() == language.unwrap_or(*s_ref.language())
            })
            .chain(declarations_matched_by_name)
            .unique_by(|s| read_symbol(s).guid().clone())
            .unique_by(|s| read_symbol(s).name().to_string())
            .take(top_n_near_cursor)
            .collect::<Vec<_>>();

        let func_usages_matched_by_name = declarations
            .iter()
            .filter(|s| read_symbol(s).symbol_type() == SymbolType::FunctionDeclaration)
            .map(|s| {
                let (full_range, name) = {
                    let s_ref = read_symbol(s);
                    (s_ref.full_range().clone(), s_ref.name().to_string())
                };
                let use_fuzzy_search = full_range.start_point.row == cursor.row;
                self.search_by_name(&name, RequestSymbolType::Usage, None, language.clone(), use_fuzzy_search)
                    .unwrap_or_else(|_| vec![])
            })
            .flatten()
            .filter(|s| {
                read_symbol(s).symbol_type() == SymbolType::FunctionCall
            })
            .unique_by(|s| read_symbol(s).guid().clone())
            .unique_by(|s| read_symbol(s).name().to_string())
            .take(top_n_usage_for_each_decl)
            .collect::<Vec<_>>();
        let usages = declarations
            .iter()
            .map(|s| {
                let guid = read_symbol(s).guid().clone();
                let symbols_by_declarations = self.search_symbols_by_declarations_usage(&guid, None)
                    .unwrap_or_default()
                    .clone();
                symbols_by_declarations
                    .iter()
                    .sorted_by_key(|s| {
                        match read_symbol(s).symbol_type() {
                            SymbolType::ClassFieldDeclaration => 1,
                            SymbolType::VariableDefinition => 1,
                            SymbolType::FunctionDeclaration => 1,
                            SymbolType::FunctionCall => 2,
                            SymbolType::VariableUsage => 2,
                            _ => 0,
                        }
                    })
                    .take(top_n_usage_for_each_decl)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .flatten()
            .chain(func_usages_matched_by_name)
            .unique_by(|s| read_symbol(s).guid().clone())
            .unique_by(|s| read_symbol(s).name().to_string())
            .collect::<Vec<_>>();

        let cursor_symbols_names = unfiltered_cursor_symbols
            .iter()
            .filter(|s| {
                let symbol_type = read_symbol(s).symbol_type();
                symbol_type == SymbolType::FunctionCall
                    || symbol_type == SymbolType::VariableUsage
                    || symbol_type == SymbolType::VariableDefinition
                    || symbol_type == SymbolType::ClassFieldDeclaration
                    || symbol_type == SymbolType::CommentDefinition
            })
            .map(|s| {
                read_symbol(s).name().to_string()
            })
            .take(50)  // top 50 usages near the cursor
            .collect::<HashSet<_>>();
        let most_similar_declarations = self.declaration_guid_to_usage_names
            .par_iter()
            .map(|(decl_guid, usage_names)| {
                (
                    decl_guid,
                    cursor_symbols_names.intersection(usage_names).count(),
                )
            })
            .collect::<Vec<_>>()
            .iter()
            .filter(|&(_, a)| *a > (cursor_symbols_names.len() / 5))
            .take(top_n_near_cursor * 2)
            .filter_map(|(g, _)| self.symbols_by_guid.get(*g))
            .unique_by(|s| read_symbol(s).guid().clone())
            .unique_by(|s| read_symbol(s).name().to_string())
            .take(top_n_near_cursor)
            .cloned()
            .collect::<Vec<_>>();

        (
            unfiltered_cursor_symbols
                .iter()
                .unique_by(|s| read_symbol(s).name().to_string())
                .cloned()
                .collect::<Vec<_>>(),
            declarations,
            usages,
            most_similar_declarations,
        )
    }

    pub async fn file_markup(
        &self,
        doc: &Document,
    ) -> Result<FileASTMarkup, String> {
        assert!(doc.text.is_some());

        // let symbols: Vec<AstSymbolInstanceArc> = self.path_by_symbols
        // .get(&doc.path)
        // .map(|symbols| {
        //     symbols
        //         .iter()
        //         .filter(|s| read_symbol(s).is_declaration())
        //         .cloned()
        //         .collect()
        // })
        // .unwrap_or_default();

        let symbols = match self.path_by_symbols.get(&doc.path) {
            Some(x) => x.clone(),
            None => {
                info!("no symbols in index for {:?}, assuming it's a new file of some sort and parsing it", doc.path);
                let mut parser = match get_ast_parser_by_filename(&doc.path) {
                    Ok(parser) => parser,
                    Err(e) => {
                        return Err(format!("no symbols in index for {:?}, and cannot find a parser this kind of file: {}", doc.path, e.message));
                    }
                };
                let symbols = parser.parse(doc.text.as_ref().unwrap().to_string().as_str(), &doc.path);
                symbols
            }
        };
        crate::ast::ast_file_markup::lowlevel_file_markup(doc, &symbols).await
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
                        let s_ref = read_symbol(s);
                        match request_symbol_type {
                            RequestSymbolType::Declaration => s_ref.is_declaration(),
                            RequestSymbolType::Usage => !s_ref.is_declaration(),
                            RequestSymbolType::All => true,
                        }
                    })
                    .map(|s| read_symbol(s).symbol_info_struct())
                    .collect()
            })
            .unwrap_or_default();
        Ok(symbols)
    }

    #[allow(unused)]
    pub fn get_file_paths(&self) -> Vec<PathBuf> {
        self.path_by_symbols.iter().map(|(path, _)| path.clone()).collect()
    }

    pub fn get_symbols_names(
        &self,
        request_symbol_type: RequestSymbolType,
    ) -> Vec<String> {
        self.symbols_by_guid
            .iter()
            .filter(|(_guid, s)| {
                let s_ref = read_symbol(s);
                match request_symbol_type {
                    RequestSymbolType::Declaration => s_ref.is_declaration(),
                    RequestSymbolType::Usage => !s_ref.is_declaration(),
                    RequestSymbolType::All => true,
                }
            })
            .map(|(_guid, s)| read_symbol(s).name().to_string())
            .collect()
    }

    pub(crate) fn symbols_by_guid(&self) -> &HashMap<Uuid, AstSymbolInstanceArc> {
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
                let s_ref = read_symbol(symbol);
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
                            .filter(|s| read_symbol(s).is_type())
                            .sorted_by(|a, b| {
                                let path_a = read_symbol(a).file_path().clone();
                                let path_b = read_symbol(b).file_path().clone();
                                FilePathIterator::compare_paths(&symb_path, &path_a, &path_b)
                            })
                            .next()
                            .map(|s| read_symbol(s).guid().clone())
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
            guid_by_symbols: &HashMap<Uuid, AstSymbolInstanceArc>,
            current_depth: usize,
        ) -> usize {
            let caller_guid = match read_symbol(symbol)
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
        let extra_index: HashMap<(String, Uuid, String), AstSymbolInstanceArc> = symbols
            .iter()
            .map(|x| {
                let x_ref = read_symbol(x);
                ((x_ref.name().to_string(),
                  x_ref.parent_guid().clone().unwrap_or_default(),
                  x_ref.file_path().to_str().unwrap_or_default().to_string()),
                 x.clone())
            })
            .collect();
        let mut depth: usize = 0; // depth means "a.b.c" it's 2 for c
        loop {
            let symbols_to_process = symbols
                .iter()
                .filter(|symbol| {
                    read_symbol(symbol).get_linked_decl_guid().is_none()
                })
                .filter(|symbol| {
                    let s_ref = read_symbol(symbol);
                    let valid_depth = get_caller_depth(symbol, &self.symbols_by_guid, 0) == depth;
                    valid_depth && (s_ref.symbol_type() == SymbolType::FunctionCall
                        || s_ref.symbol_type() == SymbolType::VariableUsage)
                })
                .collect::<Vec<_>>();

            if symbols_to_process.is_empty() {
                break;
            }

            let mut symbols_cache: HashMap<(Uuid, String), Option<Uuid>> = HashMap::new();
            for (_, usage_symbol) in symbols_to_process
                .iter()
                .enumerate() {
                tokio::task::yield_now().await;
                let (name, parent_guid, caller_guid) = {
                    let s_ref = read_symbol(usage_symbol);
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
                                    1,
                                )
                            }
                        }
                        None => find_decl_by_name(
                            *usage_symbol,
                            &self.path_by_symbols,
                            &self.symbols_by_guid,
                            &extra_index,
                            1,
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
            .filter(|s| !read_symbol(s).is_type())
            .cloned() {
            let guid = read_symbol(&symbol).guid().clone();
            if self.type_guid_to_dependent_guids.contains_key(&guid) {
                self.type_guid_to_dependent_guids.remove(&guid);
            }
            if self.declaration_guid_to_usage_names.contains_key(&guid) {
                self.declaration_guid_to_usage_names.remove(&guid);
            }
        }

        for symbol in symbols
            .iter()
            .filter(|s| !read_symbol(s).is_type())
            .cloned() {
            let (name, s_guid, mut types, is_declaration, symbol_type, parent_guid) = {
                let s_ref = read_symbol(&symbol);
                (s_ref.name().to_string(), s_ref.guid().clone(), s_ref.types(), s_ref.is_declaration(),
                 s_ref.symbol_type(), s_ref.parent_guid().clone())
            };
            types = if is_declaration {
                types
            } else {
                read_symbol(&symbol)
                    .get_linked_decl_guid()
                    .clone()
                    .map(|guid| self.symbols_by_guid.get(&guid))
                    .flatten()
                    .map(|s| read_symbol(s).types())
                    .unwrap_or_default()
            };
            for guid in types
                .iter()
                .filter_map(|t| t.guid.clone()) {
                self.type_guid_to_dependent_guids.entry(guid).or_default().insert(s_guid.clone());
            }

            // for those symbols which doesn't have their own scope
            if symbol_type == SymbolType::FunctionCall
                || symbol_type == SymbolType::VariableUsage
                || symbol_type == SymbolType::VariableDefinition
                || symbol_type == SymbolType::ClassFieldDeclaration
                || symbol_type == SymbolType::CommentDefinition {
                if let Some(p_guid) = parent_guid {
                    self.declaration_guid_to_usage_names.entry(p_guid).or_default().insert(name.clone());
                }
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

