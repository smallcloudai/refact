use async_trait::async_trait;
use itertools::Itertools;
use strsim::jaro_winkler;
use crate::at_commands::at_commands::{AtCommandsContext, AtParam};


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
