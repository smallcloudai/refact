use std::path::PathBuf;
use std::sync::Arc;
use tokenizers::Tokenizer;
use tokio::sync::RwLock as ARwLock;
use tracing::warn;

use crate::call_validation::{ChatContent, ChatMessage, ContextFile, PostprocessSettings};
use crate::files_correction::canonical_path;
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::global_context::GlobalContext;
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::postprocessing::pp_plain_text::postprocess_plain_text;
use crate::tokens::count_text_tokens_with_fallback;

const MIN_CONTEXT_SIZE: usize = 8192;

#[derive(Debug)]
pub struct ToolBudget {
    pub tokens_for_code: usize,
    pub tokens_for_text: usize,
}

impl ToolBudget {
    pub fn try_from_n_ctx(n_ctx: usize) -> Result<Self, String> {
        if n_ctx < MIN_CONTEXT_SIZE {
            return Err(format!("Model context size {} is below minimum {} tokens", n_ctx, MIN_CONTEXT_SIZE));
        }
        let total = (n_ctx / 2).max(4096);
        Ok(Self {
            tokens_for_code: total * 80 / 100,
            tokens_for_text: total * 20 / 100,
        })
    }
}

pub async fn postprocess_tool_results(
    gcx: Arc<ARwLock<GlobalContext>>,
    tokenizer: Option<Arc<Tokenizer>>,
    tool_messages: Vec<ChatMessage>,
    context_files: Vec<ContextFile>,
    budget: ToolBudget,
    pp_settings: PostprocessSettings,
    existing_messages: &[ChatMessage],
) -> Vec<ChatMessage> {
    let mut result = Vec::new();

    let (diff_messages, other_messages): (Vec<_>, Vec<_>) = tool_messages
        .into_iter()
        .partition(|m| m.role == "diff");

    result.extend(diff_messages);

    let (text_messages, _) = postprocess_plain_text(
        other_messages,
        tokenizer.clone(),
        budget.tokens_for_text,
        &None,
    ).await;
    result.extend(text_messages);

    if !context_files.is_empty() {
        let file_message = postprocess_context_file_results(
            gcx,
            tokenizer,
            context_files,
            budget.tokens_for_code,
            pp_settings,
            existing_messages,
        ).await;
        if let Some(msg) = file_message {
            result.push(msg);
        }
    }

    result
}

async fn postprocess_context_file_results(
    gcx: Arc<ARwLock<GlobalContext>>,
    tokenizer: Option<Arc<Tokenizer>>,
    context_files: Vec<ContextFile>,
    tokens_limit: usize,
    mut pp_settings: PostprocessSettings,
    existing_messages: &[ChatMessage],
) -> Option<ChatMessage> {
    let (skip_pp_files, mut pp_files): (Vec<_>, Vec<_>) = context_files
        .into_iter()
        .partition(|cf| cf.skip_pp);

    pp_settings.close_small_gaps = true;
    if pp_settings.max_files_n == 0 {
        pp_settings.max_files_n = 25;
    }

    let total_files = pp_files.len() + skip_pp_files.len();
    let pp_ratio = if total_files > 0 { pp_files.len() * 100 / total_files } else { 50 };
    let tokens_for_pp = tokens_limit * pp_ratio / 100;
    let tokens_for_skip = tokens_limit.saturating_sub(tokens_for_pp);

    let pp_result = postprocess_context_files(
        gcx.clone(),
        &mut pp_files,
        tokenizer.clone(),
        tokens_for_pp,
        false,
        &pp_settings,
    ).await;

    let skip_result = fill_skip_pp_files_with_budget(
        gcx.clone(),
        tokenizer.clone(),
        skip_pp_files,
        tokens_for_skip,
        existing_messages,
    ).await;

    let all_files: Vec<_> = pp_result.into_iter().chain(skip_result).collect();

    if all_files.is_empty() {
        return None;
    }

    Some(ChatMessage {
        role: "context_file".to_string(),
        content: ChatContent::ContextFiles(all_files),
        ..Default::default()
    })
}

