use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::ast::structs::AstReferencesSearchResult;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::att_ast_definition::results2message;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum};

pub struct AttAstReference;

#[async_trait]
impl Tool for AttAstReference {
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

        let (gcx, _top_n) = {
            let ccx_locked = ccx.lock().await;
            (ccx_locked.global_context.clone(), ccx_locked.top_n)
        };

        let ast_mb = gcx.read().await.ast_module.clone();
        let ast = ast_mb.ok_or_else(|| "AST support is turned off".to_string())?;
        let res: AstReferencesSearchResult = ast.read().await.search_references(symbol.clone()).await?;
        if (res.declaration_exact_matches.len() + res.declaration_fuzzy_matches.len()) == 0 {
            return Err(format!("No definitions with the name `{}` or similar names were found in the project.", symbol).to_string());
        }
        let (mut messages, tool_message) = if !res.declaration_exact_matches.is_empty() {
            let mut tool_message = format!("Definitions found:\n").to_string();
            for r in res.declaration_exact_matches.iter() {
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
            if res.references_for_exact_matches.is_empty() {
                tool_message.push_str("There are 0 references found in workspace for those definitions.");
                (vec![], tool_message)
            } else {
                tool_message.push_str("References found in workspace for those definitions:\n");
                for r in res.references_for_exact_matches.iter() {
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
                let messages = results2message(&res.references_for_exact_matches, true)
                    .await
                    .into_iter().map(|x| ContextEnum::ContextFile(x))
                    .collect::<Vec<ContextEnum>>();
                (messages, tool_message)
            }
        } else {
            let mut tool_message = format!(
                "No definition with name `{}` found in the workspace.\nThere are definitions with similar names though:\n",
                symbol
            ).to_string();
            for r in res.declaration_fuzzy_matches.iter() {
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
            // tool_message.push_str("You can call the `reference` tool one more time with one of those names.");
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
