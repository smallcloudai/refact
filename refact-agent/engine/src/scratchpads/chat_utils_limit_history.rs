use std::collections::{HashMap, HashSet};
use itertools::Itertools;
use serde_json::Value;
use tracing::error;
use std::time::Instant;
use serde::{Serialize, Deserialize};
use crate::call_validation::{ChatMessage, ChatContent, ContextFile, SamplingParameters};
use crate::nicer_logs::first_n_chars;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::scratchpads::token_count_cache::TokenCountCache;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionStrength {
    Absent,
    Low,
    Medium,
    High,
}

/// Returns the appropriate token parameters for a given model.
/// 
/// # Model-Specific Token Parameters
///
/// Different models have different token overhead requirements for message formatting.
/// This module provides a mapping of model names to their appropriate token parameters.
///
/// | Model                | EXTRA_TOKENS_PER_MESSAGE | EXTRA_BUDGET_OFFSET_PERC |
/// |----------------------|--------------------------|--------------------------|
/// | claude-3-7-sonnet    | 150                      | 0.2 (20%)                |
/// | claude-3-5-sonnet    | 150                      | 0.2 (20%)                |
/// | All other models     | 3                        | 0.0 (0%)                 |
///
/// The `EXTRA_TOKENS_PER_MESSAGE` parameter represents the token overhead added to each message
/// in a conversation, accounting for formatting, role indicators, etc.
///
/// The `EXTRA_BUDGET_OFFSET_PERC` parameter represents an additional buffer percentage of the
/// context window that is reserved to ensure there's enough space for both the conversation
/// history and new generated tokens.
/// 
/// # Arguments
/// 
/// * `model_id` - Provider / Model name (e.g., "Refact/claude-3-7-sonnet")
/// 
/// # Returns
/// 
/// A tuple containing (EXTRA_TOKENS_PER_MESSAGE, EXTRA_BUDGET_OFFSET_PERC)
pub fn get_model_token_params(model_id: &str) -> (i32, f32) {
    let model_lower = model_id.to_lowercase();
    match model_lower.as_str() {
        // Claude 3-4 Sonnet models need higher token overhead
        m if m.contains("claude") => (150, 0.15),

        // Default values for all other models
        _ => (3, 0.0),
    }
}

fn recalculate_token_limits(
    token_counts: &Vec<i32>,
    tools_description_tokens: i32,
    n_ctx: usize,
    max_new_tokens: usize,
    model_id: &str,
) -> (i32, i32) {
    let occupied_tokens = token_counts.iter().sum::<i32>() + tools_description_tokens;
    
    let (_, extra_budget_offset_perc) = get_model_token_params(model_id);
    
    let extra_budget = (n_ctx as f32 * extra_budget_offset_perc) as usize;
    let tokens_limit = n_ctx.saturating_sub(max_new_tokens).saturating_sub(extra_budget) as i32;
    (occupied_tokens, tokens_limit)
}

fn compress_message_at_index(
    t: &HasTokenizerAndEot,
    mutable_messages: &mut Vec<ChatMessage>,
    token_counts: &mut Vec<i32>,
    token_cache: &mut TokenCountCache,
    index: usize,
    model_id: &str,
) -> Result<i32, String> {
    let role = &mutable_messages[index].role;
    let new_summary = if role == "context_file" {
        // For context files: parse to extract a list of file names
        let content_text_only = mutable_messages[index].content.content_text_only();
        let vector_of_context_files: Vec<ContextFile> = serde_json::from_str(&content_text_only)
            .map_err(|e| {
                error!("parsing context_files has failed: {}; content: {}", e, &content_text_only);
                format!("parsing context_files failed: {}", e)
            })
            .unwrap_or(vec![]);
        let filenames = vector_of_context_files.iter().map(|cf| cf.file_name.clone()).join(", ");
        tracing::info!("Compressing ContextFile message at index {}: {}", index, filenames);
        mutable_messages[index].role = "cd_instruction".to_string();
        format!("💿 '{}' files were dropped due to compression. Ask for these files again if needed. If you see this error again - files are too large to fit completely, try to open some part of it or just complain to user.", filenames)
    } else if role == "tool" || role == "diff" {
        // For tool/diff results: create a summary with the tool call ID and first part of content
        let content = mutable_messages[index].content.content_text_only();
        let tool_info = if !mutable_messages[index].tool_call_id.is_empty() {
            format!("for tool call {}", mutable_messages[index].tool_call_id)
        } else {
            "".to_string()
        };
        let preview = content.chars().take(30).collect::<String>();
        let preview_with_ellipsis = if content.len() > 30 { format!("{}...", &preview) } else { preview.clone() };
        tracing::info!("Compressing {} message at index {}: {}", role, index, &preview);
        format!("💿 {} result {} compressed: {}", if role == "diff" { "Diff" } else { "Tool" }, tool_info, preview_with_ellipsis)
    } else {
        let content = mutable_messages[index].content.content_text_only();
        let lines: Vec<&str> = content.lines().collect();
        
        if lines.len() > 20 {
            let head: Vec<&str> = lines.iter().take(10).cloned().collect();
            let tail: Vec<&str> = lines.iter().rev().take(10).rev().cloned().collect();
            let omitted = lines.len() - 20;
            
            format!(
                "💿 Message compressed ({} lines omitted):\n{}\n... [{} lines omitted] ...\n{}",
                omitted,
                head.join("\n"),
                omitted,
                tail.join("\n")
            )
        } else {
            let preview_start: String = content.chars().take(100).collect();
            let preview_end: String = content.chars().rev().take(100).collect::<String>().chars().rev().collect();
            tracing::info!("Compressing large message at index {}: {}", index, &preview_start);
            format!("💿 Message compressed: {}... (truncated) ...{}", preview_start, preview_end)
        }
    };
    
    mutable_messages[index].content = ChatContent::SimpleText(new_summary);
    token_cache.invalidate(&mutable_messages[index]);
    let (extra_tokens_per_message, _) = get_model_token_params(model_id);
    // Recalculate token usage after compression using the cache
    token_counts[index] = token_cache.get_token_count(&mutable_messages[index], t.tokenizer.clone(), extra_tokens_per_message)?;
    Ok(token_counts[index])
}

