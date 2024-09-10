use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokenizers::Tokenizer;
use tokio::sync::Mutex as AMutex;
use tracing::warn;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::{context_file_from_file_path, file_repair_candidates};
use crate::tools::patch::snippets::CodeSnippet;
use crate::tools::patch::tool_patch::{DefaultToolPatch, N_CHOICES};
use crate::subchat::subchat_single;
use crate::cached_tokenizers;
use crate::call_validation::{ChatMessage, ChatUsage, ContextFile, DiffChunk};
use crate::scratchpads::scratchpad_utils::count_tokens;


pub async fn read_file(
    ccx: Arc<AMutex<AtCommandsContext>>,
    file_path: String,
) -> Option<ContextFile> {
    let gcx = ccx.lock().await.global_context.clone();
    let candidates = file_repair_candidates(gcx.clone(), &file_path, 10, false).await;
    match context_file_from_file_path(ccx.clone(), candidates, file_path.clone()).await {
        Ok(x) => Some(x),
        Err(_) => None
    }
}


async fn load_tokenizer(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model: &str,
) -> Result<Arc<StdRwLock<Tokenizer>>, String> {
    let gcx = ccx.lock().await.global_context.clone();
    let caps = crate::global_context::try_load_caps_quickly_if_not_present(
        gcx.clone(), 0,
    )
        .await
        .map_err(|e| {
            warn!("no caps: {:?}", e);
            "network error communicating with the model (1)".to_string()
        })?;

    cached_tokenizers::cached_tokenizer(
        caps.clone(), gcx.clone(), model.to_string(),
    ).await
}

async fn format_diff_prompt(
    ccx: Arc<AMutex<AtCommandsContext>>,
) -> String {
    let gcx = ccx.lock().await.global_context.clone();
    let mut workspace_dirs = {
        let workspace_dirs_arc = gcx.read().await.documents_state.workspace_folders.clone();
        let dirs_lock = workspace_dirs_arc.lock().unwrap();
        dirs_lock.clone().into_iter().map(|x| x.to_string_lossy().to_string()).collect::<Vec<_>>()
    };
    if workspace_dirs.is_empty() {
        workspace_dirs.push(String::from("/home/user/project"));
    }
    let workspace_project_dirs = workspace_dirs.join("\n");
    let first_workspace_dir = workspace_dirs.first().expect("added above");
    DefaultToolPatch::prompt(&workspace_project_dirs, first_workspace_dir)
}


async fn make_chat_history(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model: &str,
    max_tokens: usize,
    max_new_tokens: usize,
    snippet: &CodeSnippet
) -> Result<Vec<ChatMessage>, String> {
    let tokenizer = match load_tokenizer(ccx.clone(), model).await {
        Ok(t) => t,
        Err(e) => return Err(e),
    };

    let mut tokens: usize = 0;
    let max_tokens: usize = max_tokens.saturating_sub(max_new_tokens);
    let system_prompt = format_diff_prompt(ccx.clone()).await;
    let tokenizer_ref = tokenizer.read().unwrap().clone();

    let file_info = match read_file(ccx.clone(), snippet.filename_before.clone()).await {
        Some(text) => text,
        None => {
            return Err(format!("file to modify not found: {}", snippet.filename_before));
        }
    };
    let mut chat_messages = vec![
        ChatMessage::new(
            "system".to_string(),
            system_prompt.to_string(),
        )
    ];
    let code = format!(
        "File: {}\nContent:\n```\n{}\n```",
        file_info.file_name,
        file_info.file_content
    ).to_string();
    let section = format!(
        "Modified section:\n```\n{}\n```",
        snippet.code
    );

    tokens += 3 + count_tokens(&tokenizer_ref, &system_prompt);
    tokens += 3 + count_tokens(&tokenizer_ref, &code);
    tokens += 3 + count_tokens(&tokenizer_ref, &section);
    if tokens > max_tokens {
        return Err(format!(
            "the provided file {} is too large for the patch tool: {tokens} > {max_tokens}",
            file_info.file_name,
        ));
    }

    chat_messages.push(ChatMessage::new("user".to_string(), code));
    chat_messages.push(ChatMessage::new("user".to_string(), section));
    Ok(chat_messages)
}

pub async fn execute_chat_model(
    ccx: Arc<AMutex<AtCommandsContext>>,
    snippet: &CodeSnippet,
    model: &str,
    max_tokens: usize,
    temperature: Option<f32>,
    max_new_tokens: usize,
    tool_call_id: &String,
    usage: &mut ChatUsage,
) -> Result<Vec<Vec<DiffChunk>>, String> {
    let messages = make_chat_history(
        ccx.clone(), model, max_tokens, max_new_tokens, snippet,
    ).await?;
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
    ).await;

    let last_messages = match response {
        Ok(res) => {
            Ok(res
                .iter()
                .filter_map(|x| x
                    .iter()
                    .last()
                    .map(|x| {
                        if x.role == "assistant" { Some(x.content.clone()) } else { None }
                    })
                    .flatten())
                .collect::<Vec<_>>())
        }
        Err(err) => Err(err)
    }?;

    let mut succ_chunks = vec![];
    for m in last_messages {
        match DefaultToolPatch::parse_message(&m).await {
            Ok(chunks) => {
                succ_chunks.push(chunks);
            }
            Err(err) => {
                return Err(format!("Error while diff parsing: {:?}", err));
            }
        };
    }
    Ok(succ_chunks)
}