async fn fill_skip_pp_files_with_budget(
    gcx: Arc<ARwLock<GlobalContext>>,
    tokenizer: Option<Arc<Tokenizer>>,
    files: Vec<ContextFile>,
    tokens_limit: usize,
    existing_messages: &[ChatMessage],
) -> Vec<ContextFile> {
    if files.is_empty() {
        return vec![];
    }

    let per_file_budget = tokens_limit / files.len().max(1);
    let mut result = Vec::new();

    for mut cf in files {
        if let Some(dup_info) = find_duplicate_in_history(&cf, existing_messages) {
            cf.file_content = format!(
                "📎 Already retrieved in message #{} via `{}`. Use narrower range if needed.",
                dup_info.0, dup_info.1
            );
            result.push(cf);
            continue;
        }

        match get_file_text_from_memory_or_disk(gcx.clone(), &PathBuf::from(&cf.file_name)).await {
            Ok(text) => {
                let lines: Vec<&str> = text.lines().collect();
                let total_lines = lines.len();

                if total_lines == 0 {
                    cf.file_content = String::new();
                    result.push(cf);
                    continue;
                }

                let start = normalize_line_start(cf.line1, total_lines);
                let end = normalize_line_end(cf.line2, total_lines, start);

                let content = format_lines_with_numbers(&lines, start, end);
                let tokens = count_text_tokens_with_fallback(tokenizer.clone(), &content);

                if tokens <= per_file_budget {
                    cf.file_content = content;
                    cf.line1 = start + 1;
                    cf.line2 = end;
                } else {
                    cf.file_content = truncate_file_head_tail(
                        &lines,
                        start,
                        end,
                        tokenizer.clone(),
                        per_file_budget,
                    );
                    cf.line1 = start + 1;
                    cf.line2 = end;
                }
                result.push(cf);
            }
            Err(e) => {
                warn!("Failed to load file {}: {}", cf.file_name, e);
                cf.file_content = format!("Error: {}", e);
                result.push(cf);
            }
        }
    }

    result
}

fn find_duplicate_in_history(cf: &ContextFile, messages: &[ChatMessage]) -> Option<(usize, String)> {
    let cf_canonical = canonical_path(&cf.file_name);
    for (idx, msg) in messages.iter().enumerate() {
        if msg.role != "context_file" {
            continue;
        }
        if let ChatContent::ContextFiles(files) = &msg.content {
            for existing in files {
                let existing_canonical = canonical_path(&existing.file_name);
                if existing_canonical == cf_canonical && ranges_overlap(existing, cf) {
                    let tool_name = find_tool_name_for_context(messages, idx);
                    return Some((idx, tool_name));
                }
            }
        }
    }
    None
}

fn ranges_overlap(a: &ContextFile, b: &ContextFile) -> bool {
    let a_start = if a.line1 == 0 { 1 } else { a.line1 };
    let a_end = if a.line2 == 0 { usize::MAX } else { a.line2 };
    let b_start = if b.line1 == 0 { 1 } else { b.line1 };
    let b_end = if b.line2 == 0 { usize::MAX } else { b.line2 };
    a_start <= b_end && b_start <= a_end
}

fn find_tool_name_for_context(messages: &[ChatMessage], context_idx: usize) -> String {
    for i in (0..context_idx).rev() {
        if messages[i].role == "tool" {
            let tool_call_id = &messages[i].tool_call_id;
            for j in (0..i).rev() {
                if let Some(calls) = messages[j].tool_calls.as_ref() {
                    for call in calls {
                        if &call.id == tool_call_id {
                            return call.function.name.clone();
                        }
                    }
                }
            }
            return "tool".to_string();
        }
    }
    "unknown".to_string()
}

fn normalize_line_start(line1: usize, total: usize) -> usize {
    if line1 == 0 {
        0
    } else {
        (line1.saturating_sub(1)).min(total)
    }
}

fn normalize_line_end(line2: usize, total: usize, start: usize) -> usize {
    if line2 == 0 {
        total
    } else {
        line2.min(total).max(start)
    }
}