fn process_compression_stage(
    t: &HasTokenizerAndEot,
    mutable_messages: &mut Vec<ChatMessage>,
    token_counts: &mut Vec<i32>,
    token_cache: &mut TokenCountCache,
    tools_description_tokens: i32,
    n_ctx: usize,
    max_new_tokens: usize,
    start_idx: usize,
    end_idx: usize,
    stage_name: &str,
    model_id: &str,
    message_filter: impl Fn(usize, &ChatMessage, i32) -> bool,
    sort_by_size: bool,
) -> Result<(i32, i32, bool), String> {
    tracing::info!("n_ctx={n_ctx}, max_new_tokens={max_new_tokens}");
    tracing::info!("STAGE: {}", stage_name);
    let (mut occupied_tokens, tokens_limit) = 
        recalculate_token_limits(token_counts, tools_description_tokens, n_ctx, max_new_tokens, model_id);
    let mut budget_reached = false;
    let messages_len = mutable_messages.len();
    let end = std::cmp::min(end_idx, messages_len);
    
    let mut indices_to_process: Vec<(usize, i32)> = Vec::new();
    for i in start_idx..end {
        let should_process = {
            let msg = &mutable_messages[i];
            let token_count = token_counts[i];
            message_filter(i, msg, token_count)
        };
        
        if should_process {
            indices_to_process.push((i, token_counts[i]));
        }
    }
    
    // Sort indices by token count in descending order if requested
    if sort_by_size && indices_to_process.len() > 1 {
        indices_to_process.sort_by(|a, b| b.1.cmp(&a.1));
        tracing::info!("Sorted {} messages by token count for compression", indices_to_process.len());
    }
    
    for (i, original_tokens) in indices_to_process {
        compress_message_at_index(t, mutable_messages, token_counts, token_cache, i, model_id)?;
        let token_delta = token_counts[i] - original_tokens;
        occupied_tokens += token_delta;
        tracing::info!("Compressed message at index {}: token count {} -> {} (saved {})", 
                      i, original_tokens, token_counts[i], original_tokens - token_counts[i]);
        if occupied_tokens <= tokens_limit {
            tracing::info!("Token budget reached after {} compression.", stage_name);
            budget_reached = true;
            break;
        }
    }
    
    Ok((occupied_tokens, tokens_limit, budget_reached))
}

