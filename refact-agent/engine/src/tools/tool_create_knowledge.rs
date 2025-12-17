use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;
use tracing::{info, warn};
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::knowledge_graph::kg_subchat::{enrich_knowledge_metadata, check_deprecation};
use crate::knowledge_graph::kg_structs::KnowledgeFrontmatter;
use crate::memories::{memories_add, deprecate_document};
use crate::global_context::try_load_caps_quickly_if_not_present;

pub struct ToolCreateKnowledge {
    pub config_path: String,
}

fn extract_entities(content: &str) -> Vec<String> {
    let backtick_re = Regex::new(r"`([a-zA-Z_][a-zA-Z0-9_:]*(?:::[a-zA-Z_][a-zA-Z0-9_]*)*)`").unwrap();
    backtick_re.captures_iter(content)
        .map(|c| c.get(1).unwrap().as_str().to_string())
        .filter(|e| e.len() >= 3 && e.len() <= 100)
        .collect()
}

fn extract_file_paths(content: &str) -> Vec<String> {
    let path_re = Regex::new(r"(?:^|[\s`])((?:[a-zA-Z0-9_-]+/)+[a-zA-Z0-9_-]+\.[a-zA-Z0-9]+)").unwrap();
    path_re.captures_iter(content)
        .map(|c| c.get(1).unwrap().as_str().to_string())
        .collect()
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
            description: "Creates a new knowledge entry. Uses AI to enrich metadata and check for outdated documents.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "content".to_string(),
                    param_type: "string".to_string(),
                    description: "The knowledge content to store.".to_string(),
                },
                ToolParam {
                    name: "tags".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated tags (optional, will be auto-enriched).".to_string(),
                },
                ToolParam {
                    name: "filenames".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated related file paths (optional, will be auto-enriched).".to_string(),
                },
            ],
            parameters_required: vec!["content".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        info!("create_knowledge {:?}", args);

        let gcx = ccx.lock().await.global_context.clone();

        let content = match args.get("content") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `content` is not a string: {:?}", v)),
            None => return Err("argument `content` is missing".to_string()),
        };

        let user_tags: Vec<String> = match args.get("tags") {
            Some(Value::String(s)) => s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect(),
            _ => vec![],
        };

        let user_filenames: Vec<String> = match args.get("filenames") {
            Some(Value::String(s)) => s.split(',').map(|f| f.trim().to_string()).filter(|f| !f.is_empty()).collect(),
            _ => vec![],
        };

        let entities = extract_entities(&content);
        let detected_paths = extract_file_paths(&content);

        let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await
            .map_err(|e| format!("Failed to load caps: {}", e.message))?;
        let light_model = if caps.defaults.chat_light_model.is_empty() {
            caps.defaults.chat_default_model.clone()
        } else {
            caps.defaults.chat_light_model.clone()
        };

        let kg = crate::knowledge_graph::build_knowledge_graph(gcx.clone()).await;

        let candidate_files: Vec<String> = {
            let mut files = user_filenames.clone();
            files.extend(detected_paths);
            files.into_iter().take(30).collect()
        };

        let candidate_docs: Vec<(String, String)> = kg.active_docs()
            .take(20)
            .map(|d| {
                let id = d.frontmatter.id.clone().unwrap_or_else(|| d.path.to_string_lossy().to_string());
                let title = d.frontmatter.title.clone().unwrap_or_else(|| "Untitled".to_string());
                (id, title)
            })
            .collect();

        let enrichment = enrich_knowledge_metadata(
            ccx.clone(),
            &light_model,
            &content,
            &entities,
            &candidate_files,
            &candidate_docs,
        ).await;

        let (final_title, final_tags, final_filenames, final_kind, final_links, review_days) = match enrichment {
            Ok(e) => {
                let mut tags = user_tags.clone();
                tags.extend(e.tags);
                tags.sort();
                tags.dedup();

                let mut files = user_filenames.clone();
                files.extend(e.filenames);
                files.sort();
                files.dedup();

                let kind = e.kind.unwrap_or_else(|| if files.is_empty() { "domain".to_string() } else { "code".to_string() });

                (
                    e.title.or_else(|| content.lines().next().map(|l| l.trim_start_matches('#').trim().to_string())),
                    if tags.is_empty() { vec!["knowledge".to_string()] } else { tags },
                    files,
                    kind,
                    e.links,
                    e.review_after_days.unwrap_or(90),
                )
            }
            Err(e) => {
                warn!("Enrichment failed, using defaults: {}", e);
                let tags = if user_tags.is_empty() { vec!["knowledge".to_string()] } else { user_tags };
                let kind = if user_filenames.is_empty() { "domain".to_string() } else { "code".to_string() };
                (
                    content.lines().next().map(|l| l.trim_start_matches('#').trim().to_string()),
                    tags,
                    user_filenames,
                    kind,
                    vec![],
                    90,
                )
            }
        };

        let now = chrono::Local::now();
        let frontmatter = KnowledgeFrontmatter {
            id: Some(uuid::Uuid::new_v4().to_string()),
            title: final_title.clone(),
            tags: final_tags.clone(),
            created: Some(now.format("%Y-%m-%d").to_string()),
            updated: Some(now.format("%Y-%m-%d").to_string()),
            filenames: final_filenames.clone(),
            links: final_links,
            kind: Some(final_kind.clone()),
            status: Some("active".to_string()),
            superseded_by: None,
            deprecated_at: None,
            review_after: Some((now + chrono::Duration::days(review_days)).format("%Y-%m-%d").to_string()),
        };

        let file_path = memories_add(gcx.clone(), &frontmatter, &content).await?;
        let new_doc_id = frontmatter.id.clone().unwrap();

        let deprecation_candidates = kg.get_deprecation_candidates(
            &final_tags,
            &final_filenames,
            &entities,
            Some(&new_doc_id),
        );

        let mut deprecated_count = 0;
        if !deprecation_candidates.is_empty() {
            let snippet: String = content.chars().take(500).collect();

            match check_deprecation(
                ccx.clone(),
                &light_model,
                final_title.as_deref().unwrap_or("Untitled"),
                &final_tags,
                &final_filenames,
                &snippet,
                &deprecation_candidates,
            ).await {
                Ok(result) => {
                    for decision in result.deprecate {
                        if decision.confidence >= 0.75 {
                            if let Some(doc) = kg.get_doc_by_id(&decision.target_id) {
                                if let Err(e) = deprecate_document(
                                    gcx.clone(),
                                    &doc.path,
                                    Some(&new_doc_id),
                                    &decision.reason,
                                ).await {
                                    warn!("Failed to deprecate {}: {}", decision.target_id, e);
                                } else {
                                    deprecated_count += 1;
                                    info!("Deprecated {} (confidence: {:.2}): {}", decision.target_id, decision.confidence, decision.reason);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Deprecation check failed: {}", e);
                }
            }
        }

        let mut result_msg = format!("Knowledge entry created: {}", file_path.display());
        if deprecated_count > 0 {
            result_msg.push_str(&format!("\n{} outdated document(s) marked as deprecated.", deprecated_count));
        }

        Ok((false, vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(result_msg),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })]))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["knowledge".to_string()]
    }
}
