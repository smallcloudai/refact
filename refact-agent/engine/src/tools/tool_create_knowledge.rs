use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tracing::info;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};

pub struct ToolCreateKnowledge {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolCreateKnowledge {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "create_knowledge".to_string(),
            display_name: "Create Knowledge".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Creates a new knowledge entry in the vector database to help with future tasks.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "knowledge_entry".to_string(),
                    param_type: "string".to_string(),
                    description: "The detailed knowledge content to store. Include comprehensive information about implementation details, code patterns, architectural decisions, troubleshooting steps, or solution approaches. Document what you did, how you did it, why you made certain choices, and any important observations or lessons learned. This field should contain the rich, detailed content that future searches will retrieve.".to_string(),
                }
            ],
            parameters_required: vec![
                "knowledge_entry".to_string(),
            ],
        }
    }

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
