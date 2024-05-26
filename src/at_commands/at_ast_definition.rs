use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;

use crate::ast::structs::AstQuerySearchResult;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam, vec_context_file_to_context_tools};
use crate::at_commands::at_params::AtParamSymbolPathQuery;
use crate::call_validation::{ChatMessage, ContextFile, ContextEnum};
use tracing::info;
use crate::ast::ast_index::RequestSymbolType;
use crate::ast::ast_module::AstModule;


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
            usefulness: res.usefulness,
            is_body_important: false
        });
    }
    symbols
}

async fn run_at_definition(ast: &Option<Arc<ARwLock<AstModule>>>, the_arg: &String) -> Result<Vec<ContextFile>, String>
{
    match &ast {
        Some(ast) => {
            match ast.read().await.search_by_name(
                the_arg.clone(),
                RequestSymbolType::Declaration,
                true,
                10
            ).await {
                Ok(res) => {
                    return Ok(results2message(&res).await);
                },
                Err(err) => {
                    return Err(err);
                }
            }
        }
        None => {
            return Err("Ast module is not available".to_string());
        }
    };
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
            format!("`{}` (defined in {} and other files)", symbol_path, file_name)
        } else {
            format!("`{}` (defined in {})", symbol_path, file_name)
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

    async fn execute_as_at_command(&self, ccx: &mut AtCommandsContext, query: &String, args: &Vec<String>) -> Result<(Vec<ContextEnum>, String), String> {
        info!("execute @definition {:?}", args);
        let the_arg = match args.get(0) {
            Some(x) => x.clone(),
            None => return Err("no symbol path".to_string()),
        };
        let ast = ccx.global_context.read().await.ast_module.clone();
        let results = run_at_definition(&ast, &the_arg).await?;
        let text = text_on_clip(&the_arg, &results);
        Ok((vec_context_file_to_context_tools(results), text))
    }

    async fn execute_as_tool(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, serde_json::Value>) -> Result<Vec<ContextEnum>, String> {
        let the_arg_js = match args.get("symbol") {
            Some(x) => x,
            None => return Err("argument `symbol` is missing".to_string()),
        };
        let the_arg = match the_arg_js.as_str() {
            Some(x) => x.to_string(),
            None => return Err("argument `symbol` is not a string".to_string()),
        };
        let ast = ccx.global_context.read().await.ast_module.clone();
        let results_context_file = run_at_definition(&ast, &the_arg).await?;
        let text = text_on_clip(&the_arg, &results_context_file);
        let mut results = vec_context_file_to_context_tools(results_context_file);
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: text,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
        }));
        Ok(results)
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}

