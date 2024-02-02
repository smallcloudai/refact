use std::collections::HashMap;
use std::path::PathBuf;
use crate::ast::structs::SymbolDeclarationStruct;
use fst::{IntoStreamer, Streamer, Set};
use fst::automaton::Levenshtein;
use sorted_vec::SortedVec;
use crate::ast::treesitter::parsers::{get_parser_by_filename, LanguageParser, ParserError};


pub struct AstIndex {
    nodes: HashMap<String, SymbolDeclarationStruct>,
    nodes_indexes: HashMap<PathBuf, Set<Vec<u8>>>,
}

impl AstIndex {
    pub fn init() -> AstIndex {
        AstIndex {
            nodes: HashMap::new(),
            nodes_indexes: HashMap::new()
        }
    }

    pub async fn add_or_update(&mut self, filename: &PathBuf) -> Result<(), String> {
        let mut parser = match get_parser_by_filename(filename) {
            Ok(parser) => parser,
            Err(err) => {
                return Err(err.message)
            }
        };
        let nodes = match parser.parse_declarations(filename).await {
            Ok(nodes) => nodes,
            Err(e) => {
                return Err(format!("Error parsing {}: {}", filename.display(), e));
            }
        };
        match self.remove(filename).await {
            Ok(()) => (),
            Err(e) => return Err(format!("Error removing {}: {}", filename.display(), e)),
        }

        let mut meta_names: SortedVec<String> = SortedVec::new();
        for node in nodes.iter() {
            self.nodes.insert(node.meta_path.clone(), node.clone());
            meta_names.push(node.meta_path.clone());
        }
        let meta_names_set = match Set::from_iter(meta_names.iter()) {
            Ok(set) => set,
            Err(e) => return Err(format!("Error creating set: {}", e)),
        };
        self.nodes_indexes.insert(filename.clone(), meta_names_set);
        Ok(())
    }

    pub async fn remove(&mut self, filename: &PathBuf) -> Result<(), String> {
        if let Some(meta_names) = self.nodes_indexes.remove(&filename) {
            for name in meta_names {
                self.nodes.remove(&name);
            }
        }
        Ok(())
    }

    pub async fn fuzzy_search(
        &mut self,
        query: &str,
        filename: &PathBuf,
        top_n: usize,
    ) -> Vec<SymbolDeclarationStruct> {
        // 1. Parse query by language
        // 2. get variable info, functions info, static data info
        // 3. make from the found info meta names to match nodes
        // 4. search meta names in the `self.nodes_indexes`, get the matched metanames (use Levenstain automation queries)
        // 5. retrieve by matched meta names symbols from `self.nodes`
        // 6. return nodes
    }
}