fn remove_invalid_tool_calls_and_tool_calls_results(messages: &mut Vec<ChatMessage>) {
    let tool_call_ids: HashSet<_> = messages.iter()
        .filter(|m| !m.tool_call_id.is_empty())
        .map(|m| &m.tool_call_id)
        .cloned()
        .collect();
    messages.retain(|m| {
        if let Some(tool_calls) = &m.tool_calls {
            let should_retain = tool_calls.iter().all(|tc| tool_call_ids.contains(&tc.id));
            if !should_retain {
                tracing::warn!("removing assistant message with unanswered tool tool_calls: {:?}", tool_calls);
            }
            should_retain
        } else {
            true
        }
    });

    let tool_call_ids: HashSet<_> = messages.iter()
        .filter_map(|x| x.tool_calls.clone())
        .flatten()
        .map(|x| x.id)
        .collect();
    messages.retain(|m| {
        if !m.tool_call_id.is_empty() && !tool_call_ids.contains(&m.tool_call_id) {
            tracing::warn!("removing tool result with no tool_call: {:?}", m);
            false
        } else {
            true
        }
    });

    // Remove duplicate tool results - keep only the last occurrence of each tool_call_id
    // Anthropic API requires exactly one tool_result per tool_use
    // For file edit operations, "diff" role typically comes after "tool" and contains cleaner output
    let mut last_occurrence: HashMap<String, usize> = HashMap::new();
    for (i, m) in messages.iter().enumerate() {
        if !m.tool_call_id.is_empty() {
            last_occurrence.insert(m.tool_call_id.clone(), i);
        }
    }
    let indices_to_keep: HashSet<usize> = last_occurrence.values().cloned().collect();
    let mut current_idx = 0usize;
    messages.retain(|m| {
        let idx = current_idx;
        current_idx += 1;
        if m.tool_call_id.is_empty() {
            true
        } else if indices_to_keep.contains(&idx) {
            true
        } else {
            tracing::warn!("removing duplicate tool result (role={}) for tool_call_id: {}", m.role, m.tool_call_id);
            false
        }
    });
}

/// Determines if two file contents have a duplication relationship (one contains the other).
/// Returns true if either content is substantially contained in the other.
pub(crate) fn is_content_duplicate(
    current_content: &str, 
    current_line1: usize, 
    current_line2: usize,
    first_content: &str, 
    first_line1: usize, 
    first_line2: usize
) -> bool {
    let lines_overlap = first_line1 <= current_line2 && first_line2 >= current_line1;
    // If line ranges don't overlap at all, it's definitely not a duplicate
    if !lines_overlap {
        return false;
    }
    // Consider empty contents are not duplicate
    if current_content.is_empty() || first_content.is_empty() {
        return false;
    }
    // Check if either content is entirely contained in the other (symmetric check)
    if first_content.contains(current_content) || current_content.contains(first_content) {
        return true;
    }
    // Check for substantial line overlap (either direction)
    let first_lines: HashSet<&str> = first_content.lines().filter(|x| !x.starts_with("...")).collect();
    let current_lines: HashSet<&str> = current_content.lines().filter(|x| !x.starts_with("...")).collect();
    let intersect_count = first_lines.intersection(&current_lines).count();
    
    // Either all of current's lines are in first, OR all of first's lines are in current
    let current_in_first = !current_lines.is_empty() && intersect_count >= current_lines.len();
    let first_in_current = !first_lines.is_empty() && intersect_count >= first_lines.len();
    
    current_in_first || first_in_current
}

