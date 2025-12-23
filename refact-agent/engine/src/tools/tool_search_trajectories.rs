use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};

pub struct ToolSearchTrajectories {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolSearchTrajectories {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "search_trajectories".to_string(),
            display_name: "Search Trajectories".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: false,
            experimental: false,
            description: "Search through past chat trajectories for relevant context, patterns, or solutions. Returns trajectory ID and message range for further exploration.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "query".to_string(),
                    param_type: "string".to_string(),
                    description: "Search query to find relevant trajectory content.".to_string(),
                },
                ToolParam {
                    name: "top_n".to_string(),
                    param_type: "string".to_string(),
                    description: "Number of results to return (default: 5).".to_string(),
                },
            ],
            parameters_required: vec!["query".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let query = match args.get("query") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `query` is not a string: {:?}", v)),
            None => return Err("Missing argument `query`".to_string())
        };

        let top_n: usize = match args.get("top_n") {
            Some(Value::String(s)) => s.parse().unwrap_or(5),
            Some(Value::Number(n)) => n.as_u64().unwrap_or(5) as usize,
            _ => 5,
        };

        let gcx = ccx.lock().await.global_context.clone();

        let results = {
            let vecdb_lock = gcx.read().await.vec_db.clone();
            let vecdb_guard = vecdb_lock.lock().await;
            let vecdb = vecdb_guard.as_ref().ok_or("VecDB not available")?;

            use crate::vecdb::vdb_structs::VecdbSearch;
            vecdb.vecdb_search(query.clone(), top_n * 3, None).await
                .map_err(|e| format!("Search failed: {}", e))?
        };

        let trajectory_results: Vec<_> = results.results.iter()
            .filter(|r| r.file_path.to_string_lossy().contains(".refact/trajectories/"))
            .take(top_n)
            .collect();

        if trajectory_results.is_empty() {
            return Ok((false, vec![ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText("No trajectory results found for this query.".to_string()),
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                ..Default::default()
            })]));
        }

        let mut output = format!("Found {} trajectory segments for query: \"{}\"\n\n", trajectory_results.len(), query);

        for (i, rec) in trajectory_results.iter().enumerate() {
            let path_str = rec.file_path.to_string_lossy();
            let traj_id = path_str
                .rsplit('/')
                .next()
                .unwrap_or("")
                .trim_end_matches(".json");

            output.push_str(&format!(
                "{}. trajectory_id: {}\n   messages: {}-{}\n   relevance: {:.1}%\n\n",
                i + 1,
                traj_id,
                rec.start_line,
                rec.end_line,
                rec.usefulness
            ));
        }

        output.push_str("\nUse get_trajectory_context(trajectory_id, message_start, message_end) to retrieve full content.");

        Ok((false, vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(output),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })]))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}
