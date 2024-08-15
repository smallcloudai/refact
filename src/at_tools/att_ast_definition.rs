use std::collections::HashMap;
use std::sync::Arc;

use crate::ast::structs::{AstDeclarationSearchResult, SymbolsSearchResultStruct};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum, ContextFile};
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;


pub struct AttAstDefinition;


pub async fn results2message(
    search_results: &Vec<SymbolsSearchResultStruct>,
    is_body_important: bool
) -> Vec<ContextFile> {
    // info!("results2message {:?}", result);
    let mut symbols = vec![];
    for res in search_results {
        let file_name = res.symbol_declaration.file_path.to_string_lossy().to_string();
        let content = res.symbol_declaration.get_content_from_file().await.unwrap_or("".to_string());
        symbols.push(ContextFile {
            file_name,
            file_content: content,
            line1: res.symbol_declaration.full_range.start_point.row + 1,
            line2: res.symbol_declaration.full_range.end_point.row + 1,
            symbols: vec![res.symbol_declaration.guid.clone()],
            gradient_type: -1,
            usefulness: res.usefulness,
            is_body_important
        });
    }
    symbols
}


#[async_trait]
impl Tool for AttAstDefinition {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<Vec<ContextEnum>, String> {
        let mut symbol = match args.get("symbol") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => { return Err(format!("argument `symbol` is not a string: {:?}", v)) }
            None => { return Err("argument `symbol` is missing".to_string()) }
        };

        if let Some(dot_index) = symbol.find('.') {
            symbol = symbol[dot_index + 1..].to_string();
        }

        let gcx = ccx.lock().await.global_context.clone();
        let ast_mb = gcx.read().await.ast_module.clone();
        let ast = ast_mb.ok_or_else(|| "AST support is turned off".to_string())?;
        let res: AstDeclarationSearchResult = ast.read().await.search_declarations(symbol.clone()).await?;
        if (res.exact_matches.len() + res.fuzzy_matches.len()) == 0 {
            return Err(format!("No definitions with the name `{}` or similar names were found in the workspace.", symbol).to_string());
        }
        let (mut messages, tool_message) = if !res.exact_matches.is_empty() {
            let messages = results2message(&res.exact_matches, false)
                .await
                .into_iter().map(|x| ContextEnum::ContextFile(x))
                .collect::<Vec<ContextEnum>>();
            let mut tool_message = format!("Definitions found:\n").to_string();
            for r in res.exact_matches.iter() {
                let file_path_str = r.symbol_declaration.file_path.to_string_lossy();
                let decl_range = &r.symbol_declaration.full_range;
                tool_message.push_str(&format!(
                    "`{}` at {}:{}-{}\n",
                    r.symbol_declaration.symbol_path,
                    file_path_str,
                    decl_range.start_point.row + 1,
                    decl_range.end_point.row + 1
                ));
            }
            (messages, tool_message)
        } else {
            let mut tool_message = format!(
                "No definitions with name `{}` found in the workspace.\nThere are definitions with similar names though:\n",
                symbol
            ).to_string();
            for r in res.fuzzy_matches.iter() {
                let file_path_str = r.symbol_declaration.file_path.to_string_lossy();
                let decl_range = &r.symbol_declaration.full_range;
                tool_message.push_str(&format!(
                    "`{}` at {}:{}-{}\n",
                    r.symbol_declaration.symbol_path,
                    file_path_str,
                    decl_range.start_point.row + 1,
                    decl_range.end_point.row + 1
                ));
            }
            (vec![], tool_message)
        };

        messages.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: tool_message.clone(),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));
        Ok(messages)
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}