fn format_lines_with_numbers(lines: &[&str], start: usize, end: usize) -> String {
    lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:4} | {}", start + i + 1, line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate_file_head_tail(
    lines: &[&str],
    start: usize,
    end: usize,
    tokenizer: Option<Arc<Tokenizer>>,
    tokens_limit: usize,
) -> String {
    let total_lines = end - start;
    let head_lines = (total_lines * 80 / 100).max(1);
    let tail_lines = (total_lines * 20 / 100).max(1);

    let mut head_end = start + head_lines.min(total_lines);
    let mut tail_start = end.saturating_sub(tail_lines);

    if tail_start <= head_end {
        tail_start = head_end;
    }

    loop {
        let head_content = format_lines_with_numbers(lines, start, head_end);
        let tail_content = if tail_start < end {
            format_lines_with_numbers(lines, tail_start, end)
        } else {
            String::new()
        };

        let truncation_marker = if tail_start > head_end {
            format!("\n... ({} lines omitted) ...\n", tail_start - head_end)
        } else {
            String::new()
        };

        let full_content = format!("{}{}{}", head_content, truncation_marker, tail_content);
        let tokens = count_text_tokens_with_fallback(tokenizer.clone(), &full_content);

        if tokens <= tokens_limit || head_end <= start + 1 {
            return full_content;
        }

        head_end = start + (head_end - start) * 80 / 100;
        if tail_start < end {
            tail_start = end - (end - tail_start) * 80 / 100;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_validation::{ChatToolCall, ChatToolFunction};

    fn make_context_file(name: &str, line1: usize, line2: usize) -> ContextFile {
        ContextFile {
            file_name: name.to_string(),
            file_content: String::new(),
            line1,
            line2,
            symbols: vec![],
            gradient_type: -1,
            usefulness: 0.0,
            skip_pp: false,
        }
    }

    fn make_tool_message(content: &str, tool_call_id: &str) -> ChatMessage {
        ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(content.to_string()),
            tool_call_id: tool_call_id.to_string(),
            ..Default::default()
        }
    }

    fn make_context_file_message(files: Vec<ContextFile>) -> ChatMessage {
        ChatMessage {
            role: "context_file".to_string(),
            content: ChatContent::ContextFiles(files),
            ..Default::default()
        }
    }

    fn make_assistant_with_tool_calls(tool_names: Vec<&str>) -> ChatMessage {
        ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("".to_string()),
            tool_calls: Some(tool_names.iter().enumerate().map(|(i, name)| {
                ChatToolCall {
                    id: format!("call_{}", i),
                    index: Some(i),
                    function: ChatToolFunction {
                        name: name.to_string(),
                        arguments: "{}".to_string(),
                    },
                    tool_type: "function".to_string(),
                }
            }).collect()),
            ..Default::default()
        }
    }

    #[test]
    fn test_tool_budget_from_n_ctx() {
        let budget = ToolBudget::try_from_n_ctx(8192).unwrap();
        assert_eq!(budget.tokens_for_code, 3276);
        assert_eq!(budget.tokens_for_text, 819);

        let budget_small = ToolBudget::try_from_n_ctx(1000);
        assert!(budget_small.is_err());
        assert!(budget_small.unwrap_err().contains("below minimum"));

        let budget_large = ToolBudget::try_from_n_ctx(128000).unwrap();
        assert_eq!(budget_large.tokens_for_code, 51200);
        assert_eq!(budget_large.tokens_for_text, 12800);
    }

    #[test]
    fn test_normalize_line_start() {
        assert_eq!(normalize_line_start(0, 100), 0);
        assert_eq!(normalize_line_start(1, 100), 0);
        assert_eq!(normalize_line_start(10, 100), 9);
        assert_eq!(normalize_line_start(200, 100), 100);
    }

    #[test]
    fn test_normalize_line_end() {
        assert_eq!(normalize_line_end(0, 100, 0), 100);
        assert_eq!(normalize_line_end(50, 100, 0), 50);
        assert_eq!(normalize_line_end(200, 100, 0), 100);
        assert_eq!(normalize_line_end(10, 100, 20), 20);
    }

    #[test]
    fn test_format_lines_with_numbers() {
        let lines = vec!["line1", "line2", "line3", "line4", "line5"];
        let result = format_lines_with_numbers(&lines, 0, 3);
        assert!(result.contains("   1 | line1"));
        assert!(result.contains("   2 | line2"));
        assert!(result.contains("   3 | line3"));
        assert!(!result.contains("line4"));

        let result2 = format_lines_with_numbers(&lines, 2, 5);
        assert!(result2.contains("   3 | line3"));
        assert!(result2.contains("   4 | line4"));
        assert!(result2.contains("   5 | line5"));
    }

    #[test]
    fn test_ranges_overlap() {
        let full = make_context_file("test.rs", 0, 0);
        let partial = make_context_file("test.rs", 10, 20);
        assert!(ranges_overlap(&full, &partial));

        let a = make_context_file("test.rs", 1, 10);
        let b = make_context_file("test.rs", 5, 15);
        assert!(ranges_overlap(&a, &b));

        let c = make_context_file("test.rs", 1, 10);
        let d = make_context_file("test.rs", 20, 30);
        assert!(!ranges_overlap(&c, &d));

        let e = make_context_file("test.rs", 1, 10);
        let f = make_context_file("test.rs", 10, 20);
        assert!(ranges_overlap(&e, &f));
    }

    #[test]
    fn test_find_duplicate_in_history_no_match() {
        let cf = make_context_file("new_file.rs", 1, 10);
        let messages = vec![
            make_context_file_message(vec![make_context_file("other.rs", 1, 10)]),
        ];
        assert!(find_duplicate_in_history(&cf, &messages).is_none());
    }

    #[test]
    fn test_find_duplicate_in_history_exact_match() {
        let cf = make_context_file("test.rs", 1, 10);
        let messages = vec![
            make_context_file_message(vec![make_context_file("test.rs", 1, 10)]),
        ];
        let result = find_duplicate_in_history(&cf, &messages);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, 0);
    }

    #[test]
    fn test_find_duplicate_in_history_overlapping() {
        let cf = make_context_file("test.rs", 5, 15);
        let messages = vec![
            make_context_file_message(vec![make_context_file("test.rs", 1, 10)]),
        ];
        let result = find_duplicate_in_history(&cf, &messages);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_duplicate_in_history_full_file_overlap() {
        let cf = make_context_file("test.rs", 0, 0);
        let messages = vec![
            make_context_file_message(vec![make_context_file("test.rs", 50, 100)]),
        ];
        let result = find_duplicate_in_history(&cf, &messages);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_tool_name_for_context() {
        let messages = vec![
            make_assistant_with_tool_calls(vec!["cat"]),
            make_tool_message("result", "call_0"),
            make_context_file_message(vec![make_context_file("test.rs", 1, 10)]),
        ];
        let name = find_tool_name_for_context(&messages, 2);
        assert_eq!(name, "cat");
    }

    #[test]
    fn test_find_tool_name_for_context_no_tool() {
        let messages = vec![
            make_context_file_message(vec![make_context_file("test.rs", 1, 10)]),
        ];
        let name = find_tool_name_for_context(&messages, 0);
        assert_eq!(name, "unknown");
    }

    #[test]
    fn test_truncate_file_head_tail() {
        let lines: Vec<&str> = (0..100).map(|_| "content").collect();
        let result = truncate_file_head_tail(&lines, 0, 100, None, 50);
        assert!(result.contains("   1 |"));
        assert!(result.contains("omitted"));
    }

    #[test]
    fn test_find_duplicate_path_normalization() {
        let cf = make_context_file("src/main.rs", 1, 10);
        let messages = vec![
            make_context_file_message(vec![make_context_file("src/main.rs", 1, 10)]),
        ];
        let result = find_duplicate_in_history(&cf, &messages);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_duplicate_different_files_same_basename() {
        let cf = make_context_file("src/a/main.rs", 1, 10);
        let messages = vec![
            make_context_file_message(vec![make_context_file("src/b/main.rs", 1, 10)]),
        ];
        let result = find_duplicate_in_history(&cf, &messages);
        assert!(result.is_none());
    }

    #[test]
    fn test_budget_ratio_all_skip_pp() {
        let skip_files = vec![
            ContextFile { skip_pp: true, ..make_context_file("a.rs", 1, 10) },
            ContextFile { skip_pp: true, ..make_context_file("b.rs", 1, 10) },
        ];
        let pp_files: Vec<ContextFile> = vec![];
        let total = skip_files.len() + pp_files.len();
        let pp_ratio = if total > 0 { pp_files.len() * 100 / total } else { 50 };
        assert_eq!(pp_ratio, 0);
    }

    #[test]
    fn test_budget_ratio_all_pp() {
        let skip_files: Vec<ContextFile> = vec![];
        let pp_files = vec![
            make_context_file("a.rs", 1, 10),
            make_context_file("b.rs", 1, 10),
        ];
        let total = skip_files.len() + pp_files.len();
        let pp_ratio = if total > 0 { pp_files.len() * 100 / total } else { 50 };
        assert_eq!(pp_ratio, 100);
    }

    #[test]
    fn test_budget_ratio_mixed() {
        let skip_files = vec![
            ContextFile { skip_pp: true, ..make_context_file("a.rs", 1, 10) },
        ];
        let pp_files = vec![
            make_context_file("b.rs", 1, 10),
            make_context_file("c.rs", 1, 10),
            make_context_file("d.rs", 1, 10),
        ];
        let total = skip_files.len() + pp_files.len();
        let pp_ratio = if total > 0 { pp_files.len() * 100 / total } else { 50 };
        assert_eq!(pp_ratio, 75);
    }

    #[test]
    fn test_find_tool_name_multiple_tools() {
        let messages = vec![
            make_assistant_with_tool_calls(vec!["tree", "cat", "search"]),
            make_tool_message("tree result", "call_0"),
            make_tool_message("cat result", "call_1"),
            make_context_file_message(vec![make_context_file("test.rs", 1, 10)]),
        ];
        let name = find_tool_name_for_context(&messages, 3);
        assert_eq!(name, "cat");
    }

    #[test]
    fn test_find_tool_name_correct_tool_call_id() {
        let messages = vec![
            make_assistant_with_tool_calls(vec!["tree", "cat"]),
            make_tool_message("tree result", "call_0"),
            make_context_file_message(vec![make_context_file("test.rs", 1, 10)]),
        ];
        let name = find_tool_name_for_context(&messages, 2);
        assert_eq!(name, "tree");
    }
}
