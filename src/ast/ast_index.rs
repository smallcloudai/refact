use std::collections::HashMap;
use std::path::PathBuf;

use fst::{Set, set, Streamer};
use fst::automaton::Subsequence;
use log::info;
use sorted_vec::SortedVec;
use strsim::jaro_winkler;
use tokio::fs::read_to_string;

use crate::ast::structs::SymbolsSearchResultStruct;
use crate::ast::treesitter::parsers::{get_parser_by_filename, LanguageParser};
use crate::ast::treesitter::structs::SymbolDeclarationStruct;

pub struct AstIndex {
    nodes: HashMap<String, SymbolDeclarationStruct>,
    nodes_indexes: HashMap<PathBuf, Set<Vec<u8>>>,
}


fn make_a_query(
    nodes_indexes: &HashMap<PathBuf, Set<Vec<u8>>>,
    query_str: &str,
) -> Vec<String> {
    let matcher = Subsequence::new(query_str);
    let mut stream_builder = set::OpBuilder::new();
    for (_, set) in nodes_indexes {
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
            nodes: HashMap::new(),
            nodes_indexes: HashMap::new(),
        }
    }

    pub async fn add_or_update(&mut self, file_path: &PathBuf) -> Result<(), String> {
        let mut parser = match get_parser_by_filename(file_path) {
            Ok(parser) => parser,
            Err(err) => {
                return Err(err.message);
            }
        };
        let text = match read_to_string(file_path).await {
            Ok(s) => s,
            Err(e) => return Err(e.to_string())
        };
        let declarations = match parser.parse_declarations(text.as_str(), file_path) {
            Ok(declarations) => declarations,
            Err(e) => {
                return Err(format!("Error parsing {}: {}", file_path.display(), e));
            }
        };
        match self.remove(file_path).await {
            Ok(()) => (),
            Err(e) => return Err(format!("Error removing {}: {}", file_path.display(), e)),
        }

        let mut meta_names: SortedVec<String> = SortedVec::new();
        for (meta_path, declaration) in declarations.iter() {
            self.nodes.insert(meta_path.clone(), declaration.clone());
            meta_names.push(meta_path.clone());
        }
        let meta_names_set = match Set::from_iter(meta_names.iter()) {
            Ok(set) => set,
            Err(e) => return Err(format!("Error creating set: {}", e)),
        };
        self.nodes_indexes.insert(file_path.clone(), meta_names_set);
        Ok(())
    }

    pub async fn remove(&mut self, filename: &PathBuf) -> Result<(), String> {
        if let Some(meta_names) = self.nodes_indexes.remove(filename) {
            while let Some(name_vec) = meta_names.stream().next() {
                let name = match String::from_utf8(name_vec.to_vec()) {
                    Ok(name) => name,
                    Err(_) => {
                        continue;
                    }
                };
                self.nodes.remove(&name);
            }
        }
        Ok(())
    }

    pub async fn search(
        &self,
        query: &str,
        top_n: usize,
        exception_filename: Option<PathBuf>,
    ) -> Result<Vec<SymbolsSearchResultStruct>, String> {
        let query_str = query.to_string();
        let found_keys = make_a_query(&self.nodes_indexes, query_str.as_str());

        let exception_filename = exception_filename.unwrap_or_default();
        let filtered_found_keys = found_keys
            .iter()
            .filter_map(|k| self.nodes.get(k))
            .filter(|k| k.definition_info.path != exception_filename && !k.meta_path.is_empty())
            .collect::<Vec<_>>();

        let mut filtered_search_results: Vec<(SymbolDeclarationStruct, f32)> = filtered_found_keys
            .into_iter()
            .map(|key| (key.clone(), jaro_winkler(query, key.meta_path.as_str()) as f32))
            .collect();
        filtered_search_results.sort_by(|(key_1, dist_1), (key_2, dist_2)|
            dist_1.partial_cmp(dist_2).unwrap_or(std::cmp::Ordering::Equal)
        );

        let mut search_results: Vec<SymbolsSearchResultStruct> = vec![];
        for (key, dist) in filtered_search_results.into_iter().take(top_n) {
            let content = match key.get_content().await {
                Ok(content) => content,
                Err(err) => {
                    info!("Error getting content: {}", err);
                    continue;
                }
            };
            search_results.push(SymbolsSearchResultStruct {
                symbol_declaration: key.clone(),
                content: content,
                dist_to_query: dist,
            });
        }
        Ok(search_results)
    }
}
