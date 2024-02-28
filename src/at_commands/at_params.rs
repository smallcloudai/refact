use std::collections::HashSet;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use itertools::Itertools;
use strsim::{normalized_damerau_levenshtein, jaro_winkler};
use tokio::sync::RwLock as ARwLock;
use url::Url;

use crate::at_commands::at_commands::{AtCommandsContext, AtParam};
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

// from vecdb or from memory or from jsonl
async fn get_file_paths_from_anywhere(global_context: Arc<ARwLock<GlobalContext>>) -> Vec<String> {
    let file_paths_from_memory = global_context.read().await.documents_state.document_map.read().await.keys().cloned().collect::<Vec<Url>>();

    let file_paths_from_vecdb = match *global_context.read().await.vec_db.lock().await {
        Some(ref db) => {
            let index_file_paths = db.get_indexed_file_paths().await;
            let index_file_paths = index_file_paths.lock().await.deref().clone();
            index_file_paths.iter().map(|f| f.to_str().unwrap().to_string()).collect()
        },
        None => vec![]
    };

    let paths_in_jsonl: Vec<String> = files_in_jsonl(global_context.clone()).await.iter_mut()
        .filter_map(|doc| {
            doc.uri.to_file_path().ok().and_then(|path| path.to_str().map(|s| s.to_string()))
        })
        .collect();

    file_paths_from_memory.into_iter()
        .filter_map(|f| f.to_file_path().ok())
        .filter_map(|x| x.to_str().map(|x| x.to_string()))
        .chain(file_paths_from_vecdb.into_iter())
        .chain(paths_in_jsonl.into_iter())
        .collect::<HashSet<_>>() // dedup
        .into_iter()
        .collect()
}

async fn get_ast_file_paths(global_context: Arc<ARwLock<GlobalContext>>) -> Vec<String> {
     match *global_context.read().await.ast_module.lock().await {
        Some(ref ast) => {
            let index_file_paths = ast.get_indexed_file_paths().await;
            index_file_paths.iter().map(|f| f.to_str().unwrap().to_string()).collect()
        },
        None => vec![]
    }
}

#[async_trait]
impl AtParam for AtParamFilePath {
    fn name(&self) -> &String {
        &self.name
    }
    async fn is_value_valid(&self, value: &String, context: &AtCommandsContext) -> bool {
        get_file_paths_from_anywhere(context.global_context.clone()).await.contains(&value)
    }
    async fn complete(&self, value: &String, context: &AtCommandsContext, top_n: usize) -> Vec<String> {
        let index_file_paths = get_file_paths_from_anywhere(context.global_context.clone()).await;

        let mapped_paths = index_file_paths.iter().map(|f| {
            let path = PathBuf::from(f);
            (
                f,
                normalized_damerau_levenshtein(
                    if value.starts_with("/") {
                        f
                    } else {
                        path.file_name().unwrap().to_str().unwrap()
                    },
                    &value.to_string(),
                )
            )
        });

        let sorted_paths = mapped_paths
            .sorted_by(|(_, dist1), (_, dist2)| dist1.partial_cmp(dist2).unwrap())
            .rev()
            .map(|(path, _)| path.clone())
            .take(top_n)
            .collect::<Vec<String>>();
        sorted_paths
    }
}


#[derive(Debug)]
pub struct AtParamFilePathWithRow {
    pub name: String,
}

impl AtParamFilePathWithRow {
    pub fn new() -> Self {
        Self {
            name: "file_path".to_string()
        }
    }
}

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
        let index_paths = match *ast_module_ptr.lock().await {
            Some(ref ast) => ast.get_indexed_symbol_paths().await,
            None => vec![]
        };

        let value_lower = value.to_lowercase();
        let mapped_paths = index_paths
            .iter()
            .filter(|x| x.to_lowercase().contains(&value_lower))
            .map(|f| {
                let filename = f.split("::").dropping(1).into_iter().join("::");
                (
                    f,
                    jaro_winkler(
                        if value.starts_with("/") {
                            f
                        } else {
                            &filename
                        },
                        &value.to_string(),
                    )
                )
            });
        let sorted_paths = mapped_paths
            .sorted_by(|(_, dist1), (_, dist2)| dist1.partial_cmp(dist2).unwrap())
            .rev()
            .map(|(path, _)| path.clone())
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
            Some(ref ast) => ast.get_indexed_references().await,
            None => vec![]
        };
        let value_lower = value.to_lowercase();
        let mapped_paths = index_paths
            .iter()
            .filter(|x| x.to_lowercase().contains(&value_lower))
            .map(|f| {
                let filename = f.split("::").dropping(1).into_iter().join("::");
                (
                    f,
                    jaro_winkler(
                        if value.starts_with("/") {
                            f
                        } else {
                            &filename
                        },
                        &value.to_string(),
                    )
                )
            });
        let sorted_paths = mapped_paths
            .sorted_by(|(_, dist1), (_, dist2)| dist1.partial_cmp(dist2).unwrap())
            .rev()
            .map(|(path, _)| path.clone())
            .take(top_n)
            .collect::<Vec<String>>();
        return sorted_paths;
    }
    fn complete_if_valid(&self) -> bool {
        true
    }
}
