use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde_json::{json, Value};
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_diff::{execute_diff_for_vcs, get_last_accessed_file};
use crate::at_commands::at_file::{file_repair_candidates, get_project_paths, real_file_path_candidate};
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum};


pub struct AttDiff;

#[async_trait]
impl Tool for AttDiff {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<Vec<ContextEnum>, String> {
        let gcx = ccx.lock().await.global_context.clone();
        let diff_chunks = match args.len() {
            0 => {
                // No arguments: git diff for all tracked files
                let last_accessed_file = get_last_accessed_file(ccx.clone()).await?;
                let parent_dir = last_accessed_file.parent().ok_or(format!("Couldn't get parent directory of last accessed file: {:?}", last_accessed_file))?.to_string_lossy().to_string();
                execute_diff_for_vcs(&parent_dir, &[]).await.map_err(|e| format!("Couldn't execute git diff.\nError: {}", e))
            },
            1 => {
                // 1 argument: git diff for a specific file
                let file_path_arg = args.get("file_path").and_then(|v| v.as_str()).ok_or("Missing argument `file_path` in the diff() call.")?.to_string();
                let candidates = file_repair_candidates(gcx.clone(), &file_path_arg, 10, false).await;
                let file_path = real_file_path_candidate(gcx.clone(), &file_path_arg, &candidates, &get_project_paths(gcx.clone()).await, false).await?;
                let parent_dir = PathBuf::from(&file_path).parent().ok_or(format!("Couldn't get parent directory of file: {:?}", file_path))?.to_string_lossy().to_string();
                execute_diff_for_vcs(&parent_dir, &[&file_path]).await.map_err(|e| format!("Couldn't execute git diff {}.\nError: {}", file_path, e))
            },
            _ => {
                return Err("Invalid number of arguments".to_string());
            }
        }?;

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: json!(diff_chunks).to_string(),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok(results)
    }
}
