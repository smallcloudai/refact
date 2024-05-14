use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use uuid::Uuid;
use crate::ast::ast_index::RequestSymbolType;

use crate::ast::structs::FileReferencesResult;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::call_validation::ContextFile;


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

pub struct AtAstFileSymbols {
    pub name: String,
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

#[async_trait]
impl AtCommand for AtAstFileSymbols {
    fn name(&self) -> &String {
        &self.name
    }
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }
    async fn execute(&self, _query: &String, args: &Vec<String>, _top_n: usize, context: &AtCommandsContext) -> Result<(Vec<ContextFile>, String), String> {
        let cpath = match args.get(0) {
            Some(x) => crate::files_correction::canonical_path(&x),
            None => return Err("no file path".to_string()),
        };

        let ast = context.global_context.read().await.ast_module.clone();
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
        x.map(|i|(i, "".to_string()))
    }
    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
