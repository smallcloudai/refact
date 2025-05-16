use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tracing::info;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::tools::tools_description::Tool;

pub struct ToolCreateKnowledge;

#[async_trait]
impl Tool for ToolCreateKnowledge {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        info!("run @create-knowledge with args: {:?}", args);
        let gcx = {
            let ccx_locked = ccx.lock().await;
            ccx_locked.global_context.clone()
        };
        let knowledge_entry = match args.get("knowledge_entry") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `knowledge_entry` is not a string: {:?}", v)),
            None => return Err("argument `knowledge_entry` is missing".to_string())
        };
        crate::memories::memories_add(
            gcx.clone(),
            "knowledge-entry",
            &knowledge_entry,
            None,
            false
        ).await.map_err(|e| format!("Failed to store knowledge: {e}"))?;

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText("Knowledge entry created successfully".to_string()),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["knowledge".to_string()]
    }
}