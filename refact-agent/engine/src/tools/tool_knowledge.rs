use std::sync::Arc;
use std::collections::HashSet;
use serde_json::Value;
use tracing::info;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;
use std::collections::HashMap;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::memories::memories_search;
use crate::knowledge_graph::build_knowledge_graph;

pub struct ToolGetKnowledge {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolGetKnowledge {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "knowledge".to_string(),
            display_name: "Knowledge".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Searches project knowledge base for relevant information. Uses semantic search and knowledge graph expansion. Also searches past chat trajectories for relevant patterns and solutions.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "search_key".to_string(),
                    param_type: "string".to_string(),
                    description: "Search query for the knowledge database.".to_string(),
                }
            ],
            parameters_required: vec!["search_key".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        info!("knowledge search {:?}", args);

        let gcx = ccx.lock().await.global_context.clone();

        let search_key = match args.get("search_key") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `search_key` is not a string: {:?}", v)),
            None => return Err("argument `search_key` is missing".to_string()),
        };

        let memories = memories_search(gcx.clone(), &search_key, 5, 0).await?;

        let mut seen_memids = HashSet::new();
        let mut unique_memories: Vec<_> = memories.into_iter()
            .filter(|m| seen_memids.insert(m.memid.clone()))
            .collect();

        if !unique_memories.is_empty() {
            let kg = build_knowledge_graph(gcx.clone()).await;

            let initial_ids: Vec<String> = unique_memories.iter()
                .filter_map(|m| m.file_path.as_ref())
                .filter_map(|p| kg.get_doc_by_path(p))
                .filter_map(|d| d.frontmatter.id.clone())
                .collect();

            let expanded_ids = kg.expand_search_results(&initial_ids, 3);

            for id in expanded_ids {
                if let Some(doc) = kg.get_doc_by_id(&id) {
                    if doc.frontmatter.is_active() && !seen_memids.contains(&id) {
                        seen_memids.insert(id.clone());
                        let snippet: String = doc.content.chars().take(500).collect();
                        unique_memories.push(crate::memories::MemoRecord {
                            memid: id,
                            tags: doc.frontmatter.tags.clone(),
                            content: snippet,
                            file_path: Some(doc.path.clone()),
                            line_range: None,
                            title: doc.frontmatter.title.clone(),
                            created: doc.frontmatter.created.clone(),
                            kind: doc.frontmatter.kind.clone(),
                            score: None,  // KG expansion doesn't have scores
                        });
                    }
                }
            }
        }

        unique_memories.sort_by(|a, b| {
            let a_is_traj = a.kind.as_deref() == Some("trajectory");
            let b_is_traj = b.kind.as_deref() == Some("trajectory");
            a_is_traj.cmp(&b_is_traj)
        });

        let memories_str = if unique_memories.is_empty() {
            "No relevant knowledge found.".to_string()
        } else {
            unique_memories.iter().take(8).map(|m| {
                let mut result = String::new();
                if let Some(path) = &m.file_path {
                    result.push_str(&format!("📄 {}", path.display()));
                    if let Some((start, end)) = m.line_range {
                        result.push_str(&format!(":{}-{}", start, end));
                    }
                    result.push('\n');
                }
                if let Some(title) = &m.title {
                    result.push_str(&format!("📌 {}\n", title));
                }
                if let Some(kind) = &m.kind {
                    result.push_str(&format!("📦 {}\n", kind));
                }
                if !m.tags.is_empty() {
                    result.push_str(&format!("🏷️ {}\n", m.tags.join(", ")));
                }
                result.push_str(&m.content);
                result.push_str("\n\n---\n");
                result
            }).collect()
        };

        Ok((false, vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(memories_str),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })]))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["knowledge".to_string()]
    }
}
