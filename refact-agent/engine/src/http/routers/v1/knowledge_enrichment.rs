use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use regex::Regex;

use crate::call_validation::{ChatContent, ChatMessage, ContextFile};
use crate::global_context::GlobalContext;
use crate::memories::memories_search;

const KNOWLEDGE_TOP_N: usize = 3;
const TRAJECTORY_TOP_N: usize = 2;
const KNOWLEDGE_SCORE_THRESHOLD: f32 = 0.75;
const KNOWLEDGE_ENRICHMENT_MARKER: &str = "knowledge_enrichment";
const MAX_QUERY_LENGTH: usize = 2000;

pub async fn enrich_messages_with_knowledge(
    gcx: Arc<ARwLock<GlobalContext>>,
    messages: &mut Vec<ChatMessage>,
) -> Option<Vec<serde_json::Value>> {
    let last_user_idx = messages.iter().rposition(|m| m.role == "user")?;
    let query_raw = messages[last_user_idx].content.content_text_only();

    if has_knowledge_enrichment_near(messages, last_user_idx) {
        return None;
    }

    let query_normalized = normalize_query(&query_raw);

    if !should_enrich(messages, &query_raw, &query_normalized) {
        return None;
    }

    let existing_paths = get_existing_context_file_paths(messages);

    if let Some((knowledge_context, ui_context)) = create_knowledge_context(gcx, &query_normalized, &existing_paths).await {
        messages.insert(last_user_idx, knowledge_context);
        tracing::info!("Injected knowledge context before user message at position {}", last_user_idx);
        return Some(vec![ui_context]);
    }

    None
}

fn normalize_query(query: &str) -> String {
    let code_fence_re = Regex::new(r"```[\s\S]*?```").unwrap();
    let normalized = code_fence_re.replace_all(query, " [code] ").to_string();
    let normalized = normalized.trim();
    if normalized.len() > MAX_QUERY_LENGTH {
        normalized.chars().take(MAX_QUERY_LENGTH).collect()
    } else {
        normalized.to_string()
    }
}

fn should_enrich(messages: &[ChatMessage], query_raw: &str, query_normalized: &str) -> bool {
    let trimmed = query_raw.trim();

    // Guardrail: empty query
    if trimmed.is_empty() {
        return false;
    }

    // Guardrail: command-like messages
    if trimmed.starts_with('@') || trimmed.starts_with('/') {
        return false;
    }

    // Rule 1: Always enrich first user message
    let user_message_count = messages.iter().filter(|m| m.role == "user").count();
    if user_message_count == 1 {
        tracing::info!("Knowledge enrichment: first user message");
        return true;
    }

    // Rule 2: Signal-based for subsequent messages
    let strong = count_strong_signals(query_raw);
    let weak = count_weak_signals(query_raw, query_normalized);

    if strong >= 1 {
        tracing::info!("Knowledge enrichment: {} strong signal(s)", strong);
        return true;
    }

    if weak >= 2 && query_normalized.len() >= 20 {
        tracing::info!("Knowledge enrichment: {} weak signal(s)", weak);
        return true;
    }

    false
}

