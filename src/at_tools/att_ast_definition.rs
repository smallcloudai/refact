use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use crate::at_commands::at_ast_definition::{run_at_definition, text_on_clip};

use crate::at_commands::at_commands::{AtCommandsContext, vec_context_file_to_context_tools};
use crate::call_validation::{ChatMessage, ContextEnum};
use crate::at_tools::at_tools::AtTool;


pub struct AttAstDefinition;

#[async_trait]
impl AtTool for AttAstDefinition {
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let symbol_raw = match args.get("symbol") {
            Some(x) => x,
            None => return Err("argument `symbol` is missing".to_string()),
        };
        let symbol = match symbol_raw.as_str() {
            Some(x) => x.to_string(),
            None => return Err("argument `symbol` is not a string".to_string()),
        };
        let ast = ccx.global_context.read().await.ast_module.clone();
        let vector_of_context_file = run_at_definition(&ast, &symbol).await?;
        let text = text_on_clip(&symbol, &vector_of_context_file);
        
        let mut results = vec_context_file_to_context_tools(vector_of_context_file);
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: text.clone(),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
        }));
        Ok(results)
    }
    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}

