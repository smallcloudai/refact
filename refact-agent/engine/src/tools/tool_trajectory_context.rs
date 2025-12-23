use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use tokio::fs;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::files_correction::get_project_dirs;

pub struct ToolTrajectoryContext {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolTrajectoryContext {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "get_trajectory_context".to_string(),
            display_name: "Get Trajectory Context".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: false,
            experimental: false,
            description: "Get more context from a specific trajectory around given message indices.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "trajectory_id".to_string(),
                    param_type: "string".to_string(),
                    description: "The trajectory ID to retrieve context from.".to_string(),
                },
                ToolParam {
                    name: "message_start".to_string(),
                    param_type: "string".to_string(),
                    description: "Starting message index.".to_string(),
                },
                ToolParam {
                    name: "message_end".to_string(),
                    param_type: "string".to_string(),
                    description: "Ending message index.".to_string(),
                },
                ToolParam {
                    name: "expand_by".to_string(),
                    param_type: "string".to_string(),
                    description: "Number of messages to include before/after (default: 3).".to_string(),
                },
            ],
            parameters_required: vec!["trajectory_id".to_string(), "message_start".to_string(), "message_end".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let trajectory_id = match args.get("trajectory_id") {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("Missing argument `trajectory_id`".to_string())
        };

        let msg_start: usize = match args.get("message_start") {
            Some(Value::String(s)) => s.parse().map_err(|_| "Invalid message_start")?,
            Some(Value::Number(n)) => n.as_u64().ok_or("Invalid message_start")? as usize,
            _ => return Err("Missing argument `message_start`".to_string())
        };

        let msg_end: usize = match args.get("message_end") {
            Some(Value::String(s)) => s.parse().map_err(|_| "Invalid message_end")?,
            Some(Value::Number(n)) => n.as_u64().ok_or("Invalid message_end")? as usize,
            _ => return Err("Missing argument `message_end`".to_string())
        };

        let expand_by: usize = match args.get("expand_by") {
            Some(Value::String(s)) => s.parse().unwrap_or(3),
            Some(Value::Number(n)) => n.as_u64().unwrap_or(3) as usize,
            _ => 3,
        };

        let gcx = ccx.lock().await.global_context.clone();
        let project_dirs = get_project_dirs(gcx.clone()).await;
        let workspace_root = project_dirs.first().ok_or("No workspace folder")?;
        let traj_path = workspace_root.join(".refact/trajectories").join(format!("{}.json", trajectory_id));

        if !traj_path.exists() {
            return Err(format!("Trajectory not found: {}", trajectory_id));
        }

        let content = fs::read_to_string(&traj_path).await
            .map_err(|e| format!("Failed to read trajectory: {}", e))?;

        let trajectory: Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse trajectory: {}", e))?;

        let messages = trajectory.get("messages")
            .and_then(|v| v.as_array())
            .ok_or("No messages in trajectory")?;

        let title = trajectory.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
        let actual_start = msg_start.saturating_sub(expand_by);
        let actual_end = (msg_end + expand_by).min(messages.len().saturating_sub(1));

        let mut output = format!("Trajectory: {} ({})\nMessages {}-{} (expanded from {}-{}):\n\n",
            trajectory_id, title, actual_start, actual_end, msg_start, msg_end);

        for (i, msg) in messages.iter().enumerate() {
            if i < actual_start || i > actual_end {
                continue;
            }

            let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
            if role == "context_file" || role == "cd_instruction" {
                continue;
            }

            let content_text = extract_content(msg);
            if content_text.trim().is_empty() {
                continue;
            }

            let marker = if i >= msg_start && i <= msg_end { ">>>" } else { "   " };
            output.push_str(&format!("{} [{}] {}:\n{}\n\n", marker, i, role.to_uppercase(), content_text));
        }

        Ok((false, vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(output),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })]))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}

fn extract_content(msg: &Value) -> String {
    if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
        return content.to_string();
    }

    if let Some(content_arr) = msg.get("content").and_then(|c| c.as_array()) {
        return content_arr.iter()
            .filter_map(|item| {
                item.get("text").and_then(|t| t.as_str())
                    .or_else(|| item.get("m_content").and_then(|t| t.as_str()))
            })
            .collect::<Vec<_>>()
            .join("\n");
    }

    if let Some(tool_calls) = msg.get("tool_calls").and_then(|tc| tc.as_array()) {
        return tool_calls.iter()
            .filter_map(|tc| tc.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()))
            .map(|s| format!("[tool: {}]", s))
            .collect::<Vec<_>>()
            .join(" ");
    }

    String::new()
}
