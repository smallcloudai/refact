use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokenizers::Tokenizer;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tracing::{info, warn};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::cached_tokenizers::cached_tokenizer;
use crate::call_validation::{ChatMessage, ChatUsage, DiffChunk, SubchatParameters};
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::subchat::subchat_single;
use crate::tools::tool_apply_edit_aux::fs_utils::read_file;
use crate::tools::tool_apply_edit_aux::model_based_edit::replace_file_parser::WholeFileParser;
use crate::tools::tool_apply_edit_aux::model_based_edit::section_edit_parser::{
    section_edit_choose_correct_chunk, BlocksOfCodeParser,
};
use crate::tools::tool_apply_edit_aux::postprocessing_utils::postprocess_diff_chunks;
use crate::tools::tool_apply_edit_aux::tickets_parsing::TicketToApply;

const DEBUG: bool = true;

async fn load_tokenizer(
    gcx: Arc<ARwLock<GlobalContext>>,
    model: &str,
) -> Result<Arc<StdRwLock<Tokenizer>>, String> {
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0)
        .await
        .map_err(|e| {
            warn!("load_tokenizer: failed to load caps.\nERROR: {}", e);
            format!("load_tokenizer: failed to load caps.\nERROR: {}", e)
        })?;
    cached_tokenizer(caps.clone(), gcx.clone(), model.to_string()).await
}

async fn make_chat_history(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model: &str,
    max_tokens: usize,
    max_new_tokens: usize,
    ticket: &TicketToApply,
    use_whole_file_parser: bool,
) -> Result<Vec<ChatMessage>, String> {
    let gcx = ccx.lock().await.global_context.clone();
    let tokenizer_arc = load_tokenizer(gcx.clone(), model).await?;

    let max_tokens = max_tokens.saturating_sub(max_new_tokens);
    let context_file = read_file(gcx.clone(), ticket.filename.clone())
        .await
        .map_err(|e| {
            format!(
                "Cannot read file to modify: {}.\nERROR: {}",
                ticket.filename, e
            )
        })?;

    let mut messages = vec![];
    let system_prompt = if use_whole_file_parser {
        WholeFileParser::prompt()
    } else {
        BlocksOfCodeParser::prompt()
    };
    messages.push(ChatMessage::new("system".to_string(), system_prompt));
    messages.push(ChatMessage::new(
        "user".to_string(),
        format!(
            "File: {}\nContent:\n```\n{}\n```",
            context_file.file_name, context_file.file_content
        )
        .to_string(),
    ));
    messages.push(ChatMessage::new("user".to_string(), if ticket.hint_message.is_empty() {
        format!(
            "Modified section:\n```\n{}\n```",
            ticket.code
        )
    } else {
        format!(
            "The hints (FOLLOW them to produce correct changes!):\n```\n{}\n```\nModified section:\n```\n{}\n```",
            ticket.hint_message,
            ticket.code
        )
    }));

    let tokens = messages
        .iter()
        .map(|x| {
            3 + x
                .content
                .count_tokens(tokenizer_arc.clone(), &None)
                .unwrap_or(0) as usize
        })
        .sum::<usize>();
    if tokens > max_tokens {
        return Err(format!(
            "the provided file {} is too large for the apply_edit tool: {tokens} > {max_tokens}",
            context_file.file_name,
        ));
    }

    if DEBUG {
        info!(
            "Using {} prompt in the `SECTION_EDIT` diff generation",
            if use_whole_file_parser {
                "whole_file"
            } else {
                "file_blocks"
            }
        );
        for m in messages.iter() {
            info!("{}", m.content.content_text_only());
        }
    }

    Ok(messages)
}

async fn make_follow_up_chat_history(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model: &str,
    max_tokens: usize,
    max_new_tokens: usize,
    messages: &mut Vec<ChatMessage>,
    last_message: &ChatMessage,
    error: &String,
) -> Result<(), String> {
    let gcx = ccx.lock().await.global_context.clone();
    let tokenizer_arc = load_tokenizer(gcx.clone(), model).await?;
    let max_tokens = max_tokens.saturating_sub(max_new_tokens);

    messages.push(last_message.clone());
    messages.push(ChatMessage::new(
        "user".to_string(),
        BlocksOfCodeParser::followup_prompt(error),
    ));
    if DEBUG {
        for m in messages.iter() {
            info!("{}", m.content.content_text_only());
        }
    }

    let tokens = messages
        .iter()
        .map(|x| {
            3 + x
                .content
                .count_tokens(tokenizer_arc.clone(), &None)
                .unwrap_or(0) as usize
        })
        .sum::<usize>();
    if tokens > max_tokens {
        return Err(format!(
            "All generated patches were invalid, but cannot make a follow-up, not enough tokens: {tokens} > {max_tokens}",
        ));
    }
    Ok(())
}

