use std::collections::HashMap;
use tracing::info;

use async_trait::async_trait;
use serde_json::Value;

use crate::at_commands::at_ast_lookup_symbols::{execute_at_ast_lookup_symbols, text_on_clip};
use crate::at_commands::at_commands::{AtCommandsContext, vec_context_file_to_context_tools};
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum};


pub struct AttAstLookupSymbols;

#[async_trait]
impl Tool for AttAstLookupSymbols {
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        info!("execute tool: lookup_symbols_at {:?}", args);
        let path = match args.get("path") {
            Some(Value::String(s)) => s,
            Some(v) => { return Err(format!("argument `path` is not a string: {:?}", v)) },
            None => { return Err("argument `path` is missing".to_string()) }
        };
        let line_n = match args.get("line_number") {
            Some(Value::Number(n)) if n.is_u64() => Some(n.as_u64().unwrap() as usize),
            Some(v) => return Err(format!("argument `line_number` is not a valid u64: {:?}", v)),
            None => return Err("no line_number".to_string()),
        };

        let results = execute_at_ast_lookup_symbols(ccx, &path, line_n.unwrap()).await?;
        let text = text_on_clip(&results, false);
        let mut results = vec_context_file_to_context_tools(results);
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: text,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
        }));

        Ok(results)
    }
    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
