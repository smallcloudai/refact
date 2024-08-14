use crate::ast::ast_index::RequestSymbolType;
use crate::at_commands::at_commands::{AtCommandsContext, AtParam};
use async_trait::async_trait;
use itertools::Itertools;
use std::sync::Arc;
use strsim::jaro_winkler;
use tokio::sync::Mutex as AMutex;


#[derive(Debug)]
pub struct AtParamSymbolPathQuery;

impl AtParamSymbolPathQuery {
    pub fn new() -> Self {
        Self {}
    }
}

fn full_path_score(path: &str, query: &str) -> f32 {
    if jaro_winkler(&path, &query) <= 0.0 {
        return 0.0;
    }

    let mut score = 1.0;
    for query_comp in query.split("::") {
        for (idx, p) in path.split("::").collect::<Vec<_>>().into_iter().rev().enumerate() {
            let current_score = jaro_winkler(&query_comp, &p) as f32;
            // preliminary exit if we have a full match in the name
            if current_score >= 0.99 {
                return score;
            }
            score *= current_score * (1.0 / (idx + 1) as f32);
        }
    }
    score
}


// TODO: move to at_lookup_symbols
#[async_trait]
impl AtParam for AtParamSymbolPathQuery {
    async fn is_value_valid(
        &self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        value: &String,
    ) -> bool {
        !value.is_empty()
    }

    async fn param_completion(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        value: &String,
    ) -> Vec<String> {
        if value.is_empty() {
            return vec![];
        }
        let (gcx, top_n) = {
            let ccx_locked = ccx.lock().await;
            (ccx_locked.global_context.clone(), ccx_locked.top_n)
        };
        let ast = gcx.read().await.ast_module.clone();
        let names = match &ast {
            Some(ast) => ast.read().await.get_symbols_paths(RequestSymbolType::Declaration).await.unwrap_or_default(),
            None => vec![]
        };

        let value_lower = value.to_lowercase();
        let mapped_paths = names
            .iter()
            .filter(|x| x.to_lowercase().contains(&value_lower) && !x.is_empty())
            .map(|f| (f, full_path_score(&f, &value.to_string())));
        let sorted_paths = mapped_paths
            .sorted_by(|(_, dist1), (_, dist2)| dist1.partial_cmp(dist2).unwrap())
            .rev()
            .map(|(s, _)| s.clone())
            .take(top_n)
            .collect::<Vec<String>>();
        return sorted_paths;
    }

    fn param_completion_valid(&self) -> bool {
        true
    }
}
