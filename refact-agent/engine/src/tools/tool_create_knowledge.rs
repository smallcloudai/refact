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

        let im_going_to_use_tools = match args.get("im_going_to_use_tools") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `im_going_to_use_tools` is not a string: {:?}", v)),
            None => return Err("argument `im_going_to_use_tools` is missing".to_string())
        };

        let im_going_to_apply_to = match args.get("im_going_to_apply_to") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `im_going_to_apply_to` is not a string: {:?}", v)),
            None => return Err("argument `im_going_to_apply_to` is missing".to_string())
        };

        let search_key = match args.get("search_key") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `search_key` is not a string: {:?}", v)),
            None => return Err("argument `search_key` is missing".to_string())
        };

        let language_slash_framework = match args.get("language_slash_framework") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `language_slash_framework` is not a string: {:?}", v)),
            None => return Err("argument `language_slash_framework` is missing".to_string())
        };

        let knowledge_entry = match args.get("knowledge_entry") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `knowledge_entry` is not a string: {:?}", v)),
            None => return Err("argument `knowledge_entry` is missing".to_string())
        };

        let vec_db = gcx.read().await.vec_db.clone();
        
        // Store the memory with type "knowledge-entry"
        let memid = match crate::vecdb::vdb_highlev::memories_add(
            vec_db.clone(),
            "knowledge-entry",
            &search_key,
            &im_going_to_apply_to,
            &knowledge_entry,
            "user-created"
        ).await {
            Ok(id) => id,
            Err(e) => return Err(format!("Failed to store knowledge: {}", e))
        };

        let message = format!("Knowledge entry created successfully with ID: {}\nTools: {}\nApply to: {}\nSearch Key: {}\nLanguage/Framework: {}\nEntry: {}", 
            memid,
            im_going_to_use_tools,
            im_going_to_apply_to,
            search_key,
            language_slash_framework,
            knowledge_entry
        );

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(message),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}