use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;
use itertools::Itertools;

use crate::at_commands::at_ast_definition::results2message;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ContextEnum};
use crate::at_tools::at_tools::AtTool;
use crate::ast::ast_index::RequestSymbolType;


pub struct AttAstDefinition;

#[async_trait]
impl AtTool for AttAstDefinition {
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let mut symbol = match args.get("symbol") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => { return Err(format!("argument `symbol` is not a string: {:?}", v)) },
            None => { return Err("argument `symbol` is missing".to_string()) }
        };

        if let Some(dot_index) = symbol.find('.') {
            symbol = symbol[dot_index+1..].to_string();
        }

        let ast_mb = ccx.global_context.read().await.ast_module.clone();
        let ast = ast_mb.ok_or_else(|| "AST support is turned off".to_string())?;

        let search_results: crate::ast::structs::AstQuerySearchResult = ast.read().await.search_by_name(
            symbol.clone(),
            RequestSymbolType::Declaration,
            false,
            ccx.top_n,
        ).await?;
        if search_results.search_results.len() == 0 {
            let search_results_fuzzy: crate::ast::structs::AstQuerySearchResult = ast.read().await.search_by_name(
                symbol.clone(),
                RequestSymbolType::Declaration,
                true,
                6,
            ).await?;
            if search_results_fuzzy.search_results.len() == 0 {
                return Err("There is no `{}` in the syntax tree, and no similar names found :/".to_string());
            } else {
                let mut s = String::new();
                s.push_str("There is no `{}` in the syntax tree, call again with one of these close names:\n");
                let all_names_unique = search_results_fuzzy.search_results.iter()
                    .map(|r| r.symbol_declaration.name.clone())
                    .sorted()
                    .unique()
                    .collect::<Vec<String>>();
                for x in all_names_unique.iter() {
                    s.push_str(&format!("{}\n", x));
                }
                return Err(s);
            }
        }
        let mut results = results2message(&search_results).await
            .into_iter().map(|x| ContextEnum::ContextFile(x)).collect::<Vec<ContextEnum>>();

        let mut s = String::new();
        s.push_str(format!("Definition of `{}` found at:\n", symbol).as_str());
        for r in search_results.search_results.iter() {
            let file_path_str = r.symbol_declaration.file_path.to_str().unwrap().to_string();
            let decl_range = &r.symbol_declaration.full_range;
            s.push_str(&format!("{}:{}-{}\n", file_path_str, decl_range.start_point.row + 1, decl_range.end_point.row + 1));
        }
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: s.clone(),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
        }));
        Ok(results)
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}

