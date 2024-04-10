use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::Mutex as AMutex;
use tracing::info;
use tree_sitter::Point;

use crate::ast::structs::AstCursorSearchResult;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_file::{AtParamFilePath, RangeKind, colon_lines_range_from_arg};
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::call_validation::{ChatMessage, ContextFile};

pub async fn results2message(result: &AstCursorSearchResult) -> ChatMessage {
    // info!("results2message {:?}", result);
    let mut fvec = vec![];
    for res in &result.declaration_symbols {
        let file_name = res.symbol_declaration.file_path.to_string_lossy().to_string();
        fvec.push(ContextFile {
            file_name,
            file_content: res.content.clone(),
            line1: res.symbol_declaration.full_range.start_point.row + 1,
            line2: res.symbol_declaration.full_range.end_point.row + 1,
            symbol: res.symbol_declaration.guid.clone(),
            gradient_type: -1,
            usefulness: 90.0,
        });
    }
    for res in &result.declaration_usage_symbols {
        let file_name = res.symbol_declaration.file_path.to_string_lossy().to_string();
        fvec.push(ContextFile {
            file_name,
            file_content: res.content.clone(),
            line1: res.symbol_declaration.full_range.start_point.row + 1,
            line2: res.symbol_declaration.full_range.end_point.row + 1,
            symbol: res.symbol_declaration.guid.clone(),
            gradient_type: -1,
            usefulness: 50.0,
        });
    }
    for res in &result.most_similar_declarations {
        let file_name = res.symbol_declaration.file_path.to_string_lossy().to_string();
        fvec.push(ContextFile {
            file_name,
            file_content: res.content.clone(),
            line1: res.symbol_declaration.full_range.start_point.row + 1,
            line2: res.symbol_declaration.full_range.end_point.row + 1,
            symbol: res.symbol_declaration.guid.clone(),
            gradient_type: -1,
            usefulness: 40.0,
        });
    }
    ChatMessage {
        role: "context_file".to_string(),
        content: json!(fvec).to_string(),
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
                if x.kind == RangeKind::GradToCursorTwoSided {
                    x.line1
                } else {
                    return Err("line number is not a valid".to_string());
                }
            },
            None => return Err("line number is not a valid".to_string()),
        };

        let cpath = crate::files_in_workspace::canonical_path(&file_path);
        let file_text = get_file_text_from_memory_or_disk(context.global_context.clone(), &cpath).await?;

        let mut doc = match context.global_context.read().await.documents_state.document_map.get(&cpath) {
            Some(d) => d.read().await.clone(),
            None => return Err("no document found".to_string()),
        };
        doc.update_text(&file_text);
        let ast = context.global_context.read().await.ast_module.clone();
        let x = match &ast {
            Some(ast) => {
                match ast.write().await.retrieve_cursor_symbols_by_declarations(
                    &doc, &file_text, Point { row: row_idx, column: 0 }, 15,  3
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
