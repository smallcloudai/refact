use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use uuid::Uuid;
use crate::ast::ast_index::RequestSymbolType;

use crate::ast::structs::FileReferencesResult;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam, vec_context_file_to_context_tools};
use crate::call_validation::{ContextFile, ContextEnum};


fn results2message(result: &FileReferencesResult) -> Vec<ContextFile> {
    let simplified_symbols: Vec<ContextFile> = result.symbols.iter().map(|x| {
        // let path = format!("{:?}::", result.file_path).to_string();
        ContextFile {
            file_name: x.file_path.to_string_lossy().to_string(),
            file_content: format!("{:?}", x.symbol_type),
            line1: x.full_range.start_point.row + 1,
            line2: x.full_range.end_point.row + 1,
            symbol: Uuid::default(),
            gradient_type: -1,
            usefulness: 100.0,
            is_body_important: false
        }
    }).collect();
    simplified_symbols
}

fn text_on_clip(results: &Vec<ContextFile>, from_tool_call: bool) -> String {
    if !from_tool_call {
        return "".to_string();
    }
    let paths = results.iter().map(|x| x.file_name.clone()).collect::<Vec<_>>();
    if paths.is_empty() {
        return "".to_string();
    }
    return if paths.len() == 1 {
        format!("symbols defined in {}", paths[0])
    } else {
        format!("symbols defined in {} and other", paths[0])
    }
}

pub struct AtAstFileSymbols {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

#[async_trait]
impl AtCommand for AtAstFileSymbols {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }
    async fn execute(&self, ccx: &mut AtCommandsContext, _query: &String, args: &Vec<String>, _opt_args: &Vec<String>) -> Result<(Vec<ContextEnum>, String), String> {
        let cpath = match args.get(0) {
            Some(x) => crate::files_correction::canonical_path(&x),
            None => return Err("no file path".to_string()),
        };

        let ast = ccx.global_context.read().await.ast_module.clone();
        let x = match &ast {
            Some(ast) => {
                let doc = crate::files_in_workspace::Document { path: cpath, text: None };
                match ast.read().await.get_file_symbols(RequestSymbolType::All, &doc).await {
                    Ok(res) => Ok(results2message(&res)),
                    Err(err) => Err(err)
                }
            }
            None => Err("Ast module is not available".to_string())
        };
        let text = x.clone().map(|x| text_on_clip(&x, false)).unwrap_or("".to_string());
        let context_tools_mb = x.map(|j|vec_context_file_to_context_tools(j));
        context_tools_mb.map(|i|(i, text))
    }
    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