fn count_strong_signals(query: &str) -> usize {
    let query_lower = query.to_lowercase();
    let mut count = 0;

    // Error/debug keywords
    let error_keywords = [
        "error", "panic", "exception", "traceback", "stack trace",
        "segfault", "failed", "unable to", "cannot", "doesn't work",
        "does not work", "broken", "bug", "crash"
    ];
    if error_keywords.iter().any(|kw| query_lower.contains(kw)) {
        count += 1;
    }

    // File references
    let file_extensions = [".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".java", ".cpp", ".c", ".h"];
    let config_files = ["cargo.toml", "package.json", "tsconfig", "pyproject", ".yaml", ".yml", ".toml"];
    if file_extensions.iter().any(|ext| query_lower.contains(ext))
        || config_files.iter().any(|f| query_lower.contains(f)) {
        count += 1;
    }

    // Path-like pattern
    let path_re = Regex::new(r"\b[\w-]+/[\w-]+(?:/[\w.-]+)*\b").unwrap();
    if path_re.is_match(query) {
        count += 1;
    }

    // Code symbols
    if query.contains("::") || query.contains("->") || query.contains("`") {
        count += 1;
    }

    // Explicit retrieval intent
    let retrieval_phrases = [
        "search", "find", "where is", "which file", "look up",
        "in this repo", "in the codebase", "in the project"
    ];
    if retrieval_phrases.iter().any(|p| query_lower.contains(p)) {
        count += 1;
    }

    count
}

fn count_weak_signals(query_raw: &str, query_normalized: &str) -> usize {
    let mut count = 0;

    // Has question mark
    if query_raw.contains('?') {
        count += 1;
    }

    // Starts with question word
    let query_lower = query_raw.trim().to_lowercase();
    let question_starters = ["how", "why", "what", "where", "when", "can", "should", "could", "would", "is there", "are there"];
    if question_starters.iter().any(|s| query_lower.starts_with(s)) {
        count += 1;
    }

    // Long enough natural language (after stripping code)
    if query_normalized.len() >= 80 {
        count += 1;
    }

    count
}

async fn create_knowledge_context(
    gcx: Arc<ARwLock<GlobalContext>>,
    query_text: &str,
    existing_paths: &HashSet<String>,
) -> Option<(ChatMessage, serde_json::Value)> {

    let memories = memories_search(gcx.clone(), &query_text, KNOWLEDGE_TOP_N, TRAJECTORY_TOP_N).await.ok()?;

    let high_score_memories: Vec<_> = memories
        .into_iter()
        .filter(|m| m.score.unwrap_or(0.0) >= KNOWLEDGE_SCORE_THRESHOLD)
        .filter(|m| {
            if let Some(path) = &m.file_path {
                !existing_paths.contains(&path.to_string_lossy().to_string())
            } else {
                true
            }
        })
        .collect();

    if high_score_memories.is_empty() {
        return None;
    }

    tracing::info!("Knowledge enrichment: {} memories passed threshold {}", high_score_memories.len(), KNOWLEDGE_SCORE_THRESHOLD);

    let context_files_for_llm: Vec<ContextFile> = high_score_memories
        .iter()
        .filter_map(|memo| {
            let file_path = memo.file_path.as_ref()?;
            let (line1, line2) = memo.line_range.unwrap_or((1, 50));
            Some(ContextFile {
                file_name: file_path.to_string_lossy().to_string(),
                file_content: String::new(),
                line1: line1 as usize,
                line2: line2 as usize,
                symbols: vec![],
                gradient_type: -1,
                usefulness: 80.0 + (memo.score.unwrap_or(0.75) * 20.0),
                skip_pp: false,
            })
        })
        .collect();

    if context_files_for_llm.is_empty() {
        return None;
    }

    let context_files_for_ui: Vec<serde_json::Value> = high_score_memories
        .iter()
        .filter_map(|memo| {
            let file_path = memo.file_path.as_ref()?;
            let (line1, line2) = memo.line_range.unwrap_or((1, 50));
            Some(serde_json::json!({
                "file_name": file_path.to_string_lossy().to_string(),
                "file_content": memo.content.clone(),
                "line1": line1,
                "line2": line2,
            }))
        })
        .collect();

    let content = serde_json::to_string(&context_files_for_llm).ok()?;
    let chat_message = ChatMessage {
        role: "context_file".to_string(),
        content: ChatContent::SimpleText(content),
        tool_call_id: KNOWLEDGE_ENRICHMENT_MARKER.to_string(),
        ..Default::default()
    };

    let ui_content_str = serde_json::to_string(&context_files_for_ui).unwrap_or_default();
    let ui_message = serde_json::json!({
        "role": "context_file",
        "content": ui_content_str,
        "tool_call_id": KNOWLEDGE_ENRICHMENT_MARKER,
    });

    Some((chat_message, ui_message))
}

fn has_knowledge_enrichment_near(messages: &[ChatMessage], user_idx: usize) -> bool {
    let search_start = user_idx.saturating_sub(2);
    let search_end = (user_idx + 2).min(messages.len());

    for i in search_start..search_end {
        if messages[i].role == "context_file" && messages[i].tool_call_id == KNOWLEDGE_ENRICHMENT_MARKER {
            tracing::info!("Skipping enrichment - already enriched at position {}", i);
            return true;
        }
    }
    false
}

fn get_existing_context_file_paths(messages: &[ChatMessage]) -> HashSet<String> {
    let mut paths = HashSet::new();
    for msg in messages {
        if msg.role == "context_file" {
            let content = msg.content.content_text_only();
            if let Ok(files) = serde_json::from_str::<Vec<ContextFile>>(&content) {
                for file in files {
                    paths.insert(file.file_name.clone());
                }
            }
        }
    }
    paths
}
