use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::Mutex as AMutex;

use crate::ast::structs::FileReferencesResult;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_file::AtParamFilePath;
use crate::call_validation::{ChatMessage, ContextFile};
use crate::files_in_workspace::DocumentInfo;

fn results2message(result: &FileReferencesResult) -> ChatMessage {
    let simplified_symbols: Vec<ContextFile> = result.symbols.iter().map(|x| {
        let path = format!("{:?}::", result.file_path).to_string();
        ContextFile {
            file_name: x.meta_path.replace(path.as_str(), ""),
            file_content: format!("{:?}", x.symbol_type),
            line1: x.definition_info.range.start_point.row + 1,
            line2: x.definition_info.range.end_point.row + 1,
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

impl AtAstFileSymbols {
    pub fn new() -> Self {
        AtAstFileSymbols {
            name: "@symbols-at".to_string(),
            params: vec![
                Arc::new(AMutex::new(AtParamFilePath::new()))
            ],
        }
    }
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
            Some(x) => x,
            None => return Err("no file path".to_string()),
        };

        let binding = context.global_context.read().await;
        let x = match *binding.ast_module.lock().await {
            Some(ref ast) => {
                let doc = match DocumentInfo::from_pathbuf(&PathBuf::from(file_path)).ok() {
                    Some(doc) => doc,
                    None => return Err("file not found".to_string())
                };
                match ast.get_file_symbols(&doc).await {
                    Ok(res) => Ok(results2message(&res)),
                    Err(err) => Err(err)
                }
            }
            None => Err("Ast module is not available".to_string())
        }; x
    }
}
