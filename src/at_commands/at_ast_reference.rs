use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;

use crate::ast::structs::AstQuerySearchResult;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam, vec_context_file_to_context_tools};
use crate::at_commands::at_params::AtParamSymbolReferencePathQuery;
use crate::call_validation::{ContextFile, ContextEnum};
use tracing::info;
use crate::ast::ast_index::RequestSymbolType;
use crate::at_commands::execute_at::{AtCommandMember, correct_at_arg};


async fn results2message(result: &AstQuerySearchResult) -> Vec<ContextFile> {
    // info!("results2message {:?}", result);
    let mut symbols = vec![];
    for res in &result.search_results {
        let file_name = res.symbol_declaration.file_path.to_string_lossy().to_string();
        let content = res.symbol_declaration.get_content_from_file().await.unwrap_or("".to_string());
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
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtAstReference {
    pub fn new() -> Self {
        AtAstReference {
            params: vec![
                Arc::new(AMutex::new(AtParamSymbolReferencePathQuery::new()))
            ],
        }
    }
}

pub async fn execute_at_ast_reference(
    ccx: Arc<AMutex<AtCommandsContext>>,
    symbol_path: &String,
) -> Result<(Vec<ContextFile>, usize), String> {
    let gcx = ccx.lock().await.global_context.clone();
    let ast = gcx.read().await.ast_module.clone();
    let x = match &ast {
        Some(ast) => {
            match ast.read().await.search_by_fullpath(
                symbol_path.clone(),
                RequestSymbolType::Usage,
                false,
                10
            ).await {
                Ok(res) => Ok((results2message(&res).await, res.refs_n)),
                Err(err) => Err(err)
            }
        }
        None => Err("Ast module is not available".to_string())
    };
    x
}

#[async_trait]
impl AtCommand for AtAstReference {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn at_execute(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        cmd: &mut AtCommandMember,
        args: &mut Vec<AtCommandMember>,
    ) -> Result<(Vec<ContextEnum>, String), String> {
        info!("execute @references {:?}", args);
        let mut symbol = match args.get(0) {
            Some(x) => x.clone(),
            None => {
                cmd.ok = false; cmd.reason = Some("no symbol path".to_string());
                args.clear();
                return Err("no symbol path".to_string());
            },
        };

        correct_at_arg(ccx.clone(), self.params[0].clone(), &mut symbol).await;
        args.clear();
        args.push(symbol.clone());

        let (query_result, refs_n) = execute_at_ast_reference(ccx.clone(), &symbol.text).await?;
        let results = vec_context_file_to_context_tools(query_result);
        let text = format!("`{}` (found {} usages)", symbol.text, refs_n);

        Ok((results, text))
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
