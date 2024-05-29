use std::collections::HashMap;
use tracing::info;

use async_trait::async_trait;
use serde_json::Value;

use crate::at_commands::at_ast_lookup_symbols::{execute_at_ast_lookup_symbols, text_on_clip};
use crate::at_commands::at_commands::{AtCommandsContext, vec_context_file_to_context_tools};
use crate::at_tools::at_tools::AtTool;
use crate::call_validation::{ChatMessage, ContextEnum};


pub struct AttAstLookupSymbols;

#[async_trait]
impl AtTool for AttAstLookupSymbols {
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        info!("execute tool: lookup_symbols_at {:?}", args);

        let file_path = match args.get("file_path") {
            Some(x) => x.to_string(),
            None => return Err("no file path".to_string()),
        };
        let line_n = match args.get("line_number") {
            Some(x) => x.as_u64().map(|x|x as usize),
            None => return Err("no line_number".to_string()),
        };
        if line_n.is_none() {
            return Err("line_number is incorrect".to_string());
        }
        
        let results = execute_at_ast_lookup_symbols(ccx, &file_path, line_n.unwrap()).await?;
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
