use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use serde_json::Value;
use tracing::info;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::memories::memories_search;


pub struct ToolGetKnowledge;


#[async_trait]
impl Tool for ToolGetKnowledge {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        info!("run @get-knowledge {:?}", args);

        let (gcx, _top_n) = {
            let ccx_locked = ccx.lock().await;
            (ccx_locked.global_context.clone(), ccx_locked.top_n)
        };

        let search_key = match args.get("search_key") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => { return Err(format!("argument `search_key` is not a string: {:?}", v)) },
            None => { return Err("argument `search_key` is missing".to_string()) }
        };

        let mem_top_n = 5;
        let memories: crate::memories::MemoSearchResult = memories_search(gcx.clone(), &search_key, mem_top_n).await?;
        
        let mut seen_memids = HashSet::new();
        let unique_memories: Vec<_> = memories.results.into_iter()
            .filter(|m| seen_memids.insert(m.iknow_id.clone()))
            .collect();

        let memories_str = unique_memories.iter().map(|m| {
            let payload: String = m.iknow_memory.clone();
            let mut combined = String::new();
            combined.push_str(&format!("🗃️{}\n", m.iknow_id));
            combined.push_str(&payload);
            combined.push_str("\n\n");
            combined
        }).collect::<String>();

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(memories_str),
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