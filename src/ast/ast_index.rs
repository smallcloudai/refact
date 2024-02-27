use std::cmp::max;
use std::collections::HashMap;
use std::path::PathBuf;

use fst::{Set, set, Streamer};
use rayon::current_num_threads;
use rayon::prelude::*;
use sorted_vec::SortedVec;
use strsim::jaro_winkler;
use tracing::{debug, info};
use tree_sitter::Range;

use crate::ast::fst_extra_automation::Substring;
use crate::ast::structs::SymbolsSearchResultStruct;
use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::get_parser_by_filename;
use crate::ast::treesitter::structs::{SymbolDeclarationStruct, UsageSymbolInfo};
use crate::files_in_workspace::DocumentInfo;

#[derive(Debug)]
pub struct AstIndex {
    declarations: HashMap<String, SymbolDeclarationStruct>,
    declarations_search_index: HashMap<PathBuf, Set<Vec<u8>>>,
    usages: HashMap<String, Vec<Box<dyn UsageSymbolInfo>>>,
    usages_search_index: HashMap<PathBuf, Set<Vec<u8>>>,
}


fn make_a_query(
    nodes_indexes: &HashMap<PathBuf, Set<Vec<u8>>>,
    query_str: &str,
    exception_doc: Option<DocumentInfo>,
) -> Vec<String> {
    let matcher = Substring::new(query_str, true);
    let mut stream_builder = set::OpBuilder::new();

    for (doc, set) in nodes_indexes {
        if let Some(ref exception) = exception_doc {
            if *doc == exception.get_path() {
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
            declarations: HashMap::new(),
            declarations_search_index: HashMap::new(),
            usages: HashMap::new(),
            usages_search_index: HashMap::new(),
        }
    }

    pub fn get_declarations_and_usages(doc: &DocumentInfo)
                                       -> Result<(HashMap<String, SymbolDeclarationStruct>, Vec<Box<dyn UsageSymbolInfo>>), String> {
        let path = doc.get_path();
        let mut parser = match get_parser_by_filename(&doc.get_path()) {
            Ok(parser) => parser,
            Err(err) => {
                return Err(err.message);
            }
        };
        let text = match doc.read_file_blocked() {
            Ok(s) => s,
            Err(e) => return Err(e.to_string())
        };

        // Parse the text and get the declarations and usages
        let t_declarations = std::time::Instant::now();
        let declarations = match parser.parse_declarations(text.as_str(), &path) {
            Ok(declarations) => declarations,
            Err(e) => {
                return Err(format!("Error parsing {}: {}", path.display(), e));
            }
        };
        let t_declarations_elapsed = t_declarations.elapsed();

        let t_usages = std::time::Instant::now();
        let mut usages = match parser.parse_usages(text.as_str(), false) {
            Ok(usages) => usages,
            Err(e) => {
                return Err(format!("Error parsing {}: {}", path.display(), e));
            }
        };
        link_declarations_to_usages(&declarations, &mut usages);
        let t_usages_elapsed = t_usages.elapsed();
        info!(
            "parsed {},  {} definitions, {} usages, \
            took {:.3}s to parse decls, took {:.3}s to parse refs",
            crate::nicer_logs::last_n_chars(&path.display().to_string(), 30),
            declarations.len(), usages.len(),
            t_declarations_elapsed.as_secs_f32(), t_usages_elapsed.as_secs_f32()
        );
        Ok((declarations, usages))
    }

    pub fn add_or_update_declarations_and_usages(&mut self, doc: &DocumentInfo,
                                                 declarations: HashMap<String, SymbolDeclarationStruct>,
                                                 usages: Vec<Box<dyn UsageSymbolInfo>>) -> Result<(), String> {
        let path = doc.get_path();
        // Remove old data from all search indexes
        match self.remove(&doc) {
            Ok(()) => (),
            Err(e) => return Err(format!("Error removing {}: {}", path.display(), e)),
        }

        // Insert new data to the declarations search index
        let mut meta_names: SortedVec<String> = SortedVec::new();
        for (meta_path, declaration) in declarations.iter() {
            self.declarations.insert(meta_path.clone(), declaration.clone());
            meta_names.push(meta_path.clone());
        }
        let meta_names_set = match Set::from_iter(meta_names.iter()) {
            Ok(set) => set,
            Err(e) => return Err(format!("Error creating set: {}", e)),
        };
        self.declarations_search_index.insert(path.clone(), meta_names_set);

        // Insert new data to the usages search index
        let mut usages_meta_names: SortedVec<String> = SortedVec::new();
        for usage in usages {
            usages_meta_names.push(usage.meta_path());
            self.usages.entry(usage.meta_path()).or_default().push(usage);
        }
        let meta_names_set = match Set::from_iter(usages_meta_names.iter()) {
            Ok(set) => set,
            Err(e) => return Err(format!("Error creating set: {}", e)),
        };
        self.usages_search_index.insert(path.clone(), meta_names_set);
        Ok(())
    }

    pub fn add_or_update(&mut self, doc: &DocumentInfo) -> Result<(), String> {
        let (declarations, usages) = AstIndex::get_declarations_and_usages(doc)?;
        self.add_or_update_declarations_and_usages(doc, declarations, usages)
    }

    pub fn remove(&mut self, doc: &DocumentInfo) -> Result<(), String> {
        let path = doc.get_path();
        if let Some(meta_names) = self.declarations_search_index.remove(&path) {
            let mut stream = meta_names.stream();
            while let Some(name_vec) = stream.next() {
                let name = match String::from_utf8(name_vec.to_vec()) {
                    Ok(name) => name,
                    Err(_) => {
                        continue;
                    }
                };
                self.declarations.remove(&name);
            }
        }
        if let Some(meta_names) = self.usages_search_index.remove(&path) {
            let mut stream = meta_names.stream();
            while let Some(name_vec) = stream.next() {
                let name = match String::from_utf8(name_vec.to_vec()) {
                    Ok(name) => name,
                    Err(_) => {
                        continue;
                    }
                };
                self.usages.remove(&name);
            }
        }
        Ok(())
    }

    pub async fn clear_index(&mut self) {
        self.declarations.clear();
        self.declarations_search_index.clear();
        self.usages.clear();
        self.usages_search_index.clear();
    }

    pub fn search_declarations(
        &self,
        query: &str,
        top_n: usize,
        exception_doc: Option<DocumentInfo>,
        language: Option<LanguageId>
    ) -> Result<Vec<SymbolsSearchResultStruct>, String> {
        let query_str = query.to_string();
        let found_keys = make_a_query(
            &self.declarations_search_index, query_str.as_str(),
            exception_doc,
        );

        let mut filtered_search_results = found_keys
            .par_chunks(current_num_threads())
            .map(|keys| {
                keys.iter().filter_map(|k| {
                    if let Some(decl) = self.declarations.get(k) {
                        if decl.meta_path.is_empty() { return None; }
                        if !language.map(|x| decl.language == x).unwrap_or(true) { return None; }

                        return Some((decl.clone(),
                                     (jaro_winkler(query, decl.meta_path.as_str()) as f32).max(f32::MIN_POSITIVE) *
                                         (jaro_winkler(query, decl.name.as_str()) as f32).max(f32::MIN_POSITIVE)));
                    } else {
                        None
                    }
                }).collect::<Vec<_>>()
            }).flatten().collect::<Vec<_>>();
        filtered_search_results.sort_by(|(_, dist_1), (_, dist_2)|
            dist_1.partial_cmp(dist_2).unwrap_or(std::cmp::Ordering::Equal)
        );

        let mut search_results: Vec<SymbolsSearchResultStruct> = vec![];
        for (key, dist) in filtered_search_results
            .into_iter()
            .rev()
            .take(top_n) {
            let content = match key.get_content_blocked() {
                Ok(content) => content,
                Err(err) => {
                    info!("Error opening the file {:?}: {}", key.definition_info.path, err);
                    continue;
                }
            };
            search_results.push(SymbolsSearchResultStruct {
                symbol_declaration: key.clone(),
                content: content,
                sim_to_query: dist,
            });
        }
        Ok(search_results)
    }

    pub async fn search_usages(
        &self,
        query: &str,
        top_n: usize,
        exception_doc: Option<DocumentInfo>,
        language: Option<LanguageId>
    ) -> Result<Vec<SymbolsSearchResultStruct>, String> {
        let query_str = query.to_string();
        let found_keys = make_a_query(&self.usages_search_index, query_str.as_str(), exception_doc);

        let filtered_found_keys = found_keys
            .par_iter()
            .filter_map(|k| self.usages.get(k))
            .flatten()
            .filter(|k|
                !k.meta_path().is_empty() && k.get_declaration_meta_path().is_some()
            )
            .collect::<Vec<_>>();

        let mut filtered_search_results: Vec<(SymbolDeclarationStruct, f32)> = filtered_found_keys
            .par_chunks(current_num_threads())
            .map(|keys| {
                keys.iter().filter_map(|&key| {
                    if let Some(decl) = self.declarations.get(&key.get_declaration_meta_path().unwrap_or_default()) {
                        let dist = jaro_winkler(query, &key.meta_path()) as f32;
                        if language.map(|x| decl.language == x).unwrap_or(true) {
                            return Some((decl.clone(), dist));
                        }
                    }
                    None
                }).collect::<Vec<_>>()
            })
            .flatten().collect();
        filtered_search_results.sort_by(|(_, dist_1), (_, dist_2)|
            dist_1.partial_cmp(dist_2).unwrap_or(std::cmp::Ordering::Equal)
        );

        let mut search_results: Vec<SymbolsSearchResultStruct> = vec![];
        for (key, dist) in filtered_search_results
            .into_iter()
            .rev()
            .take(top_n) {
            let content = match key.get_content().await {
                Ok(content) => content,
                Err(err) => {
                    info!("Error opening the file {:?}: {}", key.definition_info.path, err);
                    continue;
                }
            };
            search_results.push(SymbolsSearchResultStruct {
                symbol_declaration: key.clone(),
                content: content,
                sim_to_query: dist,
            });
        }
        Ok(search_results)
    }

    pub fn get_symbols_by_file_path(&self, doc: &DocumentInfo) -> Result<Vec<SymbolDeclarationStruct>, String> {
        let path = doc.get_path();
        let mut result: Vec<SymbolDeclarationStruct> = vec![];
        if let Some(meta_names) = self.declarations_search_index.get(&path) {
            let mut stream = meta_names.stream();
            while let Some(name_vec) = stream.next() {
                let name = match String::from_utf8(name_vec.to_vec()) {
                    Ok(name) => name,
                    Err(_) => {
                        continue;
                    }
                };
                match self.declarations.get(&name) {
                    None => {
                        continue;
                    }
                    Some(s) => result.push(s.clone())
                }
            }
            return Ok(result);
        }
        return Err(format!("File {} is not found in the AST index", path.display()));
    }

    pub fn get_indexed_symbol_paths(&self) -> Vec<String> {
        self.declarations.iter().map(|(path, _)| path.clone()).collect()
    }

    pub fn get_indexed_references(&self) -> Vec<String> {
        self.usages.iter().map(|(path, _)| path.clone()).collect()
    }

    pub fn get_indexed_file_paths(&self) -> Vec<PathBuf> {
        self.usages_search_index.iter().map(|(path, _)| path.clone()).collect()
    }
}

fn link_declarations_to_usages(
    declarations: &HashMap<String, SymbolDeclarationStruct>,
    usages: &mut Vec<Box<dyn UsageSymbolInfo>>,
) {
    fn within_range(
        decl_range: &Range,
        usage_range: &Range,
    ) -> bool {
        decl_range.start_point.row <= usage_range.start_point.row && decl_range.end_point.row >= usage_range.end_point.row
    }

    for usage in usages.iter_mut() {
        let mut closest_declaration: Option<String> = None;
        let mut closest_declaration_rows_count: Option<usize> = None;
        let range = usage.get_range();
        for (meta_path, declaration) in declarations.iter() {
            if within_range(&declaration.definition_info.range, &range) {
                let distance = max(
                    declaration.definition_info.range.end_point.row - declaration.definition_info.range.start_point.row,
                    0,
                );
                if closest_declaration.is_none() || closest_declaration_rows_count.unwrap_or(distance + 1) < distance {
                    closest_declaration = Some(meta_path.clone());
                    closest_declaration_rows_count = Some(distance);
                }
            }
        }
        match closest_declaration {
            Some(closest_declaration) => {
                usage.set_definition_meta_path(closest_declaration);
            }
            None => {
                debug!("usage {:?} not found in the AST", usage.meta_path());
            }
        }
    }
}