/// Stage 0: Compress duplicate ContextFiles based on content comparison - keeping the LARGEST occurrence
pub(crate) fn compress_duplicate_context_files(messages: &mut Vec<ChatMessage>) -> Result<(usize, Vec<bool>), String> {
    #[derive(Debug, Clone)]
    struct ContextFileInfo {
        msg_idx: usize,
        cf_idx: usize,
        file_name: String,
        content: String,
        line1: usize,
        line2: usize,
        content_len: usize,
        is_compressed: bool,
    }
    
    // First pass: collect information about all context files
    let mut preserve_messages = vec![false; messages.len()];
    let mut all_files: Vec<ContextFileInfo> = Vec::new();
    for (msg_idx, msg) in messages.iter().enumerate() {
        if msg.role != "context_file" {
            continue;
        }
        let content_text = msg.content.content_text_only();
        let context_files: Vec<ContextFile> = match serde_json::from_str(&content_text) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Stage 0: Failed to parse ContextFile JSON at index {}: {}. Skipping.", msg_idx, e);
                continue;
            }
        };
        for (cf_idx, cf) in context_files.iter().enumerate() {
            all_files.push(ContextFileInfo {
                msg_idx,
                cf_idx,
                file_name: cf.file_name.clone(),
                content: cf.file_content.clone(),
                line1: cf.line1,
                line2: cf.line2,
                content_len: cf.file_content.len(),
                is_compressed: false,
            });
        }
    }
    
    // Group occurrences by file name
    let mut files_by_name: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, file) in all_files.iter().enumerate() {
        files_by_name.entry(file.file_name.clone())
            .or_insert_with(Vec::new)
            .push(i);
    }
    
    // Process each file's occurrences - keep the LARGEST one (prefer earlier if tied)
    for (filename, indices) in &files_by_name {
        if indices.len() <= 1 {
            continue;
        }
        
        // Find the index with the largest content; if tied, prefer earlier message (smaller msg_idx)
        let best_idx = *indices.iter()
            .max_by(|&&a, &&b| {
                let size_cmp = all_files[a].content_len.cmp(&all_files[b].content_len);
                if size_cmp == std::cmp::Ordering::Equal {
                    // When sizes equal, prefer EARLIER occurrence (smaller msg_idx)
                    all_files[b].msg_idx.cmp(&all_files[a].msg_idx)
                } else {
                    size_cmp
                }
            })
            .unwrap();
        let best_msg_idx = all_files[best_idx].msg_idx;
        preserve_messages[best_msg_idx] = true;
        
        tracing::info!("Stage 0: File {} - preserving best occurrence at message index {} ({} bytes)", 
            filename, best_msg_idx, all_files[best_idx].content_len);
        
        // Mark all other occurrences that are duplicates (subsets) of the best one for compression
        for &curr_idx in indices {
            if curr_idx == best_idx {
                continue;
            }
            let current_msg_idx = all_files[curr_idx].msg_idx;
            let content_is_duplicate = is_content_duplicate(
                &all_files[curr_idx].content, all_files[curr_idx].line1, all_files[curr_idx].line2,
                &all_files[best_idx].content, all_files[best_idx].line1, all_files[best_idx].line2
            );
            if content_is_duplicate {
                all_files[curr_idx].is_compressed = true;
                tracing::info!("Stage 0: Marking for compression - duplicate/subset of file {} at message index {} ({} bytes)", 
                    filename, current_msg_idx, all_files[curr_idx].content_len);
            } else {
                tracing::info!("Stage 0: Not compressing - unique content of file {} at message index {} (non-overlapping)", 
                    filename, current_msg_idx);
            }
        }
    }
    
    // Apply compressions to messages
    let mut compressed_count = 0;
    let mut modified_messages: HashSet<usize> = HashSet::new();
    for file in &all_files {
        if file.is_compressed && !modified_messages.contains(&file.msg_idx) {
            let content_text = messages[file.msg_idx].content.content_text_only();
            let context_files: Vec<ContextFile> = serde_json::from_str(&content_text)
                .expect("already checked in the previous pass");
            
            let mut remaining_files = Vec::new();
            let mut compressed_files = Vec::new();
            
            for (cf_idx, cf) in context_files.iter().enumerate() {
                if all_files.iter().any(|f| 
                    f.msg_idx == file.msg_idx && 
                    f.cf_idx == cf_idx && 
                    f.is_compressed
                ) {
                    compressed_files.push(format!("{}", cf.file_name));
                } else {
                    remaining_files.push(cf.clone());
                }
            }
            
            if !compressed_files.is_empty() {
                let compressed_files_str = compressed_files.join(", ");
                if remaining_files.is_empty() {
                    let summary = format!("💿 Duplicate files compressed: '{}' files were shown earlier in the conversation history. Do not ask for these files again.", compressed_files_str);
                    messages[file.msg_idx].content = ChatContent::SimpleText(summary);
                    messages[file.msg_idx].role = "cd_instruction".to_string();
                    tracing::info!("Stage 0: Fully compressed ContextFile at index {}: all {} files removed", 
                                  file.msg_idx, compressed_files.len());
                } else {
                    let new_content = serde_json::to_string(&remaining_files)
                        .expect("serialization of filtered ContextFiles failed");
                    messages[file.msg_idx].content = ChatContent::SimpleText(new_content);
                    tracing::info!("Stage 0: Partially compressed ContextFile at index {}: {} files removed, {} files kept", 
                                  file.msg_idx, compressed_files.len(), remaining_files.len());
                }
                
                compressed_count += compressed_files.len();
                modified_messages.insert(file.msg_idx);
            }
        }
    }
    
    Ok((compressed_count, preserve_messages))
}

