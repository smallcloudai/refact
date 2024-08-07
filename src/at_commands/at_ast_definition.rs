use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;

use crate::ast::structs::AstQuerySearchResult;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_params::AtParamSymbolPathQuery;
use crate::call_validation::{ContextFile, ContextEnum};
use tracing::info;
use crate::ast::ast_index::RequestSymbolType;
use crate::ast::ast_module::AstModule;
use crate::at_commands::execute_at::{AtCommandMember, correct_at_arg};


pub async fn results2message(result: &AstQuerySearchResult) -> Vec<ContextFile> {
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
            usefulness: res.usefulness,
            is_body_important: false
        });
    }
    symbols
}

async fn run_at_definition(ast: &Option<Arc<ARwLock<AstModule>>>, symbol: &String) -> Result<Vec<ContextFile>, String>
{
    return match &ast {
        Some(ast) => {
            match ast.read().await.search_by_fullpath(
                symbol.clone(),
                RequestSymbolType::Declaration,
                false,
                10
            ).await {
                Ok(res) => {
                    Ok(results2message(&res).await)
                },
                Err(err) => {
                    Err(err)
                }
            }
        }
        None => {
            Err("Ast module is not available".to_string())
        }
    };
}


pub struct AtAstDefinition {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtAstDefinition {
    pub fn new() -> Self {
        AtAstDefinition {
            params: vec![
                Arc::new(AMutex::new(AtParamSymbolPathQuery::new()))
            ],
        }
    }
}

#[async_trait]
impl AtCommand for AtAstDefinition {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn at_execute(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        cmd: &mut AtCommandMember,
        args: &mut Vec<AtCommandMember>,
    ) -> Result<(Vec<ContextEnum>, String), String> {
        info!("execute @definition {:?}", args);
        let mut symbol = match args.get(0) {
            Some(x) => x.clone(),
            None => {
                cmd.ok = false;
                cmd.reason = Some("parameter is missing".to_string());
                args.clear();
                return Err("parameter `symbol` is missing".to_string());
            },
        };

        correct_at_arg(ccx.clone(), self.params[0].clone(), &mut symbol).await;
        args.clear();
        args.push(symbol.clone());

        let gcx = ccx.lock().await.global_context.clone();
        let ast = gcx.read().await.ast_module.clone();

        // TODO: don't produce files from fuzzy search, it's silly.
        let results = run_at_definition(&ast, &symbol.text).await?;
        let file_paths = results.iter().map(|x| x.file_name.clone()).collect::<Vec<_>>();
        let text = if let Some(path0) = file_paths.get(0) {
            let path = PathBuf::from(path0);
            let file_name = path.file_name().unwrap_or(OsStr::new(path0)).to_string_lossy();
            if file_paths.len() > 1 {
                format!("`{}` (defined in {} and other files)", &symbol.text, file_name)
            } else {
                format!("`{}` (defined in {})", &symbol.text, file_name)
            }
        } else {
            format!("`{}` (definition not found in AST tree)", &symbol.text)
        };
        Ok((results.into_iter().map(|x| ContextEnum::ContextFile(x)).collect::<Vec<ContextEnum>>(), text))
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
