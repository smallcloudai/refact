use async_trait::async_trait;
use itertools::Itertools;
use strsim::jaro_winkler;
use crate::ast::ast_index::RequestSymbolType;
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

    async fn is_value_valid(&self, value: &String, _: &AtCommandsContext) -> bool {
        !value.is_empty()
    }

    async fn complete(&self, value: &String, ccx: &AtCommandsContext) -> Vec<String> {
        if value.is_empty() {
            return vec![];
        }
        let ast = ccx.global_context.read().await.ast_module.clone();
        let names = match &ast {
            Some(ast) => ast.read().await.get_symbols_names(RequestSymbolType::Declaration).await.unwrap_or_default(),
            None => vec![]
        };

        let value_lower = value.to_lowercase();
        let mapped_paths = names
            .iter()
            .filter(|x| x.to_lowercase().contains(&value_lower) && !x.is_empty())
            .map(|f| (f, jaro_winkler(&f, &value.to_string())));
        let sorted_paths = mapped_paths
            .sorted_by(|(_, dist1), (_, dist2)| dist1.partial_cmp(dist2).unwrap())
            .rev()
            .map(|(s, _)| s.clone())
            .take(ccx.top_n)
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

    async fn complete(&self, value: &String, ccx: &AtCommandsContext) -> Vec<String> {
        let ast = ccx.global_context.read().await.ast_module.clone();
        let index_paths = match &ast {
            Some(ast) => ast.read().await.get_symbols_names(RequestSymbolType::Usage).await.unwrap_or_default(),
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
            .take(ccx.top_n)
            .collect::<Vec<String>>();
        return sorted_paths;
    }
    fn complete_if_valid(&self) -> bool {
        true
    }
}