fn replace_broken_tool_call_messages(
    messages: &mut Vec<ChatMessage>,
    sampling_parameters: &mut SamplingParameters,
    new_max_new_tokens: usize
) {
    let high_budget_tools = vec!["create_textdoc"];
    let last_index_assistant = messages.iter()
        .rposition(|msg| msg.role == "assistant")
        .unwrap_or(0);
    for (i, message) in messages.iter_mut().enumerate() {
        if let Some(tool_calls) = &mut message.tool_calls {
            let incorrect_reasons = tool_calls.iter().map(|tc| {
                match serde_json::from_str::<HashMap<String, Value>>(&tc.function.arguments) {
                    Ok(_) => None,
                    Err(err) => {
                        Some(format!("broken {}({}): {}", tc.function.name, first_n_chars(&tc.function.arguments, 100), err))
                    }
                }
            }).filter_map(|x| x).collect::<Vec<_>>();
            let has_high_budget_tools = tool_calls.iter().any(|tc| high_budget_tools.contains(&tc.function.name.as_str()));
            if !incorrect_reasons.is_empty() {
                // Only increase max_new_tokens if this is the last message and it was truncated due to "length"
                let extra_message = if i == last_index_assistant && message.finish_reason == Some("length".to_string()) {
                    tracing::warn!("increasing `max_new_tokens` from {} to {}", sampling_parameters.max_new_tokens, new_max_new_tokens);
                    let tokens_msg = if sampling_parameters.max_new_tokens < new_max_new_tokens {
                        sampling_parameters.max_new_tokens = new_max_new_tokens;
                        format!("The message was stripped (finish_reason=`length`), the tokens budget was too small for the tool calls. Increasing `max_new_tokens` to {new_max_new_tokens}.")
                    } else {
                        "The message was stripped (finish_reason=`length`), the tokens budget cannot fit those tool calls.".to_string()
                    };
                    if has_high_budget_tools {
                        format!("{tokens_msg} Try to make changes one by one (ie using `update_textdoc()`).")
                    } else {
                        format!("{tokens_msg} Change your strategy.")
                    }
                } else {
                    "".to_string()
                };

                let incorrect_reasons_concat = incorrect_reasons.join("\n");
                message.role = "cd_instruction".to_string();
                message.content = ChatContent::SimpleText(format!("💿 Previous tool calls are not valid: {incorrect_reasons_concat}.\n{extra_message}"));
                message.tool_calls = None;
                tracing::warn!(
                    "tool calls are broken, converting the tool call message to the `cd_instruction`:\n{:?}",
                    message.content.content_text_only()
                );
            }
        }
    }
}

