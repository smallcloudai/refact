use std::collections::HashMap;
use std::path::PathBuf;

use fst::{Set, set, Streamer};
use fst::automaton::Levenshtein;
use futures::stream::{self, StreamExt};
use itertools::Itertools;
use sorted_vec::SortedVec;
use strsim::normalized_levenshtein;
use tokio::fs::read_to_string;

use crate::ast::structs::SymbolsSearchResultStruct;
use crate::ast::treesitter::parsers::{get_parser_by_filename, LanguageParser};
use crate::ast::treesitter::structs::SymbolDeclarationStruct;

pub struct AstIndex {
    nodes: HashMap<String, SymbolDeclarationStruct>,
    nodes_indexes: HashMap<PathBuf, Set<Vec<u8>>>,
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
        let nodes = match parser.parse_declarations(text.as_str(), file_path) {
            Ok(nodes) => nodes,
            Err(e) => {
                return Err(format!("Error parsing {}: {}", file_path.display(), e));
            }
        };
        match self.remove(file_path).await {
            Ok(()) => (),
            Err(e) => return Err(format!("Error removing {}: {}", file_path.display(), e)),
        }

        let mut meta_names: SortedVec<String> = SortedVec::new();
        for (meta_path, declaration) in nodes.iter() {
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
                        continue
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
        let lev = match Levenshtein::new(query, 3) {
            Ok(lev) => lev,
            Err(e) => return Err(format!("Error creating Levenshtein: {}", e)),
        };
        let mut stream_builder = set::OpBuilder::new();
        for (_, set) in &self.nodes_indexes {
            stream_builder = stream_builder.add(set.search(&lev));
        }

        let mut stream = stream_builder.union();
        let mut found_keys = vec![];
        while let Some(key) = stream.next() {
            match String::from_utf8(key.to_vec()) {
                Ok(key) => found_keys.push(key),
                Err(_) => {}
            }
        }
        let filtered_found_keys = found_keys
            .iter()
            .unique()
            .filter_map(|k| self.nodes.get(k))
            .filter(|k| k.definition_info.path == exception_filename.clone().unwrap_or(PathBuf::default()))
            .collect::<Vec<_>>();

        let futures = filtered_found_keys.clone()
            .into_iter()
            .map(|k| async {
                let content = k.get_content().await.unwrap(); // TODO fix this
                Ok(SymbolsSearchResultStruct {
                    symbol_path: k.meta_path.clone(),
                    content,
                    lev_dist_to_query: normalized_levenshtein(query, k.meta_path.as_str()) as f32,
                })
            });

        let mut symbols: Vec<_> = stream::iter(futures)
            .buffer_unordered(filtered_found_keys.len())
            .filter_map(|res: Result<SymbolsSearchResultStruct, String>| async { res.ok() })
            .collect()
            .await;

        symbols.sort_by(|a, b| a.lev_dist_to_query.partial_cmp(&b.lev_dist_to_query).unwrap_or(std::cmp::Ordering::Equal));
        Ok(symbols.into_iter().take(top_n).collect())

    }
}
