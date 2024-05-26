use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;

use crate::ast::structs::AstQuerySearchResult;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam, vec_context_file_to_context_tools};
use crate::at_commands::at_params::AtParamSymbolReferencePathQuery;
use crate::call_validation::{ContextFile, ContextEnum};
use tracing::info;
use crate::ast::ast_index::RequestSymbolType;


async fn results2message(result: &AstQuerySearchResult) -> Vec<ContextFile> {
    // info!("results2message {:?}", result);
    let mut symbols = vec![];
    for res in &result.search_results {
        let file_name = res.symbol_declaration.file_path.to_string_lossy().to_string();
        let content = res.symbol_declaration.get_content().await.unwrap_or("".to_string());
        symbols.push(ContextFile {
            file_name,
            file_content: content,
            line1: res.symbol_declaration.full_range.start_point.row + 1,
            line2: res.symbol_declaration.full_range.end_point.row + 1,
            symbol: res.symbol_declaration.guid.clone(),
            gradient_type: -1,
            usefulness: 0.5 * res.usefulness,
            is_body_important: true
        });
    }
    symbols
}

pub struct AtAstReference {
    pub name: String,
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtAstReference {
    pub fn new() -> Self {
        AtAstReference {
            name: "@references".to_string(),
            params: vec![
                Arc::new(AMutex::new(AtParamSymbolReferencePathQuery::new()))
            ],
        }
    }
}

#[async_trait]
impl AtCommand for AtAstReference {
    fn name(&self) -> &String {
        &self.name
    }

    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn execute_as_at_command(&self, ccx: &mut AtCommandsContext, _query: &String, args: &Vec<String>) -> Result<(Vec<ContextEnum>, String), String> {
        info!("execute @references {:?}", args);
        let symbol_path = match args.get(0) {
            Some(x) => x,
            None => return Err("no symbol path".to_string()),
        };
        let ast = ccx.global_context.read().await.ast_module.clone();
        let x = match &ast {
            Some(ast) => {
                match ast.read().await.search_by_name(
                    symbol_path.clone(),
                    RequestSymbolType::Usage,
                    true,
                    10
                ).await {
                    Ok(res) => Ok(results2message(&res).await),
                    Err(err) => Err(err)
                }
            }
            None => Err("Ast module is not available".to_string())
        };
        let x = x.map(|j|vec_context_file_to_context_tools(j));
        x.map(|i|(i, format!("\"usages of {}\"", symbol_path)))
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
