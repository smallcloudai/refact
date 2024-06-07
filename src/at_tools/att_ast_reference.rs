use std::collections::HashMap;
use serde_json::Value;
use tracing::info;

use async_trait::async_trait;

use crate::at_commands::at_ast_reference::{execute_at_ast_reference, text_on_clip};
use crate::at_commands::at_commands::{AtCommandsContext, vec_context_file_to_context_tools};
use crate::at_tools::at_tools::AtTool;
use crate::call_validation::{ChatMessage, ContextEnum};


pub struct AttAstReference;

#[async_trait]
impl AtTool for AttAstReference {
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        info!("execute @references {:?}", args);
        let mut symbol = match args.get("symbol") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => { return Err(format!("argument `symbol` is not a string: {:?}", v)) },
            None => { return Err("argument `symbol` is missing".to_string()) }
        };

        if let Some(dot_index) = symbol.find('.') {
            symbol = symbol[dot_index+1..].to_string();
        }

        let (query_result, refs_n) = execute_at_ast_reference(ccx, &symbol).await?;

        let mut results = vec_context_file_to_context_tools(query_result);
        let text = text_on_clip(&symbol, refs_n);

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
