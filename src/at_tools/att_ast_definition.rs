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
        let mut symbol = match args.get("symbol") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => { return Err(format!("argument `symbol` is not a string: {:?}", v)) },
            None => { return Err("argument `symbol` is missing".to_string()) }
        };

        if let Some(dot_index) = symbol.find('.') {
            symbol = symbol[dot_index+1..].to_string();
        }

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

