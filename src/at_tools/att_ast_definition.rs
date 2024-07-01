use std::cmp::max;
use std::collections::HashMap;

use async_trait::async_trait;
use itertools::Itertools;
use serde_json::Value;

use crate::ast::ast_index::RequestSymbolType;
use crate::ast::structs::AstQuerySearchResult;
use crate::at_commands::at_ast_definition::results2message;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::tools::AtTool;
use crate::call_validation::{ChatMessage, ContextEnum};

pub struct AttAstDefinition;

#[async_trait]
impl AtTool for AttAstDefinition {
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let mut symbol = match args.get("symbol") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => { return Err(format!("argument `symbol` is not a string: {:?}", v)) }
            None => { return Err("argument `symbol` is missing".to_string()) }
        };

        if let Some(dot_index) = symbol.find('.') {
            symbol = symbol[dot_index + 1..].to_string();
        }

        let ast_mb = ccx.global_context.read().await.ast_module.clone();
        let ast = ast_mb.ok_or_else(|| "AST support is turned off".to_string())?;

        let mut found_by_fuzzy_search: bool = false;
        let mut res: AstQuerySearchResult = ast.read().await.search_by_fullpath(
            symbol.clone(),
            RequestSymbolType::Declaration,
            false,
            ccx.top_n,
        ).await?;
        res = if res.search_results.is_empty() {
            found_by_fuzzy_search = true;
            ast.read().await.search_by_fullpath(
                symbol.clone(),
                RequestSymbolType::Declaration,
                true,
                max(ccx.top_n, 6),
            ).await?
        } else {
            res
        };
        if res.search_results.is_empty() {
            return Err(format!("There is no `{}` in the syntax tree, and no similar names found :/", symbol).to_string());
        }

        let (mut messages, tool_message) = if found_by_fuzzy_search {
            let messages = results2message(&res)
                .await
                .into_iter()
                .map(|x| ContextEnum::ContextFile(x))
                .take(1)
                .collect::<Vec<ContextEnum>>();
            let found_path = res.search_results[0].symbol_declaration.symbol_path.clone();
            let other_names = res.search_results
                .iter()
                .skip(1)
                .map(|r| r.symbol_declaration.symbol_path.clone())
                .sorted()
                .unique()
                .collect::<Vec<String>>();
            let mut tool_message = format!(
                "Definition of `{symbol}` haven't found by exact name, but found the close result`{found_path}`. \
                You can call again with one of these other names:\n"
            ).to_string();
            for x in other_names.into_iter() {
                tool_message.push_str(&format!("{}\n", x));
            }
            (messages, tool_message)
        } else {
            let messages = results2message(&res)
                .await
                .into_iter().map(|x| ContextEnum::ContextFile(x))
                .collect::<Vec<ContextEnum>>();
            let mut tool_message = format!("Definition of `{}` found at:\n", symbol).to_string();
            for r in res.search_results.iter() {
                let file_path_str = r.symbol_declaration.symbol_path.to_string();
                let decl_range = &r.symbol_declaration.full_range;
                tool_message.push_str(&format!("{}:{}-{}\n", file_path_str, decl_range.start_point.row + 1, decl_range.end_point.row + 1));
            }
            (messages, tool_message)
        };

        messages.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: tool_message.clone(),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
        }));
        Ok(messages)
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}