pub async fn get_valid_chunks_from_messages(
    ccx: Arc<AMutex<AtCommandsContext>>,
    filename: &PathBuf,
    messages: &Vec<ChatMessage>,
    use_whole_file_parser: bool,
) -> Vec<Result<Vec<DiffChunk>, String>> {
    let mut chunks = vec![];
    let mut tasks = vec![];
    for m in messages {
        let filename = filename.clone();
        let content = m.content.clone();
        let gcx = ccx.lock().await.global_context.clone();
        tasks.push(tokio::spawn(async move {
            if use_whole_file_parser {
                WholeFileParser::parse_message(
                    gcx.clone(),
                    content.content_text_only().as_str(),
                    &filename,
                )
                .await
            } else {
                BlocksOfCodeParser::parse_message(
                    gcx.clone(),
                    content.content_text_only().as_str(),
                    &filename,
                )
                .await
            }
        }));
    }

    for task in tasks {
        match task.await {
            Ok(Ok(c)) => {
                chunks.push(Ok(c));
            }
            Ok(Err(err)) => {
                warn!("diff parsing error: {err}");
                chunks.push(Err(err));
            }
            Err(err) => {
                chunks.push(Err(format!("task join error: {err}")));
            }
        }
    }
    chunks
}
pub async fn execute_blocks_of_code_patch(
    ccx: Arc<AMutex<AtCommandsContext>>,
    ticket: &TicketToApply,
    model: &str,
    max_tokens: usize,
    temperature: Option<f32>,
    max_new_tokens: usize,
    tool_call_id: &String,
    usage: &mut ChatUsage,
) -> Result<Vec<Vec<DiffChunk>>, (String, Option<String>)> {
    let filename = PathBuf::from(&ticket.filename);
    let mut messages = make_chat_history(
        ccx.clone(),
        model,
        max_tokens,
        max_new_tokens,
        ticket,
        false,
    )
    .await
    .map_err(|e| (e, None))?;
    let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let response = subchat_single(
        ccx.clone(),
        model,
        messages.clone(),
        Some(vec![]),
        None,
        false,
        temperature,
        Some(max_new_tokens),
        1,
        None,
        true,
        Some(usage),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-apply_edit")),
    )
    .await
    .map_err(|e| (e, None))?;

    let last_messages = response
        .iter()
        .filter_map(|x| x.iter().last())
        .filter(|x| x.role == "assistant")
        .cloned()
        .collect::<Vec<_>>();
    if DEBUG {
        info!("apply_edit responses: ");
        for (idx, m) in last_messages.iter().enumerate() {
            info!("choice {idx}:\n{}", m.content.content_text_only());
        }
    }
    let chunks =
        get_valid_chunks_from_messages(ccx.clone(), &filename, &last_messages, false).await;
    if chunks.is_empty() || chunks.iter().any(|x| x.is_ok()) {
        return Ok(chunks
            .iter()
            .map(|x| x.clone().ok())
            .filter_map(|x| x)
            .collect());
    }

    // If every chunk is an error, trying a follow-up iteration
    warn!(
        "no valid chunks after first iteration, making a follow-up in order to get a valid patch"
    );
    if let Err(err) = make_follow_up_chat_history(
        ccx.clone(),
        model,
        max_tokens,
        max_new_tokens,
        &mut messages,
        &last_messages
            .first()
            .expect("no messages returned from `subchat_single`")
            .clone(),
        &chunks
            .first()
            .expect("no messages returned from `subchat_single`")
            .clone()
            .err()
            .unwrap_or("".to_string()),
    )
    .await
    {
        return Err((
            err,
            Some("tickets are invalid. Create new tickets from scratch. If file is that big, use FULL_REWRITE".to_string())
        ));
    };
    let response = subchat_single(
        ccx.clone(),
        model,
        messages,
        Some(vec![]),
        None,
        false,
        Some(0.2),
        Some(max_new_tokens),
        4,
        None,
        true,
        Some(usage),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-apply_edit")),
    )
    .await
    .map_err(|e| (e, None))?;
    let last_messages = response
        .iter()
        .filter_map(|x| x.iter().last())
        .filter(|x| x.role == "assistant")
        .cloned()
        .collect::<Vec<_>>();
    if DEBUG {
        info!("follow-up apply_edit responses: ");
        for (idx, m) in last_messages.iter().enumerate() {
            info!("choice {idx}:\n{}", m.content.content_text_only());
        }
    }
    let chunks =
        get_valid_chunks_from_messages(ccx.clone(), &filename, &last_messages, false).await;
    if chunks.is_empty() || chunks.iter().any(|x| x.is_ok()) {
        Ok(chunks
            .iter()
            .map(|x| x.clone().ok())
            .filter_map(|x| x)
            .collect())
    } else {
        Err((
            "after a follow-up, all diffs were parsed with errors".to_string(),
            Some("tickets are invalid. Create new tickets from scratch. If file is that big, use FULL_REWRITE".to_string())
        ))
    }
}

