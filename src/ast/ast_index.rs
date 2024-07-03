use std::cell::RefCell;
use std::collections::HashMap as StdHashMap;
use std::hash::Hash;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use hashbrown::{HashMap, HashSet};
use itertools::Itertools;
use rand::Rng;
use rayon::prelude::*;
use ropey::Rope;
use strsim::jaro_winkler;
use tracing::info;
use tree_sitter::Point;
use uuid::Uuid;

use crate::ast::comments_wrapper::get_language_id_by_filename;
use crate::ast::imports_resolver::{possible_filepath_candidates, top_n_prefixes, try_find_file_path};
use crate::ast::structs::FileASTMarkup;
use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstance, AstSymbolInstanceArc, AstSymbolInstanceRc, ImportDeclaration, ImportType, SymbolInformation};
use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::get_ast_parser_by_filename;
use crate::ast::treesitter::structs::SymbolType;
use crate::ast::usages_declarations_merger::{FilePathIterator, find_decl_by_caller_guid, find_decl_by_name};
use crate::files_in_workspace::Document;

const TOO_MANY_SYMBOLS_IN_FILE: usize = 10000;


#[derive(Debug)]
pub struct AstIndex {
    shutdown_flag: Arc<AtomicBool>,
    declaration_symbols_by_name: HashMap<String, Vec<AstSymbolInstanceRc>>,
    declaration_symbols_by_fullpath: HashMap<String, Vec<AstSymbolInstanceRc>>,
    usage_symbols_by_name: HashMap<String, Vec<AstSymbolInstanceRc>>,
    usage_symbols_by_fullpath: HashMap<String, Vec<AstSymbolInstanceRc>>,
    symbols_by_guid: HashMap<Uuid, AstSymbolInstanceRc>,
    path_by_symbols: HashMap<PathBuf, Vec<AstSymbolInstanceRc>>,
    type_guid_to_dependent_guids: HashMap<Uuid, HashSet<Uuid>>,
    declaration_guid_to_usage_names: StdHashMap<Uuid, HashSet<String>>,
    import_components_succ_solution_index: HashMap<String, ImportDeclaration>,
    ast_index_max_files: usize,
    has_changes: bool,
    ast_light_mode: bool,
}

unsafe impl Send for AstIndex {}

