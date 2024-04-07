use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::Mutex as AMutex;
use crate::ast::ast_index::RequestSymbolType;

use crate::ast::structs::FileReferencesResult;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::call_validation::{ChatMessage, ContextFile};


fn results2message(result: &FileReferencesResult) -> ChatMessage {
    let simplified_symbols: Vec<ContextFile> = result.symbols.iter().map(|x| {
        // let path = format!("{:?}::", result.file_path).to_string();
        ContextFile {
            file_name: x.file_path.to_string_lossy().to_string(),
            file_content: format!("{:?}", x.symbol_type),
            line1: x.full_range.start_point.row + 1,
            line2: x.full_range.end_point.row + 1,
            symbol: "".to_string(),
            usefulness: 100.0
        }
    }).collect();
    ChatMessage {
        role: "simplified_symbol_declaration".to_string(),
        content: json!(simplified_symbols).to_string(),
    }
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
    async fn can_execute(&self, args: &Vec<String>, context: &AtCommandsContext) -> bool {
        let param = self.params.get(0).unwrap();
        if let Some(arg) = args.get(0) {
            if param.lock().await.is_value_valid(arg, context).await {
                return true;
            }
        }
        false
    }
    async fn execute(&self, _query: &String, args: &Vec<String>, _top_n: usize, context: &AtCommandsContext) -> Result<ChatMessage, String> {
        let can_execute = self.can_execute(args, context).await;
        if !can_execute {
            return Err("incorrect arguments".to_string());
        }

        let file_path = match args.get(0) {
            Some(x) => crate::files_in_workspace::canonical_path(&x),
            None => return Err("no file path".to_string()),
        };

        let ast = context.global_context.read().await.ast_module.clone();
        let x = match &ast {
            Some(ast) => {
                let doc = match context.global_context.read().await.documents_state.document_map.get(&file_path).cloned() {
                    Some(doc) => doc.read().await.clone(),
                    None => return Err(format!("file not found: {}", file_path.display()))
                };
                match ast.read().await.get_file_symbols(RequestSymbolType::All, &doc).await {
                    Ok(res) => Ok(results2message(&res)),
                    Err(err) => Err(err)
                }
            }
            None => Err("Ast module is not available".to_string())
        }; x
    }
}
