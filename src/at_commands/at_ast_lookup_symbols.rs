use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Mutex as AMutex;
use tracing::info;
use tree_sitter::Point;

use crate::ast::structs::AstCursorSearchResult;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_file::{AtParamFilePath, RangeKind, colon_lines_range_from_arg};
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::call_validation::{ChatMessage, ContextFile};
use crate::files_in_workspace::DocumentInfo;


#[derive(Debug, Serialize, Deserialize, Clone)]
struct SimplifiedSymbolDeclarationStruct {
    pub symbol_path: String,
    pub symbol_type: String,
    pub line1: usize,
    pub line2: usize,
}

pub async fn results2message(result: &AstCursorSearchResult) -> ChatMessage {
    // info!("results2message {:?}", result);
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
            line1: res.symbol_declaration.definition_info.range.start_point.row + 1,
            line2: res.symbol_declaration.definition_info.range.end_point.row + 1,
            usefulness: res.sim_to_query,
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
                Arc::new(AMutex::new(AtParamFilePath::new()))
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
    async fn can_execute(&self, args: &Vec<String>, context: &AtCommandsContext) -> bool {
        let param = self.params.get(0).unwrap();
        if let Some(arg) = args.get(0) {
            let mut arg_clone = arg.clone();
            colon_lines_range_from_arg(&mut arg_clone);
            if param.lock().await.is_value_valid(&arg_clone, context).await {
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
        info!("execute @lookup_symbols_at {:?}", args);

        let mut file_path = match args.get(0) {
            Some(x) => x.clone(),
            None => return Err("no file path".to_string()),
        };
        let row_idx = match colon_lines_range_from_arg(&mut file_path) {
            Some(x) => {
                if x.kind == RangeKind::GradToCursorTwosided {
                    x.line1
                } else {
                    return Err("line number is not a valid".to_string());
                }
            },
            None => return Err("line number is not a valid".to_string()),
        };

        let file_text = get_file_text_from_memory_or_disk(context.global_context.clone(), &file_path.to_string()).await?;
        let binding = context.global_context.read().await;
        let doc_info = match DocumentInfo::from_pathbuf_and_text(
            &PathBuf::from(&file_path), &file_text,
        ) {
            Ok(doc) => doc,
            Err(err) => {
                return Err(format!("{err}: {file_path}"));
            }
        };
        let x = match *binding.ast_module.lock().await {
            Some(ref mut ast) => {
                match ast.search_references_by_cursor(
                    &doc_info, &file_text, Point { row: row_idx, column: 0 }, 5, true
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
