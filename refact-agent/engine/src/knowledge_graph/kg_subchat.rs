use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::ChatMessage;
use crate::subchat::subchat_single;

use super::kg_structs::KnowledgeDoc;

#[derive(Debug, Serialize, Deserialize)]
pub struct EnrichmentResult {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub filenames: Vec<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(default)]
    pub review_after_days: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeprecationDecision {
    #[serde(default)]
    pub target_id: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub confidence: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeprecationResult {
    #[serde(default)]
    pub deprecate: Vec<DeprecationDecision>,
    #[serde(default)]
    pub keep: Vec<String>,
}

const ENRICHMENT_PROMPT: &str = r#"Analyze the following knowledge content and extract metadata.

CONTENT:
{content}

EXTRACTED ENTITIES (backticked identifiers found):
{entities}

CANDIDATE FILES (from workspace, pick only relevant ones):
{candidate_files}

CANDIDATE RELATED DOCS (from knowledge base, pick only relevant ones):
{candidate_docs}

Return a JSON object with:
- title: a concise title for this knowledge (max 80 chars)
- tags: array of relevant tags (lowercase, max 8 tags)
- filenames: array of file paths this knowledge relates to (only from candidates)
- kind: one of "code", "decision", "domain", "process" (based on content)
- links: array of related doc IDs (only from candidates)
- review_after_days: suggested review period (90 for code/decision, 180 for domain)

JSON only, no explanation:"#;

const DEPRECATION_PROMPT: &str = r#"A new knowledge document was created. Determine if any existing documents should be deprecated.

NEW DOCUMENT:
Title: {new_title}
Tags: {new_tags}
Files: {new_files}
Content snippet: {new_snippet}

CANDIDATE DOCUMENTS TO POTENTIALLY DEPRECATE:
{candidates}

For each candidate, decide if it should be deprecated because:
- It covers the same topic and the new doc is more complete/updated
- It references the same files with outdated information
- It's a duplicate or near-duplicate

Return JSON:
{{
  "deprecate": [
    {{"target_id": "...", "reason": "...", "confidence": 0.0-1.0}}
  ],
  "keep": ["id1", "id2"]
}}

Only deprecate with confidence >= 0.75. JSON only:"#;

pub async fn enrich_knowledge_metadata(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model_id: &str,
    content: &str,
    entities: &[String],
    candidate_files: &[String],
    candidate_docs: &[(String, String)],
) -> Result<EnrichmentResult, String> {
    let entities_str = entities.join(", ");
    let files_str = candidate_files.iter().take(20).cloned().collect::<Vec<_>>().join("\n");
    let docs_str = candidate_docs.iter()
        .take(10)
        .map(|(id, title)| format!("- {}: {}", id, title))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = ENRICHMENT_PROMPT
        .replace("{content}", &content.chars().take(2000).collect::<String>())
        .replace("{entities}", &entities_str)
        .replace("{candidate_files}", &files_str)
        .replace("{candidate_docs}", &docs_str);

    let messages = vec![ChatMessage::new("user".to_string(), prompt)];

    let results = subchat_single(
        ccx,
        model_id,
        messages,
        Some(vec![]),
        Some("none".to_string()),
        false,
        Some(0.0),
        Some(1024),
        1,
        None,
        false,
        None,
        None,
        None,
    ).await?;

    let response = results.get(0)
        .and_then(|msgs| msgs.last())
        .map(|m| m.content.content_text_only())
        .unwrap_or_default();

    let json_start = response.find('{').unwrap_or(0);
    let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
    let json_str = &response[json_start..json_end];

    serde_json::from_str(json_str).map_err(|e| format!("Failed to parse enrichment JSON: {}", e))
}

pub async fn check_deprecation(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model_id: &str,
    new_doc_title: &str,
    new_doc_tags: &[String],
    new_doc_files: &[String],
    new_doc_snippet: &str,
    candidates: &[&KnowledgeDoc],
) -> Result<DeprecationResult, String> {
    if candidates.is_empty() {
        return Ok(DeprecationResult { deprecate: vec![], keep: vec![] });
    }

    let candidates_str = candidates.iter()
        .map(|doc| {
            let id = doc.frontmatter.id.clone().unwrap_or_else(|| doc.path.to_string_lossy().to_string());
            let title = doc.frontmatter.title.clone().unwrap_or_default();
            let tags = doc.frontmatter.tags.join(", ");
            let files = doc.frontmatter.filenames.join(", ");
            let snippet: String = doc.content.chars().take(300).collect();
            format!("ID: {}\nTitle: {}\nTags: {}\nFiles: {}\nSnippet: {}\n---", id, title, tags, files, snippet)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = DEPRECATION_PROMPT
        .replace("{new_title}", new_doc_title)
        .replace("{new_tags}", &new_doc_tags.join(", "))
        .replace("{new_files}", &new_doc_files.join(", "))
        .replace("{new_snippet}", &new_doc_snippet.chars().take(500).collect::<String>())
        .replace("{candidates}", &candidates_str);

    let messages = vec![ChatMessage::new("user".to_string(), prompt)];

    let results = subchat_single(
        ccx,
        model_id,
        messages,
        Some(vec![]),
        Some("none".to_string()),
        false,
        Some(0.0),
        Some(1024),
        1,
        None,
        false,
        None,
        None,
        None,
    ).await?;

    let response = results.get(0)
        .and_then(|msgs| msgs.last())
        .map(|m| m.content.content_text_only())
        .unwrap_or_default();

    let json_start = response.find('{').unwrap_or(0);
    let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
    let json_str = &response[json_start..json_end];

    serde_json::from_str(json_str).map_err(|e| format!("Failed to parse deprecation JSON: {}", e))
}
