use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use itertools::Itertools;
use strsim::{normalized_damerau_levenshtein, jaro_winkler};
use tokio::sync::RwLock as ARwLock;
use url::Url;
use crate::ast::ast_index::RequestSymbolType;

use crate::at_commands::at_commands::{AtCommandsContext, AtParam};
use crate::at_commands::at_file::{colon_lines_range_from_arg, range_print, ColonLinesRange};
use crate::files_in_jsonl::files_in_jsonl;
use crate::global_context::GlobalContext;

#[derive(Debug)]
pub struct AtParamFilePath {
    pub name: String,
}

impl AtParamFilePath {
    pub fn new() -> Self {
        Self {
            name: "file_path".to_string()
        }
    }
}

async fn get_file_paths_from_anywhere(global_context: Arc<ARwLock<GlobalContext>>) -> Vec<String> {
    let file_paths_from_memory = global_context.read().await.documents_state.document_map.read().await.keys().cloned().collect::<Vec<Url>>();

    let urls_from_workspace: Vec<Url> = global_context.read().await.documents_state.workspace_files.lock().unwrap().clone();
    let paths_from_workspace: Vec<String> = urls_from_workspace.iter()
        .filter_map(|x| x.to_file_path().ok().and_then(|path| path.to_str().map(|s| s.to_string())))
        .collect();

    let paths_in_jsonl: Vec<String> = files_in_jsonl(global_context.clone()).await.iter_mut()
        .filter_map(|doc| {
            doc.uri.to_file_path().ok().and_then(|path| path.to_str().map(|s| s.to_string()))
        })
        .collect();

    file_paths_from_memory.into_iter()
        .filter_map(|f| f.to_file_path().ok())
        .filter_map(|x| x.to_str().map(|x| x.to_string()))
        .chain(paths_from_workspace.into_iter())
        .chain(paths_in_jsonl.into_iter())
        .collect::<HashSet<_>>() // dedup
        .into_iter()
        .collect()
}

async fn get_ast_file_paths(global_context: Arc<ARwLock<GlobalContext>>) -> Vec<String> {
     match *global_context.read().await.ast_module.lock().await {
        Some(ref ast) => {
            let index_file_paths = ast.get_file_paths().await.unwrap_or_default();
            index_file_paths.iter().map(|f| f
                .to_file_path()
                .unwrap_or_default()
                .to_path_buf()
                .to_str()
                .unwrap_or_default()
                .to_string()
            ).collect()
        },
        None => vec![]
    }
}

fn put_colon_back_to_arg(value: &mut String, colon: &Option<ColonLinesRange>) {
    if let Some(colon) = colon {
        value.push_str(":");
        value.push_str(range_print(colon).as_str());
    }
}

// TODO: move to at_file
#[async_trait]
impl AtParam for AtParamFilePath {
    fn name(&self) -> &String {
        &self.name
    }

    async fn is_value_valid(&self, value: &String, context: &AtCommandsContext) -> bool {
        let mut value = value.clone();
        colon_lines_range_from_arg(&mut value);
        get_file_paths_from_anywhere(context.global_context.clone()).await.contains(&value)
    }

    async fn complete(&self, value: &String, context: &AtCommandsContext, top_n: usize) -> Vec<String> {
        let mut correction_candidate = value.clone();
        let colon_mb = colon_lines_range_from_arg(&mut correction_candidate);
        tracing::info!("correction_candidate: {}", correction_candidate);

        let index_file_paths = get_file_paths_from_anywhere(context.global_context.clone()).await;

        let mapped_paths = index_file_paths.iter().map(|f| {
            let path = PathBuf::from(f);
            (
                f,
                normalized_damerau_levenshtein(
                    if correction_candidate.starts_with("/") {
                        f
                    } else {
                        path.file_name().unwrap().to_str().unwrap()
                    },
                    &correction_candidate.to_string(),
                )
            )
        });

        let sorted_paths = mapped_paths
            .sorted_by(|(_, dist1), (_, dist2)| dist1.partial_cmp(dist2).unwrap())
            .rev()
            .map(|(p, _)| p.clone())
            .take(top_n)
            .map(|mut x| { put_colon_back_to_arg(&mut x, &colon_mb); x.clone() })
            .collect::<Vec<String>>();
        tracing::info!("sorted_paths: {:?}", sorted_paths);
        sorted_paths
    }
}


#[derive(Debug)]
pub struct AtParamFilePathWithRow {
    pub name: String,
}

// impl AtParamFilePathWithRow {
//     pub fn new() -> Self {
//         Self {
//             name: "file_path".to_string()
//         }
//     }
// }

#[derive(Debug)]
pub struct AtParamSymbolPathQuery {
    pub name: String,
}

impl AtParamSymbolPathQuery {
    pub fn new() -> Self {
        Self {
            name: "context_file".to_string()
        }
    }
}

// TODO: move to at_lookup_symbols
#[async_trait]
impl AtParam for AtParamSymbolPathQuery {
    fn name(&self) -> &String {
        &self.name
    }
    async fn is_value_valid(&self, _: &String, _: &AtCommandsContext) -> bool {
        return true;
    }

    async fn complete(&self, value: &String, context: &AtCommandsContext, top_n: usize) -> Vec<String> {
        let ast_module_ptr = context.global_context.read().await.ast_module.clone();
        let names = match *ast_module_ptr.lock().await {
            Some(ref ast) => ast.get_symbols_names(RequestSymbolType::Declaration).await.unwrap_or_default(),
            None => vec![]
        };

        let value_lower = value.to_lowercase();
        let mapped_paths = names
            .iter()
            .filter(|x| x.to_lowercase().contains(&value_lower))
            .map(|f| (f, jaro_winkler(&f, &value.to_string())));
        let sorted_paths = mapped_paths
            .sorted_by(|(_, dist1), (_, dist2)| dist1.partial_cmp(dist2).unwrap())
            .rev()
            .map(|(s, _)| s.clone())
            .take(top_n)
            .collect::<Vec<String>>();
        return sorted_paths;
    }

    fn complete_if_valid(&self) -> bool {
        true
    }
}


#[derive(Debug)]
pub struct AtParamSymbolReferencePathQuery {
    pub name: String,
}

impl AtParamSymbolReferencePathQuery {
    pub fn new() -> Self {
        Self {
            name: "context_file".to_string()
        }
    }
}

#[async_trait]
impl AtParam for AtParamSymbolReferencePathQuery {
    fn name(&self) -> &String {
        &self.name
    }
    async fn is_value_valid(&self, _: &String, _: &AtCommandsContext) -> bool {
        return true;
    }
    async fn complete(&self, value: &String, context: &AtCommandsContext, top_n: usize) -> Vec<String> {
        let ast_module_ptr = context.global_context.read().await.ast_module.clone();
        let index_paths = match *ast_module_ptr.lock().await {
            Some(ref ast) => ast.get_symbols_names(RequestSymbolType::Usage).await.unwrap_or_default(),
            None => vec![]
        };
        let value_lower = value.to_lowercase();
        let mapped_paths = index_paths
            .iter()
            .filter(|x| x.to_lowercase().contains(&value_lower))
            .map(|f| (f, jaro_winkler(&f, &value.to_string())));
        let sorted_paths = mapped_paths
            .sorted_by(|(_, dist1), (_, dist2)| dist1.partial_cmp(dist2).unwrap())
            .rev()
            .map(|(s, _)| s.clone())
            .take(top_n)
            .collect::<Vec<String>>();
        return sorted_paths;
    }
    fn complete_if_valid(&self) -> bool {
        true
    }
}