pub async fn execute_whole_file_patch(
    ccx: Arc<AMutex<AtCommandsContext>>,
    ticket: &TicketToApply,
    model: &str,
    max_tokens: usize,
    max_new_tokens: usize,
    tool_call_id: &String,
    usage: &mut ChatUsage,
) -> Result<Vec<Vec<DiffChunk>>, (String, Option<String>)> {
    let filename = PathBuf::from(&ticket.filename);
    let messages = make_chat_history(
        ccx.clone(),
        model,
        max_tokens,
        max_new_tokens,
        ticket,
        true,
    )
    .await
    .map_err(|e| (e, None))?;
    let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let response = subchat_single(
        ccx.clone(),
        model,
        messages.clone(),
        Some(vec![]),
        None,
        false,
        Some(0.1),
        Some(max_new_tokens),
        1,
        None,
        true,
        Some(usage),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-apply_edit")),
    )
    .await
    .map_err(|e| (e, None))?;
    let last_messages = response
        .iter()
        .filter_map(|x| x.iter().last())
        .filter(|x| x.role == "assistant")
        .cloned()
        .collect::<Vec<_>>();
    if DEBUG {
        info!("apply_edit responses: ");
        for (idx, m) in last_messages.iter().enumerate() {
            info!("choice {idx}:\n{}", m.content.content_text_only());
        }
    }
    let chunks = get_valid_chunks_from_messages(ccx.clone(), &filename, &last_messages, true).await;
    if chunks.iter().any(|x| x.is_ok()) {
        Ok(chunks
            .iter()
            .map(|x| x.clone().ok())
            .filter_map(|x| x)
            .collect())
    } else {
        Err((
            "all diffs were parsed with errors".to_string(),
            Some("tickets are invalid. Create new tickets from scratch. If file is that big, use FULL_REWRITE".to_string())
        ))
    }
}

pub async fn section_edit_tickets_to_chunks(
    ccx_subchat: Arc<AMutex<AtCommandsContext>>,
    ticket: &TicketToApply,
    params: &SubchatParameters,
    tool_call_id: &String,
    usage: &mut ChatUsage,
) -> Result<Vec<DiffChunk>, (String, Option<String>)> {
    let gcx = ccx_subchat.lock().await.global_context.clone();
    let mut all_chunks = match execute_blocks_of_code_patch(
        ccx_subchat.clone(),
        ticket,
        &params.subchat_model,
        params.subchat_n_ctx,
        params.subchat_temperature,
        params.subchat_max_new_tokens,
        tool_call_id,
        usage,
    )
    .await
    {
        Ok(chunks) => Ok(chunks),
        Err((err, _)) => {
            warn!("cannot apply tickets to the file, error: {err}");
            warn!("trying a fallback `whole_file_rewrite` prompt");
            execute_whole_file_patch(
                ccx_subchat.clone(),
                ticket,
                &params.subchat_model,
                params.subchat_n_ctx,
                params.subchat_max_new_tokens,
                tool_call_id,
                usage,
            )
            .await
        }
    }?;
    let mut chunks_for_answers = vec![];
    for chunks in all_chunks.iter_mut() {
        let diffs = if !chunks.is_empty() {
            postprocess_diff_chunks(gcx.clone(), chunks).await
        } else {
            Ok(vec![])
        };
        chunks_for_answers.push(diffs);
    }
    section_edit_choose_correct_chunk(chunks_for_answers).map_err(|e| (e, None))
}
