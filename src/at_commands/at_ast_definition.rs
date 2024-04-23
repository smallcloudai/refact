use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;

use crate::ast::structs::AstQuerySearchResult;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_params::AtParamSymbolPathQuery;
use crate::call_validation::ContextFile;
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
            usefulness: res.usefulness
        });
    }
    symbols
}

pub struct AtAstDefinition {
    pub name: String,
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtAstDefinition {
    pub fn new() -> Self {
        AtAstDefinition {
            name: "@definition".to_string(),
            params: vec![
                Arc::new(AMutex::new(AtParamSymbolPathQuery::new()))
            ],
        }
    }
}

fn text_on_clip(symbol_path: &String, results: &Vec<ContextFile>) -> String {
    let file_paths = results.iter().map(|x| x.file_name.clone()).collect::<Vec<_>>();
    if let Some(path0) = file_paths.get(0) {
        let path = PathBuf::from(path0);
        let file_name = path.file_name().unwrap_or(OsStr::new(path0)).to_string_lossy();
        if file_paths.len() > 1 {
            format!("{} (defined in {} and other files)", symbol_path, file_name)
        } else {
            format!("{} (defined in {})", symbol_path, file_name)
        }
    } else {
        symbol_path.clone()
    }
}

#[async_trait]
impl AtCommand for AtAstDefinition {
    fn name(&self) -> &String {
        &self.name
    }
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }
    async fn execute(&self, _query: &String, args: &Vec<String>, _top_n: usize, context: &AtCommandsContext) -> Result<(Vec<ContextFile>, String), String> {
        info!("execute @definition {:?}", args);
        let symbol_path = match args.get(0) {
            Some(x) => x,
            None => return Err("no symbol path".to_string()),
        };
        let ast = context.global_context.read().await.ast_module.clone();
        let x = match &ast {
            Some(ast) => {
                match ast.read().await.search_by_name(
                    symbol_path.clone(),
                    RequestSymbolType::Declaration,
                    true,
                    10
                ).await {
                    Ok(res) => {
                        Ok(results2message(&res).await)
                    },
                    Err(err) => Err(err)
                }
            }
            None => Err("Ast module is not available".to_string())
        };
        x.map(|x| (x.clone(), text_on_clip(symbol_path, &x)))
    }
    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
