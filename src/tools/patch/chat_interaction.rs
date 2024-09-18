use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::RwLock as ARwLock;
use tokenizers::Tokenizer;
use tokio::sync::Mutex as AMutex;
use tracing::warn;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::{context_file_from_file_path, file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::privacy::load_privacy_if_needed;
use crate::tools::patch::snippets::CodeSnippet;
use crate::tools::patch::tool_patch::{DefaultToolPatch, N_CHOICES};
use crate::subchat::subchat_single;
use crate::cached_tokenizers::cached_tokenizer;
use crate::call_validation::{ChatMessage, ChatUsage, ContextFile, DiffChunk};
use crate::files_correction::get_project_dirs;
use crate::global_context::{GlobalContext, try_load_caps_quickly_if_not_present};
use crate::scratchpads::scratchpad_utils::count_tokens;


pub async fn read_file(
    gcx: Arc<ARwLock<GlobalContext>>,
    file_path: String,
) -> Result<ContextFile, String> {
    let candidates = file_repair_candidates(gcx.clone(), &file_path, 10, false).await;
    let candidate = return_one_candidate_or_a_good_error(
        gcx.clone(), &file_path, &candidates, &get_project_dirs(gcx.clone()).await, false
    ).await?;
    context_file_from_file_path(gcx.clone(), vec![candidate], file_path.clone()).await
}

async fn load_tokenizer(
    gcx: Arc<ARwLock<GlobalContext>>,
    model: &str,
) -> Result<Arc<StdRwLock<Tokenizer>>, String> {
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await.map_err(|e| {
        warn!("load_tokenizer: failed to load caps.\nERROR: {}", e);
        format!("load_tokenizer: failed to load caps.\nERROR: {}", e)
    })?;
    cached_tokenizer(caps.clone(), gcx.clone(), model.to_string()).await
}

async fn format_diff_prompt(gcx: Arc<ARwLock<GlobalContext>>) -> String {
    let workspace_dirs = {
        let dirs = get_project_dirs(gcx.clone()).await.into_iter()
            .map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>();
        if dirs.is_empty() {
            vec!["/home/user/project".to_string()]
        } else {
            dirs
        }
    };
    DefaultToolPatch::prompt(workspace_dirs)
}

async fn make_chat_history(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model: &str,
    max_tokens: usize,
    max_new_tokens: usize,
    snippets: Vec<CodeSnippet>,
) -> Result<Vec<ChatMessage>, String> {
    let gcx = ccx.lock().await.global_context.clone();
    let tokenizer = {
        let tokenizer_arc = load_tokenizer(gcx.clone(), model).await?;
        tokenizer_arc.clone().read().unwrap().clone()
    };

    let mut tokens = 0;
    let max_tokens = max_tokens.saturating_sub(max_new_tokens);
    let system_prompt = format_diff_prompt(gcx.clone()).await;
    
    let snippet0 = snippets.get(0).expect("no snippet provided");
    let context_file = read_file(gcx.clone(), snippet0.filename_before.clone()).await
        .map_err(|e| format!("Cannot read file to modify: {}.\nERROR: {}", snippet0.filename_before, e))?;

    let mut chat_messages = vec![];

    tokens += 3 + count_tokens(&tokenizer, &system_prompt);
    chat_messages.push(ChatMessage::new("system".to_string(), system_prompt));

    let code = format!(
        "File: {}\nContent:\n```\n{}\n```",
        context_file.file_name,
        context_file.file_content
    ).to_string();
    tokens += 3 + count_tokens(&tokenizer, &code);
    chat_messages.push(ChatMessage::new("user".to_string(), code));

    for snippet in snippets {
        let section = format!(
            "Modified section:\n```\n{}\n```",
            snippet.code
        );
        tokens += 3 + count_tokens(&tokenizer, &section);
        chat_messages.push(ChatMessage::new("user".to_string(), section));
    }
    
    if tokens > max_tokens {
        return Err(format!(
            "the provided file {} is too large for the patch tool: {tokens} > {max_tokens}",
            context_file.file_name,
        ));
    }
    
    Ok(chat_messages)
}

pub async fn execute_chat_model(
    ccx: Arc<AMutex<AtCommandsContext>>,
    snippets: Vec<CodeSnippet>,
    model: &str,
    max_tokens: usize,
    temperature: Option<f32>,
    max_new_tokens: usize,
    tool_call_id: &String,
    usage: &mut ChatUsage,
) -> Result<Vec<Vec<DiffChunk>>, (String, Option<String>)> {
    let messages = make_chat_history(
        ccx.clone(), model, max_tokens, max_new_tokens, snippets,
    ).await.map_err(|e|(e, None))?;
    let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let response = subchat_single(
        ccx.clone(),
        model,
        messages,
        vec![],
        None,
        false,
        temperature,
        Some(max_new_tokens),
        N_CHOICES,
        Some(usage),
        Some(format!("{log_prefix}-patch")),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-patch")),
    ).await.map_err(|e|(e, None))?;

    let last_messages = response.iter()
        .filter_map(|x| x.iter().last())
        .filter(|x| x.role == "assistant")
        .collect::<Vec<_>>();

    // what does succ even mean?
    let mut succ_chunks = vec![];
    let gcx = ccx.lock().await.global_context.clone();
    let privacy_settings = load_privacy_if_needed(gcx.clone()).await;
    for m in last_messages {
        match DefaultToolPatch::parse_message(m.content.as_str(), privacy_settings.clone()).await {
            Ok(chunks) => {
                succ_chunks.push(chunks);
            }
            Err(err) => {
                return Err((
                    format!("diff parsing error: {err}"), 
                    Some("tickets are invalid. Create new tickets from scratch".to_string())
                ));
            }
        };
    }
    Ok(succ_chunks)
}
