use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Mutex as AMutex;
use tracing::info;
use tree_sitter::Point;

use crate::ast::structs::{AstCursorSearchResult, AstQuerySearchResult};
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_params::AtParamFilePathWithRow;
use crate::at_commands::utils::{get_file_text_from_disk, get_file_text_from_vecdb};
use crate::call_validation::{ChatMessage, ContextFile};
use crate::files_in_workspace::DocumentInfo;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SimplifiedSymbolDeclarationStruct {
    pub symbol_path: String,
    pub symbol_type: String,
    pub line1: usize,
    pub line2: usize,
}

async fn results2message(result: &AstCursorSearchResult) -> ChatMessage {
    info!("results2message {:?}", result);
    let mut symbols = vec![];
    for res in &result.search_results {
        let file_path: String = res.symbol_declaration.meta_path
            .split("::")
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .first()
            .cloned()
            .unwrap_or("".to_string());
        let content = res.symbol_declaration.get_content().await.unwrap_or("".to_string());
        symbols.push(ContextFile {
            file_name: file_path,
            file_content: content,
            line1: res.symbol_declaration.definition_info.range.start_point.row as i32,
            line2: res.symbol_declaration.definition_info.range.end_point.row as i32,
            usefullness: res.sim_to_query,
        });
    }
    ChatMessage {
        role: "context_file".to_string(),
        content: json!(symbols).to_string(),
    }
}

pub struct AtAstLookupSymbols {
    pub name: String,
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtAstLookupSymbols {
    pub fn new() -> Self {
        AtAstLookupSymbols {
            name: "@lookup_symbols_at".to_string(),
            params: vec![
                Arc::new(AMutex::new(AtParamFilePathWithRow::new()))
            ],
        }
    }
}

#[async_trait]
impl AtCommand for AtAstLookupSymbols {
    fn name(&self) -> &String {
        &self.name
    }
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }
    async fn are_args_valid(&self, args: &Vec<String>, context: &AtCommandsContext) -> Vec<bool> {
        let mut results = Vec::new();
        for (arg, param) in args.iter().zip(self.params.iter()) {
            let param = param.lock().await;
            results.push(param.is_value_valid(arg, context).await);
        }
        results
    }

    async fn can_execute(&self, args: &Vec<String>, context: &AtCommandsContext) -> bool {
        if self.are_args_valid(args, context).await.iter().any(|&x| x == false) || args.len() != self.params.len() {
            return false;
        }
        return true;
    }

    async fn execute(&self, _query: &String, args: &Vec<String>, _top_n: usize, context: &AtCommandsContext) -> Result<ChatMessage, String> {
        let can_execute = self.can_execute(args, context).await;
        if !can_execute {
            return Err("incorrect arguments".to_string());
        }
        info!("execute @lookup_symbols_at {:?}", args);

        let (file_path, row_idx_str) = match args.get(0) {
            Some(x) => {
                let mut parts = x.split(":");
                let file_path = parts.next().ok_or("missing file path")?;
                let row_idx = parts.next().ok_or("missing row index")?;
                (file_path, row_idx)
            }
            None => return Err("no file path".to_string()),
        };
        let row_idx: usize = row_idx_str.parse().map_err(|_| "row index is not a valid number")?;

        let file_text = get_file_text_from_disk(context.global_context.clone(), &file_path.to_string()).await?;
        let binding = context.global_context.read().await;
        let doc_info = match DocumentInfo::from_pathbuf_and_text(
            &PathBuf::from(file_path), &file_text,
        ) {
            Ok(doc) => doc,
            Err(err) => {
                return Err(format!("{err}: {file_path}"));
            }
        };
        let x = match *binding.ast_module.lock().await {
            Some(ref mut ast) => {
                match ast.search_references_by_cursor(
                    &doc_info, &file_text, Point { row: row_idx, column: 0 }, 5,
                ).await {
                    Ok(res) => Ok(results2message(&res).await),
                    Err(err) => Err(err)
                }
            }
            None => Err("Ast module is not available".to_string())
        };
        x
    }
}
