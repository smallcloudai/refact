use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use serde_json::Value;
use tracing::info;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::vecdb::vdb_highlev::memories_search;


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

        let im_going_to_use_tools = match args.get("im_going_to_use_tools") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => { return Err(format!("argument `im_going_to_use_tools` is not a string: {:?}", v)) },
            None => { return Err("argument `im_going_to_use_tools` is missing".to_string()) }
        };
        let im_going_to_apply_to = match args.get("im_going_to_apply_to") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => { return Err(format!("argument `im_going_to_apply_to` is not a string: {:?}", v)) },
            None => { return Err("argument `im_going_to_apply_to` is missing".to_string()) }
        };
        let goal = match args.get("goal") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => { return Err(format!("argument `goal` is not a string: {:?}", v)) },
            None => { return Err("argument `goal` is missing".to_string()) }
        };
        let language_slash_framework = match args.get("language_slash_framework") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => { return Err(format!("argument `language_slash_framework` is not a string: {:?}", v)) },
            None => { return Err("argument `language_slash_framework` is missing".to_string()) }
        };

        let mem_top_n = 3;
        let memories1: crate::vecdb::vdb_structs::MemoSearchResult = memories_search(gcx.clone(), &im_going_to_use_tools, mem_top_n).await?;
        let memories2: crate::vecdb::vdb_structs::MemoSearchResult = memories_search(gcx.clone(), &im_going_to_apply_to, mem_top_n).await?;
        let memories3: crate::vecdb::vdb_structs::MemoSearchResult = memories_search(gcx.clone(), &goal, mem_top_n).await?;
        let memories4: crate::vecdb::vdb_structs::MemoSearchResult = memories_search(gcx.clone(), &language_slash_framework, mem_top_n).await?;
        let combined_memories = [memories1.results, memories2.results, memories3.results, memories4.results].concat();
        let mut seen_memids = HashSet::new();
        let unique_memories: Vec<_> = combined_memories.into_iter()
            .filter(|m| seen_memids.insert(m.memid.clone()))
            .collect();

        let memories_str = unique_memories.iter().map(|m| {
            let payload: String = m.m_payload.clone();
            let mut combined = String::new();
            combined.push_str(&format!("🗃️{}\n", m.memid));
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
        vec!["vecdb".to_string()]
    }
}
