use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use serde_json::Value;

use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;
use crate::vecdb::vdb_structs::SplitResult;
use crate::ast::chunk_utils::official_text_hashing_function;

const MESSAGES_PER_CHUNK: usize = 4;
const MAX_CONTENT_PER_MESSAGE: usize = 2000;
const OVERLAP_MESSAGES: usize = 1;

pub struct TrajectoryFileSplitter {
    max_tokens: usize,
}

#[derive(Debug, Clone)]
struct ExtractedMessage {
    index: usize,
    role: String,
    content: String,
}

struct MessageChunk {
    text: String,
    start_msg: usize,
    end_msg: usize,
}

impl TrajectoryFileSplitter {
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }

    pub async fn split(
        &self,
        doc: &Document,
        gcx: Arc<ARwLock<GlobalContext>>,
    ) -> Result<Vec<SplitResult>, String> {
        let text = doc.clone().get_text_or_read_from_disk(gcx).await.map_err(|e| e.to_string())?;
        let path = doc.doc_path.clone();

        let trajectory: Value = serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse trajectory JSON: {}", e))?;

        let trajectory_id = trajectory.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
        let title = trajectory.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();
        let messages = trajectory.get("messages").and_then(|v| v.as_array()).ok_or("No messages array")?;

        let extracted = self.extract_messages(messages);
        if extracted.is_empty() {
            return Ok(vec![]);
        }

        let mut results = Vec::new();

        let metadata_text = format!("Trajectory: {}\nTitle: {}\nMessages: {}", trajectory_id, title, extracted.len());
        results.push(SplitResult {
            file_path: path.clone(),
            window_text: metadata_text.clone(),
            window_text_hash: official_text_hashing_function(&metadata_text),
            start_line: 0,
            end_line: 0,
            symbol_path: format!("traj:{}:meta", trajectory_id),
        });

        for chunk in self.chunk_messages(&extracted) {
            results.push(SplitResult {
                file_path: path.clone(),
                window_text: chunk.text.clone(),
                window_text_hash: official_text_hashing_function(&chunk.text),
                start_line: chunk.start_msg as u64,
                end_line: chunk.end_msg as u64,
                symbol_path: format!("traj:{}:msg:{}-{}", trajectory_id, chunk.start_msg, chunk.end_msg),
            });
        }

        Ok(results)
    }

    fn extract_messages(&self, messages: &[Value]) -> Vec<ExtractedMessage> {
        messages.iter().enumerate()
            .filter_map(|(idx, msg)| {
                let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                if role == "context_file" || role == "cd_instruction" {
                    return None;
                }

                let content = self.extract_content(msg);
                if content.trim().is_empty() {
                    return None;
                }

                let truncated = if content.len() > MAX_CONTENT_PER_MESSAGE {
                    format!("{}...", &content[..MAX_CONTENT_PER_MESSAGE])
                } else {
                    content
                };

                Some(ExtractedMessage { index: idx, role, content: truncated })
            })
            .collect()
    }

    fn extract_content(&self, msg: &Value) -> String {
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
            let names: Vec<_> = tool_calls.iter()
                .filter_map(|tc| tc.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()))
                .map(|s| format!("[tool: {}]", s))
                .collect();
            if !names.is_empty() {
                return names.join(" ");
            }
        }

        String::new()
    }

    fn chunk_messages(&self, messages: &[ExtractedMessage]) -> Vec<MessageChunk> {
        if messages.is_empty() {
            return vec![];
        }

        let mut chunks = Vec::new();
        let mut i = 0;

        while i < messages.len() {
            let end_idx = (i + MESSAGES_PER_CHUNK).min(messages.len());
            let chunk_messages = &messages[i..end_idx];
            let text = self.format_chunk(chunk_messages);

            let estimated_tokens = text.len() / 4;
            if estimated_tokens > self.max_tokens && chunk_messages.len() > 1 {
                for msg in chunk_messages {
                    chunks.push(MessageChunk {
                        text: self.format_chunk(&[msg.clone()]),
                        start_msg: msg.index,
                        end_msg: msg.index,
                    });
                }
            } else {
                chunks.push(MessageChunk {
                    text,
                    start_msg: chunk_messages.first().map(|m| m.index).unwrap_or(0),
                    end_msg: chunk_messages.last().map(|m| m.index).unwrap_or(0),
                });
            }

            i += MESSAGES_PER_CHUNK.saturating_sub(OVERLAP_MESSAGES).max(1);
        }

        chunks
    }

    fn format_chunk(&self, messages: &[ExtractedMessage]) -> String {
        messages.iter()
            .flat_map(|msg| {
                let role = match msg.role.as_str() {
                    "user" => "USER",
                    "assistant" => "ASSISTANT",
                    "tool" => "TOOL_RESULT",
                    "system" => "SYSTEM",
                    _ => &msg.role,
                };
                vec![format!("[{}]:", role), msg.content.clone(), String::new()]
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub fn is_trajectory_file(path: &PathBuf) -> bool {
    path.to_string_lossy().contains(".refact/trajectories/")
        && path.extension().map(|e| e == "json").unwrap_or(false)
}