fn validate_chat_history(
    messages: &Vec<ChatMessage>,
) -> Result<Vec<ChatMessage>, String> {
    // 1. Check that there is at least one message (and that at least one is "system" or "user")
    if messages.is_empty() {
        return Err("Invalid chat history: no messages present".to_string());
    }
    let has_system_or_user = messages.iter()
        .any(|msg| msg.role == "system" || msg.role == "user");
    if !has_system_or_user {
        return Err("Invalid chat history: must have at least one message of role 'system' or 'user'".to_string());
    }

    // 2. The first message must be system or user.
    if messages[0].role != "system" && messages[0].role != "user" {
        return Err(format!("Invalid chat history: first message must be 'system' or 'user', got '{}'", messages[0].role));
    }

    // 3. For every tool call in any message, verify its function arguments are parseable.
    for (msg_idx, msg) in messages.iter().enumerate() {
        if let Some(tool_calls) = &msg.tool_calls {
            for tc in tool_calls {
                if let Err(e) = serde_json::from_str::<HashMap<String, Value>>(&tc.function.arguments) {
                    return Err(format!(
                        "Message at index {} has an unparseable tool call arguments for tool '{}': {} (arguments: {})",
                        msg_idx, tc.function.name, e, tc.function.arguments));
                }
            }
        }
    }

    // 4. For each assistant message with nonempty tool_calls,
    //    check that every tool call id mentioned is later (i.e. at a higher index) answered by a tool message.
    for (idx, msg) in messages.iter().enumerate() {
        if msg.role == "assistant" {
            if let Some(tool_calls) = &msg.tool_calls {
                if !tool_calls.is_empty() {
                    for tc in tool_calls {
                        // Look for a following "tool" message whose tool_call_id equals tc.id
                        let mut found = false;
                        for later_msg in messages.iter().skip(idx + 1) {
                            if later_msg.tool_call_id == tc.id {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            return Err(format!(
                                "Assistant message at index {} has a tool call id '{}' that is unresponded (no following tool message with that id)",
                                idx, tc.id
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(messages.to_vec())
}

pub fn fix_and_limit_messages_history(
    t: &HasTokenizerAndEot,
    messages: &Vec<ChatMessage>,
    sampling_parameters_to_patch: &mut SamplingParameters,
    n_ctx: usize,
    tools_description: Option<String>,
    model_id: &str,
    use_compression: bool,
) -> Result<(Vec<ChatMessage>, CompressionStrength), String> {
    let start_time = Instant::now();

    if n_ctx <= sampling_parameters_to_patch.max_new_tokens {
        return Err(format!("bad input, n_ctx={}, max_new_tokens={}", n_ctx, sampling_parameters_to_patch.max_new_tokens));
    }

    // If compression is disabled, just validate and return messages as-is
    if !use_compression {
        tracing::info!("Compression disabled, skipping all compression stages");
        let mut mutable_messages = messages.clone();
        replace_broken_tool_call_messages(
            &mut mutable_messages,
            sampling_parameters_to_patch,
            16000
        );
        remove_invalid_tool_calls_and_tool_calls_results(&mut mutable_messages);
        return validate_chat_history(&mutable_messages).map(|msgs| (msgs, CompressionStrength::Absent));
    }

    let mut mutable_messages = messages.clone();
    let mut highest_compression_stage = 0;

    // STAGE 0: Compress duplicated ContextFiles
    // This is done before token calculation to reduce the number of messages that need to be tokenized
    let mut preserve_in_later_stages = vec![false; mutable_messages.len()];
    
    let stage0_result = compress_duplicate_context_files(&mut mutable_messages);
    if let Err(e) = &stage0_result {
        tracing::warn!("Stage 0 compression failed: {}", e);
    } else if let Ok((count, preservation_flags)) = stage0_result {
        tracing::info!("Stage 0: Compressed {} duplicate ContextFile messages", count);
        preserve_in_later_stages = preservation_flags;
    }
    
    replace_broken_tool_call_messages(
        &mut mutable_messages,
        sampling_parameters_to_patch,
        16000
    );

    let (extra_tokens_per_message, _) = get_model_token_params(model_id);
    let mut token_cache = TokenCountCache::new();
    let mut token_counts: Vec<i32> = Vec::with_capacity(mutable_messages.len());
    for msg in &mutable_messages {
        let count = token_cache.get_token_count(msg, t.tokenizer.clone(), extra_tokens_per_message)?;
        token_counts.push(count);
    }
    let tools_description_tokens = if let Some(desc) = tools_description.as_ref() {
        t.count_tokens(desc).unwrap_or(0)
    } else { 0 };
    let mut undroppable_msg_n = mutable_messages.iter()
        .rposition(|msg| msg.role == "user")
        .unwrap_or(0);
    tracing::info!("Calculated undroppable_msg_n = {} (last user message)", undroppable_msg_n);
    let outlier_threshold = 1000;
    let (mut occupied_tokens, mut tokens_limit) = 
        recalculate_token_limits(&token_counts, tools_description_tokens, n_ctx, sampling_parameters_to_patch.max_new_tokens, model_id);
    tracing::info!("Before compression: occupied_tokens={} vs tokens_limit={}", occupied_tokens, tokens_limit);
    
    // STAGE 1: Compress ContextFile messages before the last user message
    if occupied_tokens > tokens_limit {
        let msg_len = mutable_messages.len();
        let stage1_end = std::cmp::min(undroppable_msg_n, msg_len);
        let result = process_compression_stage(
            t, 
            &mut mutable_messages, 
            &mut token_counts,
            &mut token_cache,
            tools_description_tokens,
            n_ctx,
            sampling_parameters_to_patch.max_new_tokens,
            1, // Start from index 1 to preserve the initial message
            stage1_end,
            "Stage 1: Compressing ContextFile messages before the last user message",
            model_id,
            |i, msg, _| i != 0 && msg.role == "context_file" && !preserve_in_later_stages[i],
            true
        )?;
        
        occupied_tokens = result.0;
        tokens_limit = result.1;
        highest_compression_stage = 1;
        
        if result.2 { // If budget reached
            tracing::info!("Token budget reached after Stage 1 compression.");
        }
    }
    
    // STAGE 2: Compress Tool Result messages before the last user message
    if occupied_tokens > tokens_limit {
        let msg_len = mutable_messages.len();
        let stage2_end = std::cmp::min(undroppable_msg_n, msg_len);
        let result = process_compression_stage(
            t, 
            &mut mutable_messages, 
            &mut token_counts,
            &mut token_cache,
            tools_description_tokens,
            n_ctx,
            sampling_parameters_to_patch.max_new_tokens,
            1, // Start from index 1 to preserve the initial message
            stage2_end,
            "Stage 2: Compressing Tool Result messages before the last user message",
            model_id,
            |i, msg, _| i != 0 && (msg.role == "tool" || msg.role == "diff"),
            true
        )?;
        
        occupied_tokens = result.0;
        tokens_limit = result.1;
        highest_compression_stage = 2;
        
        if result.2 { // If budget reached
            tracing::info!("Token budget reached after Stage 2 compression.");
        }
    }
    
    // STAGE 3: Compress "outlier" messages before the last user message
    if occupied_tokens > tokens_limit {
        let msg_len = mutable_messages.len();
        let stage3_end = std::cmp::min(undroppable_msg_n, msg_len);
        let result = process_compression_stage(
            t, 
            &mut mutable_messages, 
            &mut token_counts,
            &mut token_cache,
            tools_description_tokens,
            n_ctx,
            sampling_parameters_to_patch.max_new_tokens,
            1, // Start from index 1 to preserve the initial message
            stage3_end,
            "Stage 3: Compressing outlier messages before the last user message",
            model_id,
            |i, msg, token_count| {
                i != 0 && 
                token_count > outlier_threshold && 
                msg.role != "context_file" && 
                msg.role != "tool" &&
                msg.role != "diff"
            },
            true
        )?;
        
        occupied_tokens = result.0;
        tokens_limit = result.1;
        highest_compression_stage = 3;
        
        if result.2 { // If budget reached
            tracing::info!("Token budget reached after Stage 3 compression.");
        }
    }

    // STAGE 4: Drop non-essential messages one by one within each block until budget is reached
    if occupied_tokens > tokens_limit {
        tracing::info!("STAGE 4: Iterating conversation blocks to drop non-essential messages");
        let mut current_occupied_tokens = occupied_tokens;
        let user_indices: Vec<usize> =
            mutable_messages.iter().enumerate().filter_map(|(i, m)| {
                if m.role == "user" { Some(i) } else { None }
            }).collect();

        let mut messages_ids_to_filter_out: HashSet<usize> = HashSet::new();
        for block_idx in 0..user_indices.len().saturating_sub(1) {
            let start_idx = user_indices[block_idx];
            let end_idx = user_indices[block_idx + 1];
            tracing::info!("Processing block {}: messages {}..{}", block_idx, start_idx, end_idx);
            if end_idx >= undroppable_msg_n || current_occupied_tokens <= tokens_limit {
                break;
            }
            let mut last_assistant_idx: Option<usize> = None;
            for i in (start_idx + 1..end_idx).rev() {
                if mutable_messages[i].role == "assistant" {
                    last_assistant_idx = Some(i);
                    break;
                }
            }

            for i in start_idx + 1..end_idx {
                if Some(i) != last_assistant_idx {
                    messages_ids_to_filter_out.insert(i);
                    let new_current_occupied_tokens = current_occupied_tokens - token_counts[i];
                    tracing::info!("Dropping message at index {} to stay under token limit: {} -> {}", i, current_occupied_tokens, new_current_occupied_tokens);
                    current_occupied_tokens = new_current_occupied_tokens;
                }
                if current_occupied_tokens <= tokens_limit {
                    break;
                }
            }
        }

        occupied_tokens = current_occupied_tokens;
        mutable_messages = mutable_messages
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !messages_ids_to_filter_out.contains(i))
            .map(|(_, x)| x)
            .collect();
        token_counts = token_counts
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !messages_ids_to_filter_out.contains(i))
            .map(|(_, x)| x)
            .collect();

        if !messages_ids_to_filter_out.is_empty() {
            highest_compression_stage = 4;
        }

        // Recalculate undroppable_msg_n after Stage 4 message removal
        // The old index is now stale since messages have been removed
        // NOTE: We update the outer mutable variable, not create a new shadowing one!
        undroppable_msg_n = mutable_messages.iter()
            .rposition(|msg| msg.role == "user")
            .unwrap_or(0);
        tracing::info!("Recalculated undroppable_msg_n = {} after Stage 4", undroppable_msg_n);

        tracing::info!(
            "Stage 4 complete: {} -> {} tokens ({} messages -> {} messages)", 
            occupied_tokens, current_occupied_tokens, mutable_messages.len() + messages_ids_to_filter_out.len(), 
            mutable_messages.len()
        );
        if occupied_tokens <= tokens_limit {
            tracing::info!("Token budget reached after Stage 4 compression.");
        }
    }

    // STAGE 5: Compress ContextFile messages after the last user message (last resort)
    if occupied_tokens > tokens_limit {
        tracing::warn!("Starting to compress messages in the last conversation block - this is a last resort measure");
        tracing::warn!("This may affect the quality of responses as we're now modifying the most recent context");
        let msg_len = mutable_messages.len();
        let result = process_compression_stage(
            t, 
            &mut mutable_messages, 
            &mut token_counts,
            &mut token_cache,
            tools_description_tokens,
            n_ctx,
            sampling_parameters_to_patch.max_new_tokens,
            undroppable_msg_n,
            msg_len,
            "Stage 5: Compressing ContextFile messages after the last user message (last resort)",
            model_id,
            |_, msg, _| msg.role == "context_file",
            true
        )?;
        
        occupied_tokens = result.0;
        tokens_limit = result.1;
        highest_compression_stage = 5;
        
        if result.2 { // If budget reached
            tracing::info!("Token budget reached after Stage 5 compression.");
        }
    }

    // STAGE 6: Compress Tool Result messages after the last user message (last resort)
    if occupied_tokens > tokens_limit {
        let msg_len = mutable_messages.len();
        let result = process_compression_stage(
            t,
            &mut mutable_messages,
            &mut token_counts,
            &mut token_cache,
            tools_description_tokens,
            n_ctx,
            sampling_parameters_to_patch.max_new_tokens,
            undroppable_msg_n,
            msg_len,
            "Stage 6: Compressing Tool Result messages after the last user message (last resort)",
            model_id,
            |_, msg, _| msg.role == "tool" || msg.role == "diff",
            true
        )?;
        
        occupied_tokens = result.0;
        tokens_limit = result.1;
        highest_compression_stage = 6;
        
        if result.2 { // If budget reached
            tracing::info!("Token budget reached after Stage 6 compression.");
        }
    }
    
    // STAGE 7: Compress "outlier" messages after the last user message, including the last user message (last resort)
    if occupied_tokens > tokens_limit {
        let msg_len = mutable_messages.len();
        let result = process_compression_stage(
            t, 
            &mut mutable_messages, 
            &mut token_counts,
            &mut token_cache,
            tools_description_tokens,
            n_ctx,
            sampling_parameters_to_patch.max_new_tokens,
            undroppable_msg_n,
            msg_len,
            "Stage 7: Compressing outlier messages in the last conversation block (last resort)",
            model_id,
            |i, msg, token_count| {
                i >= undroppable_msg_n &&
                token_count > outlier_threshold && 
                msg.role != "context_file" && 
                msg.role != "tool" &&
                msg.role != "diff"
            },
            false
        )?;
        
        highest_compression_stage = 7;
        
        if result.2 { // If budget reached
            tracing::info!("Token budget reached after Stage 7 compression.");
        }
    }

    remove_invalid_tool_calls_and_tool_calls_results(&mut mutable_messages);
    // Recalculate token counts after removing invalid tool calls, as the message count may have changed
    let mut token_counts: Vec<i32> = Vec::with_capacity(mutable_messages.len());
    for msg in &mutable_messages {
        let count = token_cache.get_token_count(msg, t.tokenizer.clone(), extra_tokens_per_message)?;
        token_counts.push(count);
    }
    let (occupied_tokens, tokens_limit) =
        recalculate_token_limits(&token_counts, tools_description_tokens, n_ctx, sampling_parameters_to_patch.max_new_tokens, model_id);
    tracing::info!("Final occupied_tokens={} <= tokens_limit={}", occupied_tokens, tokens_limit);

    // If we're still over the limit after all compression stages, return an error
    if occupied_tokens > tokens_limit {
        return Err("Cannot compress chat history enough: the mandatory messages still exceed the allowed token budget. Please start the new chat session.".to_string());
    }

    let (hits, misses, hit_rate) = token_cache.stats();
    tracing::info!("Tokenizer cache stats: {} hits, {} misses, {:.2}% hit rate", 
                  hits, misses, hit_rate * 100.0);
    
    let total_duration = start_time.elapsed();
    tracing::info!("Total compression time: {:?}", total_duration);
    
    let compression_strength = match highest_compression_stage {
        0 => CompressionStrength::Absent,
        1..=3 => CompressionStrength::Low,
        4 => CompressionStrength::Medium,
        5..=7 => CompressionStrength::High,
        _ => CompressionStrength::High,
    };
    tracing::info!("Used compression stage {} resulting in {:?} compression strength",
                  highest_compression_stage, compression_strength);

    // Insert cd_instruction message to instruct the model to prompt the user about compression
    let compression_notice = match compression_strength {
        CompressionStrength::Low => Some(
            "💿 Light compression was applied to fit the context window. \
            Inform the user that some older context has been summarized and suggest they press the 'Compress Chat' button to save tokens."
        ),
        CompressionStrength::Medium => Some(
            "💿 Medium compression was applied - conversation blocks were dropped. \
            Strongly recommend to the user that they press the 'Compress Chat' button to create a summary and continue with fresh context. \
            Explain this will significantly reduce token costs."
        ),
        CompressionStrength::High => Some(
            "💿 Heavy compression was applied affecting recent context quality. \
            Urgently prompt the user to press the 'Compress Chat' button immediately. \
            Warn them that continuing without compression will waste tokens and degrade response quality."
        ),
        CompressionStrength::Absent => None,
    };

    if let Some(notice) = compression_notice {
        let compression_msg = ChatMessage {
            role: "cd_instruction".to_string(),
            content: ChatContent::SimpleText(notice.to_string()),
            finish_reason: None,
            tool_calls: None,
            tool_call_id: String::new(),
            tool_failed: None,
            usage: None,
            checkpoints: Vec::new(),
            thinking_blocks: None,
            output_filter: None,
        };
        mutable_messages.push(compression_msg);
    }

    validate_chat_history(&mutable_messages).map(|msgs| (msgs, compression_strength))
}