unsafe impl Sync for AstIndex {}


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
    pub fn init(
        ast_index_max_files: usize,
        shutdown_flag: Arc<AtomicBool>,
        ast_light_mode: bool,
    ) -> AstIndex {
        AstIndex {
            shutdown_flag,
            declaration_symbols_by_name: HashMap::new(),
            declaration_symbols_by_fullpath: HashMap::new(),
            usage_symbols_by_name: HashMap::new(),
            usage_symbols_by_fullpath: HashMap::new(),
            symbols_by_guid: HashMap::new(),
            path_by_symbols: HashMap::new(),
            type_guid_to_dependent_guids: HashMap::new(),
            declaration_guid_to_usage_names: StdHashMap::new(),
            import_components_succ_solution_index: HashMap::new(),
            ast_index_max_files,
            has_changes: false,
            ast_light_mode,
        }
    }

    pub fn parse(doc: &Document) -> Result<Vec<AstSymbolInstanceArc>, String> {
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
        if symbol_instances.len() > TOO_MANY_SYMBOLS_IN_FILE {
            info!(
                "parsed {}, {} symbols, took {:.3}s to parse, skip",
                crate::nicer_logs::last_n_chars(&doc.path.display().to_string(), 30),
                symbol_instances.len(),
                t_elapsed.as_secs_f32()
            );
            return Err("too many symbols, assuming generated file".to_string());
        } else {
            info!(
                "parsed {}, {} symbols, took {:.3}s to parse",
                crate::nicer_logs::last_n_chars(&doc.path.display().to_string(), 30),
                symbol_instances.len(),
                t_elapsed.as_secs_f32()
            );
        }
        Ok(symbol_instances)
    }

    pub fn add_or_update_symbols_index(
        &mut self,
        doc: &Document,
        symbols: Vec<AstSymbolInstanceArc>,
        make_dirty: bool,
    ) -> Result<(), String> {
        if self.is_overflowed() {
            info!(
                "Too many files in the ast index ({} >= {}), skipping the {}",
                self.path_by_symbols.len(),
                self.ast_index_max_files,
                crate::nicer_logs::last_n_chars(&doc.path.display().to_string(), 30)
            );
            return Err("ast index too many files".to_string());
        }
        let symbols_filtered = if self.ast_light_mode {
            symbols
                .iter()
                .filter(|s| {
                    let symbol_type = s.read().symbol_type();
                    let is_reference_type = symbol_type == SymbolType::VariableUsage
                        || symbol_type == SymbolType::FunctionCall
                        || symbol_type == SymbolType::VariableDefinition;
                    !is_reference_type
                })
                .cloned()
                .collect::<Vec<_>>()
        } else {
            symbols
        };
        let mut symbols_cloned = symbols_filtered
            .iter()
            .map(|sym| {
                let mut write_lock = sym.write();
                Rc::new(RefCell::new(std::mem::replace(&mut *write_lock, Box::new(ImportDeclaration::default()))))
            }).collect::<Vec<_>>();
        let has_changes_before = self.has_changes;
        let has_removed = self.remove(&doc);
        if has_removed {
            self.resolve_declaration_symbols(&mut symbols_cloned);
            let (_, import_components_succ_solution_index) = self.resolve_imports(
                &mut symbols_cloned, &self.import_components_succ_solution_index,
            );
            self.import_components_succ_solution_index.extend(import_components_succ_solution_index);
            self.merge_usages_to_declarations(&mut symbols_cloned);
            self.create_extra_indexes(&mut symbols_cloned);
            self.has_changes = has_changes_before;
        } else {
            // TODO: we don't want to update the whole index for a single file
            // even if we might miss some new cross-references
            // later we should think about some kind of force update, ie once in a while self.has_changes=false
            self.has_changes = make_dirty;
        }

        for symbol in symbols_cloned.iter() {
            let symbol_ref = symbol.borrow();
            if symbol_ref.is_declaration() {
                self.declaration_symbols_by_name.entry(symbol_ref.name().to_string()).or_insert_with(Vec::new).push(symbol.clone());
            } else {
                self.usage_symbols_by_name.entry(symbol_ref.name().to_string()).or_insert_with(Vec::new).push(symbol.clone());
            }
            self.symbols_by_guid.insert(symbol_ref.guid().clone(), symbol.clone());
            self.path_by_symbols.entry(doc.path.clone()).or_insert_with(Vec::new).push(symbol.clone());
        }

        Ok(())
    }

    pub fn add_or_update(&mut self, doc: &Document, make_dirty: bool) -> Result<usize, String> {
        let symbols = AstIndex::parse(doc)?;
        let symbols_len = symbols.len();
        match self.add_or_update_symbols_index(doc, symbols, make_dirty) {
            Ok(_) => Ok(symbols_len),
            Err(e) => Err(e)
        }
    }

    pub fn remove(&mut self, doc: &Document) -> bool {
        // TODO:
        // We do not remove those guid (O(n) complexity):
        // - which are in the `declaration_symbols_by_name` and in the `usage_symbols_by_name` indexes
        // - `dependent_guids` in the `type_guid_to_dependent_guids` index
        // - `linked_guids` in the all TypeDefs (inside all symbols)

        let symbols = self.path_by_symbols.remove(&doc.path);
        let has_removed = symbols.is_some();
        if !has_removed {
            return false;
        }
        let mut removed_guids = HashSet::new();
        for symbol in symbols
            .unwrap_or_default()
            .iter() {
            let guid = symbol.borrow().guid().clone();
            self.symbols_by_guid.remove(&guid);
            self.type_guid_to_dependent_guids.remove(&guid);
            self.declaration_guid_to_usage_names.remove(&guid);
            removed_guids.insert(guid.clone());
        }

        self.has_changes = true;
        has_removed
    }

    pub fn clear_index(&mut self) {
        self.declaration_symbols_by_name.clear();
        self.declaration_symbols_by_fullpath.clear();
        self.usage_symbols_by_name.clear();
        self.usage_symbols_by_fullpath.clear();
        self.symbols_by_guid.clear();
        self.path_by_symbols.clear();
        self.type_guid_to_dependent_guids.clear();
        self.declaration_guid_to_usage_names.clear();
        self.has_changes = true;
    }

    fn search_by_name_or_path_base(
        &self,
        declaration_symbols_index: &HashMap<String, Vec<AstSymbolInstanceRc>>,
        usage_symbols_index: &HashMap<String, Vec<AstSymbolInstanceRc>>,
        query: &str,
        request_symbol_type: RequestSymbolType,
        exception_doc: Option<Document>,
        language: Option<LanguageId>,
        try_fuzzy_if_not_found: bool,
        sort_results: bool,
    ) -> Result<Vec<AstSymbolInstanceRc>, String> {
        fn exact_search(
            symbols_by_name: &HashMap<String, Vec<AstSymbolInstanceRc>>,
            query: &str,
        ) -> Vec<AstSymbolInstanceRc> {
            let binding = vec![];
            let symbols = symbols_by_name.get(query).unwrap_or(&binding);
            symbols.clone()
        }

        fn fuzzy_search(
            symbols_by_name: &HashMap<String, Vec<AstSymbolInstanceRc>>,
            query: &str,
        ) -> Vec<AstSymbolInstanceRc> {
            let lower_query = query.to_lowercase();
            symbols_by_name
                .iter()
                .filter(|(name, _)| {
                    let lower_name = name.to_lowercase();
                    lower_name.contains(&lower_query)
                })
                .map(|(_, symbols)| symbols.clone())
                .collect::<Vec<_>>()
                .iter()
                .flatten()
                .cloned()
                .collect()
        }

        let symbols = match request_symbol_type {
            RequestSymbolType::Declaration => {
                let mut symbols = exact_search(&declaration_symbols_index, query);
                if try_fuzzy_if_not_found && symbols.is_empty() {
                    symbols = fuzzy_search(&declaration_symbols_index, query);
                }
                symbols
            }
            RequestSymbolType::Usage => {
                let mut symbols = exact_search(&usage_symbols_index, query);
                if try_fuzzy_if_not_found && symbols.is_empty() {
                    symbols = fuzzy_search(&usage_symbols_index, query);
                }
                symbols
            }
            RequestSymbolType::All => {
                let mut symbols = exact_search(&declaration_symbols_index, query);
                symbols.extend(exact_search(&usage_symbols_index, query));
                if try_fuzzy_if_not_found && symbols.is_empty() {
                    symbols = fuzzy_search(&declaration_symbols_index, query);
                    symbols.extend(fuzzy_search(&usage_symbols_index, query));
                }
                symbols
            }
        };

        let symbols_it = symbols
            .iter()
            .filter(|s| {
                let s_ref = s.borrow();
                let correct_doc = exception_doc.clone().map_or(true, |doc| doc.path != *s_ref.file_path());
                let correct_language = language.map_or(true, |l| l == *s_ref.language());
                correct_doc && correct_language
            });

        if sort_results {
            Ok(symbols_it
                .map(|s| {
                    let s_ref = s.borrow();
                    (s, (jaro_winkler(query, s_ref.name()) as f32).max(f32::MIN_POSITIVE))
                })
                .sorted_by(|(_, dist_1), (_, dist_2)|
                    dist_1.partial_cmp(dist_2).unwrap_or(std::cmp::Ordering::Equal)
                )
                .rev()
                .map(|(s, _)| s.clone())
                .collect::<Vec<_>>())
        } else {
            Ok(symbols_it.cloned().collect::<Vec<_>>())
        }
    }

    pub(crate) fn search_by_name(
        &self,
        query: &str,
        request_symbol_type: RequestSymbolType,
        exception_doc: Option<Document>,
        language: Option<LanguageId>,
        try_fuzzy_if_not_found: bool,
        sort_results: bool,
    ) -> Result<Vec<AstSymbolInstanceRc>, String> {
        self.search_by_name_or_path_base(
            &self.declaration_symbols_by_name,
            &self.usage_symbols_by_name,
            query,
            request_symbol_type,
            exception_doc,
            language,
            try_fuzzy_if_not_found,
            sort_results,
        )
    }

    pub(crate) fn search_by_fullpath(
        &self,
        query: &str,
        request_symbol_type: RequestSymbolType,
        exception_doc: Option<Document>,
        language: Option<LanguageId>,
        try_fuzzy_if_not_found: bool,
        sort_results: bool,
    ) -> Result<Vec<AstSymbolInstanceRc>, String> {
        self.search_by_name_or_path_base(
            &self.declaration_symbols_by_fullpath,
            &self.usage_symbols_by_fullpath,
            query,
            request_symbol_type,
            exception_doc,
            language,
            try_fuzzy_if_not_found,
            sort_results,
        )
    }

    pub(crate) fn search_by_content(
        &self,
        query: &str,
        request_symbol_type: RequestSymbolType,
        exception_doc: Option<Document>,
        language: Option<LanguageId>,
    ) -> Result<Vec<AstSymbolInstanceRc>, String> {
        Ok(self.path_by_symbols
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
                    let s_ref = symbol.borrow();
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
                let is_declaration = s.borrow().is_declaration();
                match request_symbol_type {
                    RequestSymbolType::Declaration => is_declaration,
                    RequestSymbolType::Usage => !is_declaration,
                    RequestSymbolType::All => true,
                }
            })
            .collect::<Vec<_>>())
    }

    pub(crate) fn search_related_declarations(&self, guid: &Uuid) -> Result<Vec<AstSymbolInstanceRc>, String> {
        match self.symbols_by_guid.get(guid) {
            Some(symbol) => {
                Ok(symbol.borrow()
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

    pub(crate) fn search_usages_with_this_declaration(
        &self,
        declaration_guid: &Uuid,
        exception_doc: Option<Document>,
    ) -> Result<Vec<AstSymbolInstanceRc>, String> {
        Ok(self.type_guid_to_dependent_guids
            .get(declaration_guid)
            .map(|x| x.clone())
            .unwrap_or_default()
            .iter()
            .filter_map(|guid| self.symbols_by_guid.get(guid))
            .filter(|s| {
                let s_ref = s.borrow();
                exception_doc.clone().map_or(true, |doc| doc.path != *s_ref.file_path())
            })
            .cloned()
            .collect::<Vec<_>>())
    }

    pub(crate) fn get_symbol_full_path(
        &self,
        symbol: &AstSymbolInstanceRc,
    ) -> String {
        let mut current_symbol = symbol.clone();
        let mut current_path = current_symbol.borrow().name().to_string();

        loop {
            let parent_guid = match current_symbol.borrow().parent_guid().clone() {
                Some(g) => g,
                None => {
                    return current_path;
                }
            };
            match self.symbols_by_guid.get(&parent_guid) {
                Some(s) => {
                    current_symbol = s.clone();
                    if !s.borrow().name().is_empty() {
                        current_path = format!("{}::{}", s.borrow().name(), current_path);
                    }
                }
                None => {
                    return current_path;
                }
            }
        }
    }

    fn get_declarations_by_parent(
        &self,
        symbol: &AstSymbolInstanceRc,
        base_usefulness: f32,
        symbols_by_guid: &HashMap<Uuid, AstSymbolInstanceRc>
    ) -> (Vec<AstSymbolInstanceRc>, HashMap<Uuid, f32>) {
        let mut current_symbol = symbol.clone();
        let mut parents_symbols: Vec<AstSymbolInstanceRc> = vec![];
        let mut guid_to_usefulness: HashMap<Uuid, f32> = HashMap::new();
        let mut level: u64 = 0;
        loop {
            let parent_guid = current_symbol.borrow().parent_guid().unwrap_or_default();
            if let Some(parent_symbol) = symbols_by_guid.get(&parent_guid) {
                parents_symbols.extend(
                    parent_symbol
                        .borrow()
                        .types()
                        .iter()
                        .filter_map(|t| t.guid.clone())
                        .filter_map(|g| match self.symbols_by_guid.get(&g) {
                            None => symbols_by_guid.get(&g),
                            Some(item) => Some(item)
                        })
                        .cloned()
                        .map(|s| {
                            *guid_to_usefulness
                                .entry(s.borrow().guid().clone())
                                .or_insert_with(|| base_usefulness) -= 10.0 * level as f32;
                            s
                        })
                        .collect::<Vec<_>>()
                );
                current_symbol = parent_symbol.clone();
                level += 1
            } else {
                break;
            }
        }
        (parents_symbols, guid_to_usefulness)
    }

    pub(crate) fn symbols_near_cursor_to_buckets(
        &self,
        doc: &Document,
        code: &str,
        cursor: Point,
        top_n_near_cursor: usize,
        top_n_usage_for_each_decl: usize,
        fuzzy_search_limit: usize,
    ) -> (
        Vec<AstSymbolInstanceRc>,
        Vec<AstSymbolInstanceRc>,
        Vec<AstSymbolInstanceRc>,
        Vec<AstSymbolInstanceRc>,
        Vec<AstSymbolInstanceRc>,
        HashMap<Uuid, f32>
    ) {
        let t_parse_t0 = std::time::Instant::now();
        let file_symbols = self.parse_single_file(doc, code);
        let language = get_language_id_by_filename(&doc.path);
        let t_parse_ms = t_parse_t0.elapsed().as_millis() as i32;

        let mut guid_to_usefulness: HashMap<Uuid, f32> = HashMap::new();
        let t_cursor_t0 = std::time::Instant::now();
        let unfiltered_cursor_symbols = file_symbols
            .iter()
            .filter(|s| !s.borrow().name().is_empty())
            .sorted_unstable_by_key(|a| a.borrow().distance_to_cursor(&cursor))
            .map(|s| {
                *guid_to_usefulness
                    .entry(s.borrow().guid().clone())
                    .or_default() = 0.0;
                s
            })
            .cloned()
            .collect::<Vec<_>>();
        let scope_symbols = if let Some(parent_guid) = unfiltered_cursor_symbols
            .iter()
            .next()
            .map(|s| s.borrow().parent_guid().clone())
            .flatten() {
            unfiltered_cursor_symbols
                .iter()
                .filter(|s| s.borrow().parent_guid().map_or(true, |g| g == parent_guid))
                .cloned()
                .collect::<Vec<_>>()
        } else {
            vec![]
        };

        let cursor_symbols_with_types = unfiltered_cursor_symbols
            .iter()
            .filter_map(|s| {
                let s_ref = s.borrow();
                if s_ref.is_declaration() {
                    Some(s)
                } else {
                    s_ref.get_linked_decl_guid()
                        .clone()
                        .map(|guid| self.symbols_by_guid.get(&guid))
                        .flatten()
                }
            })
            .unique_by(|s| s.borrow().guid().clone())
            .cloned()
            .collect::<Vec<_>>();
        let t_cursor_ms = t_cursor_t0.elapsed().as_millis() as i32;

        let t_decl_t0 = std::time::Instant::now();
        let mut decl_fuzzy_count = 0;
        let declarations_matched_by_name = unfiltered_cursor_symbols
            .iter()
            .map(|s| {
                let s_ref = s.borrow();
                let mut use_fuzzy_search = s_ref.full_range().start_point.row == cursor.row && s_ref.is_error();
                decl_fuzzy_count += if use_fuzzy_search { 1 } else { 0 };
                if decl_fuzzy_count >= fuzzy_search_limit { use_fuzzy_search = false; }
                self.search_by_name(&s_ref.name(), RequestSymbolType::Declaration, None, language.clone(), use_fuzzy_search, false)
                    .unwrap_or_else(|_| vec![])
            })
            .flatten()
            .filter(|s| {
                let symbol_type = s.borrow().symbol_type();
                symbol_type == SymbolType::StructDeclaration
                    || symbol_type == SymbolType::TypeAlias
                    || symbol_type == SymbolType::FunctionDeclaration
            })
            .unique_by(|s| s.borrow().guid().clone())
            .unique_by(|s| s.borrow().name().to_string())
            .take(top_n_near_cursor)
            .map(|s| {
                *guid_to_usefulness
                    .entry(s.borrow().guid().clone())
                    .or_default() += 70.0;
                s
            })
            .collect::<Vec<_>>();
        let t_decl_ms = t_decl_t0.elapsed().as_millis() as i32;

        // use usage symbol's parents to get definition of extra types (template types, signature types, parent classes, ...)
        let local_symbols_by_guid = file_symbols.iter().map(|s| (s.borrow().guid().clone(), s.clone())).collect::<HashMap<_, _>>();
        let (declarations_matched_by_parent, guid_to_usefulness_to_merge) = if let Some(symbol) = unfiltered_cursor_symbols
            .iter()
            .filter(|s| !s.borrow().is_declaration())
            .next() {
            self.get_declarations_by_parent(symbol, 70.0, &local_symbols_by_guid)
        } else {
            (vec![], HashMap::new())
        };
        guid_to_usefulness_to_merge
            .iter()
            .for_each(|(guid, usefulness)| {
                *guid_to_usefulness
                    .entry(guid.clone())
                    .or_default() += *usefulness;
            });


        // (3) cursor_symbols_with_types + declarations_matched_by_name
        let t_stage3_t0 = std::time::Instant::now();
        let declarations = cursor_symbols_with_types
            .iter()
            .filter(|s| s.borrow().types().is_empty())
            .map(|s| {
                s.borrow()
                    .types()
                    .iter()
                    .filter_map(|t| t.guid.clone())
                    .filter_map(|g| self.symbols_by_guid.get(&g))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .flatten()
            .filter(|s| {
                let s_ref = s.borrow();
                *s_ref.language() == language.unwrap_or(*s_ref.language())
            })
            .map(|s| {
                *guid_to_usefulness
                    .entry(s.borrow().guid().clone())
                    .or_default() += 90.0;
                s
            })
            .chain(declarations_matched_by_name)
            .chain(declarations_matched_by_parent)
            .unique_by(|s| s.borrow().guid().clone())
            .unique_by(|s| s.borrow().name().to_string())
            .take(top_n_near_cursor)
            .collect::<Vec<_>>();
        let t_stage3_ms = t_stage3_t0.elapsed().as_millis() as i32;

        // (5) Match function calls by name, with fuzzy search on the current line
        let t_stage5_t0 = std::time::Instant::now();
        let mut stage5_fuzzy_count = 0;
        let func_calls_matched_by_name = declarations
            .iter()
            .filter(|s| s.borrow().symbol_type() == SymbolType::FunctionDeclaration)
            .map(|s| {
                let (full_range, name) = {
                    let s_ref = s.borrow();
                    (s_ref.full_range().clone(), s_ref.name().to_string())
                };
                let mut use_fuzzy_search = full_range.start_point.row == cursor.row;
                stage5_fuzzy_count += if use_fuzzy_search { 1 } else { 0 };
                if decl_fuzzy_count + stage5_fuzzy_count >= fuzzy_search_limit { use_fuzzy_search = false; }
                self.search_by_name(&name, RequestSymbolType::Usage, None, language.clone(), use_fuzzy_search, false)
                    .unwrap_or_else(|_| vec![])
            })
            .flatten()
            .filter(|s| {
                s.borrow().symbol_type() == SymbolType::FunctionCall
            })
            .unique_by(|s| s.borrow().guid().clone())
            .unique_by(|s| s.borrow().name().to_string())
            .take(top_n_usage_for_each_decl)
            .map(|s| {
                *guid_to_usefulness
                    .entry(s.borrow().guid().clone())
                    .or_default() += 40.0;
                s
            })
            .collect::<Vec<_>>();
        let t_stage5_ms = t_stage5_t0.elapsed().as_millis() as i32;

        // (4) Find anything (especially FunctionCall, VariableUsage) that uses the same declarations (list from step 3, matched by guid)
        let t_stage4_t0 = std::time::Instant::now();
        let usages = declarations
            .iter()
            .map(|decl_s| {
                let guid = decl_s.borrow().guid().clone();
                let symbols_by_declarations = self.search_usages_with_this_declaration(&guid, None)
                    .unwrap_or_default()
                    .clone();
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
                    .take(top_n_usage_for_each_decl)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .flatten()
            .map(|s| {
                *guid_to_usefulness
                    .entry(s.borrow().guid().clone())
                    .or_default() += 50.0;
                s
            })
            .chain(func_calls_matched_by_name)
            .unique_by(|s| s.borrow().guid().clone())
            .unique_by(|s| s.borrow().name().to_string())
            .collect::<Vec<_>>();
        let t_stage4_ms = t_stage4_t0.elapsed().as_millis() as i32;

        // (6) Detect declarations with high symbols overlap (compile cursor_symbols_names first)
        let t_stage6_t0 = std::time::Instant::now();
        let cursor_symbols_names = scope_symbols
            .iter()
            .map(|s| {
                s.borrow().name().to_string()
            })
            .collect::<HashSet<_>>();
        let high_overlap_declarations = self.declaration_guid_to_usage_names
            .par_iter()
            .map(|(decl_guid, usage_names)| {
                (
                    decl_guid,
                    cursor_symbols_names.intersection(usage_names).count(),
                )
            })
            .collect::<Vec<_>>()
            .iter()
            .sorted_unstable_by_key(|a| a.1)
            .filter(|&(_, a)| *a > (cursor_symbols_names.len() / 5))
            .take(top_n_near_cursor * 2)
            .filter_map(|(g, _)| self.symbols_by_guid.get(*g))
            .unique_by(|s| s.borrow().guid().clone())
            .unique_by(|s| s.borrow().name().to_string())
            .take(top_n_near_cursor)
            .map(|s| {
                *guid_to_usefulness
                    .entry(s.borrow().guid().clone())
                    .or_default() += 35.0;
                s
            })
            .cloned()
            .collect::<Vec<_>>();
        let t_stage6_ms = t_stage6_t0.elapsed().as_millis() as i32;

        // (7) declarations from imported files
        let t_stage7_t0 = std::time::Instant::now();
        let bucket_imports = self
            .decl_symbols_from_imports(&file_symbols, 0)
            .into_iter()
            .map(|s| {
                *guid_to_usefulness
                    .entry(s.borrow().guid().clone())
                    .or_default() += 15.0;
                s
            })
            .collect::<Vec<_>>();
        let t_stage7_ms = t_stage7_t0.elapsed().as_millis() as i32;

        info!(
            "\t_parse={t_parse_ms}ms symbols_near_cursor_to_buckets -> t_cursor={t_cursor_ms}ms \
            t_decl={t_decl_ms}ms ({decl_fuzzy_count} fuzzy_req) \
            t_stage3={t_stage3_ms}ms t_stage5={t_stage5_ms}ms ({stage5_fuzzy_count} fuzzy_req) \
            t_stage4={t_stage4_ms}ms t_stage6={t_stage6_ms}ms t_stage7={t_stage7_ms}ms"
        );

        (
            unfiltered_cursor_symbols
                .iter()
                .unique_by(|s| s.borrow().name().to_string())
                .cloned()
                .collect::<Vec<_>>(),
            declarations,
            usages,
            high_overlap_declarations,
            bucket_imports,
            guid_to_usefulness,
        )
    }

    pub(crate) fn decl_symbols_from_imports(
        &self,
        parsed_symbols: &Vec<AstSymbolInstanceRc>,
        imports_depth: usize,
    ) -> Vec<AstSymbolInstanceRc> {
        let mut paths: Vec<PathBuf> = vec![];
        let mut current_depth_symbols = parsed_symbols.clone();
        let mut current_depth = 0;
        loop {
            if current_depth > imports_depth { break; }
            let mut current_paths = vec![];
            for symbol in current_depth_symbols
                .iter()
                .filter(|s| s.borrow().symbol_type() == SymbolType::ImportDeclaration) {
                let s_ref = symbol.borrow();
                let import_decl = s_ref.as_any().downcast_ref::<ImportDeclaration>().expect("wrong type");
                if let Some(import_path) = import_decl.filepath_ref.clone() {
                    current_paths.push(import_path.clone());
                    paths.push(import_path);
                }
            }
            current_depth_symbols = current_paths
                .iter()
                .filter_map(|p| self.path_by_symbols.get(p))
                .flatten()
                .cloned()
                .collect::<Vec<_>>();
            current_depth += 1;
        }

        paths
            .iter()
            .unique()
            .filter_map(|p| self.path_by_symbols.get(p))
            .flatten()
            .cloned()
            .filter(|s| {
                let symbol_type = s.borrow().symbol_type();
                symbol_type == SymbolType::StructDeclaration
                    || symbol_type == SymbolType::TypeAlias
                    || symbol_type == SymbolType::FunctionDeclaration
            })
            .collect::<Vec<_>>()
    }

    pub fn file_markup(
        &self,
        doc: &Document,
    ) -> Result<FileASTMarkup, String> {
        assert!(doc.text.is_some());
        let symbols = match self.path_by_symbols.get(&doc.path) {
            Some(x) => x.iter().map(|s| s.borrow().symbol_info_struct()).collect::<Vec<_>>(),
            None => {
                info!("no symbols in index for {:?}, assuming it's a new file of some sort and parsing it", doc.path);
                let mut parser = match get_ast_parser_by_filename(&doc.path) {
                    Ok(parser) => parser,
                    Err(e) => {
                        return Err(format!("no symbols in index for {:?}, and cannot find a parser this kind of file: {}", doc.path, e.message));
                    }
                };
                let t0 = std::time::Instant::now();
                let symbols = parser.parse(doc.text.as_ref().unwrap().to_string().as_str(), &doc.path);
                info!("/parse {}ms", t0.elapsed().as_millis());
                symbols.iter().map(|s| s.read().symbol_info_struct()).collect::<Vec<_>>()
            }
        };
        crate::ast::ast_file_markup::lowlevel_file_markup(doc, &symbols)
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
                        let s_ref = s.borrow();
                        match request_symbol_type {
                            RequestSymbolType::Declaration => s_ref.is_declaration(),
                            RequestSymbolType::Usage => !s_ref.is_declaration(),
                            RequestSymbolType::All => true,
                        }
                    })
                    .map(|s| s.borrow().symbol_info_struct())
                    .collect()
            })
            .unwrap_or_default();
        Ok(symbols)
    }

    pub fn get_symbols_names(
        &self,
        request_symbol_type: RequestSymbolType,
    ) -> Vec<String> {
        match request_symbol_type {
            RequestSymbolType::Declaration => {
                self.declaration_symbols_by_name
                    .iter()
                    .map(|(name, _)| name.to_string())
                    .collect()
            }
            RequestSymbolType::Usage => {
                self.usage_symbols_by_name
                    .iter()
                    .map(|(name, _)| name.to_string())
                    .collect()
            }
            RequestSymbolType::All => {
                self.declaration_symbols_by_name
                    .iter()
                    .chain(self.usage_symbols_by_name.iter())
                    .map(|(name, _)| name.to_string())
                    .collect()
            }
        }
    }

    pub fn get_symbols_paths(
        &self,
        request_symbol_type: RequestSymbolType,
    ) -> Vec<String> {
        match request_symbol_type {
            RequestSymbolType::Declaration => {
                self.declaration_symbols_by_fullpath
                    .iter()
                    .map(|(name, _)| name.to_string())
                    .collect()
            }
            RequestSymbolType::Usage => {
                self.usage_symbols_by_fullpath
                    .iter()
                    .map(|(name, _)| name.to_string())
                    .collect()
            }
            RequestSymbolType::All => {
                self.declaration_symbols_by_fullpath
                    .iter()
                    .chain(self.usage_symbols_by_fullpath.iter())
                    .map(|(name, _)| name.to_string())
                    .collect()
            }
        }
    }

    pub(crate) fn symbols_by_guid(&self) -> &HashMap<Uuid, AstSymbolInstanceRc> {
        &self.symbols_by_guid
    }

    pub(crate) fn needs_update(&self) -> bool {
        self.has_changes || self.symbols_by_guid.is_empty()
    }

    pub(crate) fn set_updated(&mut self) {
        self.has_changes = false;
    }

    pub(crate) fn reindex(&mut self) {
        let mut symbols = self
            .symbols_by_guid()
            .values()
            .cloned()
            .collect::<Vec<_>>();
        info!("Resolving declaration symbols");
        let t0 = std::time::Instant::now();
        let stats = self.resolve_declaration_symbols(&mut symbols);
        info!(
            "Resolving declaration symbols finished, took {:.3}s, {} found, {} not found",
            t0.elapsed().as_secs_f64(),
            stats.found,
            stats.non_found
        );
        if self.shutdown_flag.load(Ordering::SeqCst) {
            info!("Aborting ast indexing, shutdown signal received");
            return;
        }

        info!("Resolving import symbols");
        let t0 = std::time::Instant::now();
        let (stats, import_components_succ_solution_index) = self.resolve_imports(
            &mut symbols, &self.import_components_succ_solution_index,
        );
        if self.shutdown_flag.load(Ordering::SeqCst) {
            info!("Aborting ast indexing, shutdown signal received");
            return;
        }
        self.import_components_succ_solution_index.extend(import_components_succ_solution_index);
        info!(
            "Resolving import symbols finished, took {:.3}s, {} found, {} not found",
            t0.elapsed().as_secs_f64(),
            stats.found,
            stats.non_found
        );

        info!("Linking usage and declaration symbols");
        let t1 = std::time::Instant::now();
        let stats = self.merge_usages_to_declarations(&mut symbols);
        if self.shutdown_flag.load(Ordering::SeqCst) {
            info!("Aborting ast indexing, shutdown signal received");
            return;
        }
        info!(
            "Linking usage and declaration symbols finished, took {:.3}s, {} found, {} not found",
            t1.elapsed().as_secs_f64(),
            stats.found,
            stats.non_found
        );

        info!("Creating extra ast indexes");
        let t2 = std::time::Instant::now();
        self.create_extra_indexes(&symbols);
        self.set_updated();
        info!("Creating extra ast indexes finished, took {:.3}s", t2.elapsed().as_secs_f64());
        let _ = write!(std::io::stderr(), "AST COMPLETE\n");
        info!("AST COMPLETE");  // you can see stderr "AST COMPLETE" sometimes faster vs logs
    }

    pub(crate) fn total_files(&self) -> usize {
        self.path_by_symbols.len()
    }

    pub(crate) fn total_symbols(&self) -> usize {
        self.symbols_by_guid.len()
    }

    pub(crate) fn is_overflowed(&self) -> bool {
        self.path_by_symbols.len() >= self.ast_index_max_files
    }

    fn resolve_declaration_symbols(&self, symbols: &mut Vec<AstSymbolInstanceRc>) -> IndexingStats {
        let mut stats = IndexingStats { found: 0, non_found: 0 };
        for symbol in symbols.iter_mut() {
            if self.shutdown_flag.load(Ordering::SeqCst) {
                return stats;
            }
            let (type_names, symb_type, symb_path) = {
                let s_ref = symbol.borrow();
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
                if t.is_pod || t.name.is_none() {
                    stats.non_found += 1;
                    new_guids.push(t.guid.clone());
                    continue;
                }

                if let Some(guid) = t.guid {
                    if self.symbols_by_guid.contains_key(&guid) {
                        new_guids.push(t.guid.clone());
                        continue;
                    }
                }

                let name = t.name.clone().expect("filter has invalid condition");
                let maybe_guid = match self.declaration_symbols_by_name.get(&name) {
                    Some(symbols) => {
                        symbols
                            .iter()
                            .filter(|s| s.borrow().is_type())
                            .min_by(|a, b| {
                                let path_a = a.borrow().file_path().clone();
                                let path_b = b.borrow().file_path().clone();
                                FilePathIterator::compare_paths(&symb_path, &path_a, &path_b)
                            })
                            .map(|s| s.borrow().guid().clone())
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
            {
                let mut symbol_ref = symbol.borrow_mut();
                symbol_ref.set_guids_to_types(&new_guids);
                symbol_ref.temporary_types_cleanup();
            }
        }
        stats
    }

    fn merge_usages_to_declarations(&self, symbols: &mut Vec<AstSymbolInstanceRc>) -> IndexingStats {
        fn get_caller_depth(
            symbol: &AstSymbolInstanceRc,
            guid_by_symbols: &HashMap<Uuid, AstSymbolInstanceRc>,
        ) -> Option<usize> {
            match symbol.borrow().get_caller_guid() {
                Some(_) => {}
                None => {
                    return None;
                }
            };

            let mut current_symbol = symbol.clone();
            let mut current_depth = 0;
            loop {
                let caller_guid = match current_symbol.borrow().get_caller_guid().clone() {
                    Some(g) => g,
                    None => {
                        return Some(current_depth);
                    }
                };
                match guid_by_symbols.get(&caller_guid) {
                    Some(s) => {
                        current_symbol = s.clone();
                        current_depth += 1;
                    }
                    None => {
                        return Some(current_depth);
                    }
                }
            }
        }
        for s in symbols.iter_mut() {
            let caller_depth = get_caller_depth(s, &self.symbols_by_guid);
            s.borrow_mut().set_caller_depth(caller_depth);
        }

        let mut stats = IndexingStats { found: 0, non_found: 0 };
        let search_by_name_extra_index: HashMap<(String, Uuid, String), AstSymbolInstanceRc> = symbols
            .iter()
            .map(|x| {
                let x_ref = x.borrow();
                ((x_ref.name().to_string(),
                  x_ref.parent_guid().clone().unwrap_or_default(),
                  x_ref.file_path().to_str().unwrap_or_default().to_string()),
                 x.clone())
            })
            .collect();
        let search_by_caller_extra_index: HashMap<(String, Uuid, SymbolType), Uuid> = symbols
            .iter()
            .map(|x| {
                let x_ref = x.borrow();
                ((x_ref.name().to_string(),
                  x_ref.parent_guid().clone().unwrap_or_default(),
                  x_ref.symbol_type().clone()),
                 x_ref.guid().clone())
            })
            .collect();
        let mut depth: usize = 0; // depth means "a.b.c" it's 2 for c
        loop {
            let mut symbols_to_process = symbols
                .iter()
                .filter(|symbol| {
                    let s_ref = symbol.borrow();
                    let has_no_valid_linked_decl = if let Some(guid) = s_ref.get_linked_decl_guid() {
                        !self.symbols_by_guid.contains_key(guid)
                    } else {
                        true
                    };
                    let valid_depth = match symbol.borrow().get_caller_depth() {
                        Some(d) => *d == depth,
                        None => false
                    };
                    has_no_valid_linked_decl && valid_depth && (s_ref.symbol_type() == SymbolType::FunctionCall
                        || s_ref.symbol_type() == SymbolType::VariableUsage)
                })
                .collect::<Vec<_>>();

            if symbols_to_process.is_empty() {
                break;
            }

            let mut symbols_cache: HashMap<(Uuid, String), Option<Uuid>> = HashMap::new();
            for usage_symbol in symbols_to_process.iter_mut() {
                if self.shutdown_flag.load(Ordering::SeqCst) {
                    return stats;
                }

                let (name, parent_guid, caller_guid) = {
                    let s_ref = usage_symbol.borrow();
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
                                &search_by_caller_extra_index,
                            ) {
                                Some(decl_guid) => { Some(decl_guid) }
                                None => find_decl_by_name(
                                    *usage_symbol,
                                    &self.path_by_symbols,
                                    &self.symbols_by_guid,
                                    &search_by_name_extra_index,
                                    1,
                                )
                            }
                        }
                        None => find_decl_by_name(
                            *usage_symbol,
                            &self.path_by_symbols,
                            &self.symbols_by_guid,
                            &search_by_name_extra_index,
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
                            usage_symbol.borrow_mut().set_linked_decl_guid(Some(guid))
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

    fn resolve_imports(
        &self,
        symbols: &mut Vec<AstSymbolInstanceRc>,
        import_components_succ_solution_index: &HashMap<String, ImportDeclaration>,
    ) -> (IndexingStats, HashMap<String, ImportDeclaration>) {
        fn random_keys<K, V>(map: &HashMap<K, V>, n: usize) -> Vec<K>
            where
                K: Clone + Hash + Eq,
        {
            let mut rng = rand::thread_rng();
            let mut result = Vec::with_capacity(n);
            let mut keys_iter = map.keys();

            for (i, key) in keys_iter.by_ref().enumerate() {
                if i < n {
                    result.push(key.clone());
                } else {
                    let j = rng.gen_range(0..=i);
                    if j < n {
                        result[j] = key.clone();
                    }
                }
            }

            result
        }

        let mut stats = IndexingStats { found: 0, non_found: 0 };
        let paths_str = self.path_by_symbols
            .keys()
            .filter_map(|path| path.to_str())
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let paths = random_keys(&self.path_by_symbols, 100);
        let prefixes = top_n_prefixes(&paths, 3);
        let min_prefix = prefixes.iter().next().map(|x| x.0.clone());
        let mut notfound_import_components_index: HashMap<String, Vec<AstSymbolInstanceRc>> = HashMap::new();
        let mut import_components_succ_solution_index_local: HashMap<String, ImportDeclaration> = HashMap::new();
        for symbol in symbols
            .iter_mut()
            .filter(|s| s.borrow().as_any().downcast_ref::<ImportDeclaration>().is_some()) {
            if self.shutdown_flag.load(Ordering::SeqCst) {
                return (stats, import_components_succ_solution_index_local);
            }

            let (import_type, file_path, path_components, language) = {
                let s_ref = symbol.borrow();
                let import_decl = s_ref.as_any().downcast_ref::<ImportDeclaration>().expect("wrong type");
                (import_decl.import_type.clone(), import_decl.file_path().clone(), import_decl.path_components.clone(), import_decl.language().clone())
            };
            if import_type == ImportType::System || import_type == ImportType::Library {
                continue;
            }
            let import_components_path = path_components.iter().join("/");
            if import_components_succ_solution_index.contains_key(&import_components_path) {
                let prepared_imp = import_components_succ_solution_index.get(&import_components_path).expect("assert");
                let mut symbol_ref = symbol.borrow_mut();
                let imp_decl = symbol_ref.as_any_mut().downcast_mut::<ImportDeclaration>().expect("wrong type");
                imp_decl.filepath_ref = prepared_imp.filepath_ref.clone();
                imp_decl.ast_fields.name = prepared_imp.ast_fields.name.clone();
                continue;
            }
            if import_components_succ_solution_index_local.contains_key(&import_components_path) {
                let prepared_imp = import_components_succ_solution_index_local.get(&import_components_path).expect("assert");
                let mut symbol_ref = symbol.borrow_mut();
                let imp_decl = symbol_ref.as_any_mut().downcast_mut::<ImportDeclaration>().expect("wrong type");
                imp_decl.filepath_ref = prepared_imp.filepath_ref.clone();
                imp_decl.ast_fields.name = prepared_imp.ast_fields.name.clone();
                continue;
            }
            let search_items = possible_filepath_candidates(
                &prefixes,
                &min_prefix,
                &file_path,
                &path_components,
                &language,
            );
            match try_find_file_path(
                &search_items,
                &self.path_by_symbols,
                &paths_str,
            ) {
                Some(search_res) => {
                    stats.found += 1;
                    let mut symbol_ref = symbol.borrow_mut();
                    let import_decl = symbol_ref.as_any_mut().downcast_mut::<ImportDeclaration>().expect("wrong type");
                    import_decl.filepath_ref = Some(search_res.path.clone());
                    import_decl.ast_fields.name = search_res.name.clone().unwrap_or_default();
                    import_components_succ_solution_index_local.insert(import_components_path, import_decl.clone());
                }
                None => {
                    notfound_import_components_index.entry(import_components_path).or_default().push(symbol.clone());
                    stats.non_found += 1;
                }
            };
        }
        for (import_components_path, symbols) in notfound_import_components_index.iter_mut() {
            if let Some(succ_import_decl) = import_components_succ_solution_index_local.get(import_components_path) {
                for symbol in symbols.iter_mut() {
                    let mut symbol_ref = symbol.borrow_mut();
                    let symbol_decl = symbol_ref.as_any_mut().downcast_mut::<ImportDeclaration>().expect("wrong type");
                    symbol_decl.filepath_ref = succ_import_decl.filepath_ref.clone();
                    symbol_decl.ast_fields.name = succ_import_decl.ast_fields.name.clone();
                    stats.found += 1;
                    stats.non_found -= 1;
                }
            }
        }
        (stats, import_components_succ_solution_index_local)
    }

    fn create_extra_indexes(&mut self, symbols: &Vec<AstSymbolInstanceRc>) {
        for symbol in symbols
            .iter()
            .filter(|s| !s.borrow().is_type())
            .cloned() {
            if self.shutdown_flag.load(Ordering::SeqCst) {
                return;
            }

            let guid = symbol.borrow().guid().clone();
            if self.type_guid_to_dependent_guids.contains_key(&guid) {
                self.type_guid_to_dependent_guids.remove(&guid);
            }
            if self.declaration_guid_to_usage_names.contains_key(&guid) {
                self.declaration_guid_to_usage_names.remove(&guid);
            }
        }

        for symbol in symbols
            .iter()
            .cloned() {
            if self.shutdown_flag.load(Ordering::SeqCst) {
                return;
            }
            let full_path = self.get_symbol_full_path(&symbol);
            if !full_path.is_empty() {
                if symbol.borrow().is_declaration() {
                    self.declaration_symbols_by_fullpath.entry(full_path)
                        .or_insert_with(Vec::new)
                        .push(symbol.clone());
                } else {
                    self.usage_symbols_by_fullpath.entry(full_path)
                        .or_insert_with(Vec::new)
                        .push(symbol.clone());
                }
            }
            if symbol.borrow().is_type() {
                continue
            }

            let (name, s_guid, mut types, is_declaration, symbol_type, parent_guid) = {
                let s_ref = symbol.borrow();
                (s_ref.name().to_string(), s_ref.guid().clone(), s_ref.types(), s_ref.is_declaration(),
                 s_ref.symbol_type(), s_ref.parent_guid().clone())
            };
            types = if is_declaration {
                types
            } else {
                symbol.borrow()
                    .get_linked_decl_guid()
                    .clone()
                    .map(|guid| self.symbols_by_guid.get(&guid))
                    .flatten()
                    .map(|s| s.borrow().types())
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

    fn parse_single_file(
        &self,
        doc: &Document,
        code: &str,
    ) -> Vec<AstSymbolInstanceRc> {
        // This function runs to find symbols near cursor
        let mut doc = doc.clone();
        doc.update_text(&code.to_string());
        let mut symbols = AstIndex::parse(&doc)
            .unwrap_or_default()
            .iter()
            .map(|sym| {
                let mut write_lock = sym.write();
                Rc::new(RefCell::new(std::mem::replace(&mut *write_lock, Box::new(ImportDeclaration::default()))))
            }).collect::<Vec<_>>();
        self.resolve_declaration_symbols(&mut symbols);
        _ = self.resolve_imports(&mut symbols, &self.import_components_succ_solution_index);
        self.merge_usages_to_declarations(&mut symbols);
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
