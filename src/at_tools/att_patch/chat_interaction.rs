use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokenizers::Tokenizer;
use tokio::sync::Mutex as AMutex;
use tracing::{info, warn};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::execute_at_file;
use crate::at_tools::att_patch::args_parser::PatchArguments;
use crate::at_tools::att_patch::ast_interaction::{get_signatures_by_imports_traversal, get_signatures_by_symbol_names};
use crate::at_tools::att_patch::tool::{DefaultToolPatch, DEFAULT_MODEL_NAME, MAX_NEW_TOKENS, N_CHOICES, TEMPERATURE};
use crate::at_tools::subchat::subchat_single;
use crate::cached_tokenizers;
use crate::call_validation::ChatMessage;
use crate::scratchpads::pp_utils::count_tokens;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocateItem {
    pub file_path: String,
    pub reason: String,
}


async fn get_max_tokens(
    ccx: Arc<AMutex<AtCommandsContext>>,
) -> Result<usize, String> {
    let gcx = ccx.lock().await.global_context.clone();
    let caps = crate::global_context::try_load_caps_quickly_if_not_present(
        gcx.clone(), 0,
    )
        .await
        .map_err(|e| {
            warn!("no caps: {:?}", e);
            "network error communicating with the model (1)".to_string()
        })?;

    let x = match caps.read().unwrap().code_chat_models.get(
        DEFAULT_MODEL_NAME
    ) {
        Some(res) => Ok(res.n_ctx),
        None => Err(format!(
            "the default patch model {} is not available in the caps",
            &DEFAULT_MODEL_NAME
        ))
    };
    x
}

async fn load_tokenizer(
    ccx: Arc<AMutex<AtCommandsContext>>
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
        caps.clone(), gcx.clone(), DEFAULT_MODEL_NAME.to_string(),
    ).await
}


async fn get_locate_data(
    ccx: Arc<AMutex<AtCommandsContext>>,
) -> Option<Vec<LocateItem>> {
    let messages = ccx.lock().await.messages.clone();
    let mut locate_tool_ids = vec![];
    for message in messages.iter().rev() {
        for tools in message.tool_calls.iter() {
            for tool in tools.iter() {
                if tool.function.name == "locate" {
                    locate_tool_ids.push(tool.id.clone());
                }
            }
        }
    }

    let mut locate_data = vec![];
    for id in locate_tool_ids.iter() {
        let data = match messages.iter().find_or_first(|x| x.tool_call_id == *id) {
            Some(data) => {
                let locate_items: Vec<LocateItem> = match serde_json::from_str(&data.content) {
                    Ok(res) => res,
                    Err(err) => {
                        warn!("failed to parse locate data: {}", err);
                        continue;
                    }
                };
                locate_items
            }
            None => {
                continue
            }
        };
        locate_data.push(data);
    }
    locate_data.first().cloned()
}

async fn make_chat_history(
    ccx: Arc<AMutex<AtCommandsContext>>,
    args: &PatchArguments,
) -> Result<Vec<ChatMessage>, String> {
    let tokenizer = match load_tokenizer(ccx.clone()).await {
        Ok(t) => t,
        Err(e) => return Err(e),
    };
    let max_tokens = match get_max_tokens(ccx.clone()).await {
        Ok(n) => n,
        Err(e) => return Err(e),
    };

    let gcx = ccx.lock().await.global_context.clone();
    let system_prompt = DefaultToolPatch::prompt();
    // TODO: use budget for extra context construction
    let maybe_extra_context = if let Some(symbols_names) = args.symbol_names.clone() {
        get_signatures_by_symbol_names(&symbols_names, gcx.clone()).await
    } else {
        get_signatures_by_imports_traversal(&args.paths, gcx.clone()).await
    };
    let mut tokens: usize = 0;
    let max_tokens: usize = max_tokens - MAX_NEW_TOKENS;
    let tokenizer_ref = tokenizer.read().unwrap().clone();
    let task_message = format!("The task is:\n{}", args.todo).to_string();
    let mut chat_messages = vec![
        ChatMessage::new(
            "system".to_string(),
            system_prompt.to_string(),
        )
    ];
    tokens += 3 + count_tokens(&tokenizer_ref, &system_prompt);
    tokens += 3 + count_tokens(&tokenizer_ref, &task_message);
    if tokens > max_tokens {
        return Err(format!("too many tokens: {tokens} > {max_tokens}"));
    }

    let has_single_file = args.paths.len() == 1;
    for (idx, file) in args.paths.iter().enumerate() {
        match execute_at_file(ccx.clone(), file.clone()).await {
            Ok(res) => {
                let message = format!("{}\n```\n{}\n```", res.file_name, res.file_content).to_string();
                tokens += 3 + count_tokens(&tokenizer_ref, &message);
                if tokens > max_tokens {
                    let err_message = if has_single_file || idx == 0 {
                        format!("the provided file {file} is too large for the patch tool: {tokens} > {max_tokens}")
                    } else {
                        format!("too many files are provided: {tokens} ctx > {max_tokens} max available ctx, use the tool for each file separately")
                    };
                    return Err(err_message);
                }
                chat_messages.push(ChatMessage::new("user".to_string(), message));
            }
            Err(_) => {
                let message = format!(
                    "{}\n<{}>",
                    file,
                    "Cannot find given file on the disk, probably it's intended to be added"
                ).to_string();
                tokens += 3 + count_tokens(&tokenizer_ref, &message);
                chat_messages.push(ChatMessage::new("user".to_string(), message));
            }
        }
    }
    if let Some(extra_context) = maybe_extra_context {
        let message = format!("Extra context for the files:\n{}", extra_context).to_string();
        tokens += 3 + count_tokens(&tokenizer_ref, &message);
        if tokens > max_tokens {
            warn!("Too many tokens for the extra context, skipping it: {tokens} > {max_tokens}");
        } else {
            chat_messages.push(ChatMessage::new("user".to_string(), message));
        }
    }

    chat_messages.push(ChatMessage::new("user".to_string(), task_message));
    info!("tokens num: {tokens}");
    Ok(chat_messages)
}

pub async fn execute_chat_model(
    ccx: Arc<AMutex<AtCommandsContext>>,
    tool_call_id: &String,
    args: &PatchArguments,
) -> Result<Vec<String>, String> {
    let messages = make_chat_history(ccx.clone(), args).await?;
    let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let response = subchat_single(
        ccx.clone(),
        &DEFAULT_MODEL_NAME,
        messages,
        vec![],
        None,
        false,
        Some(TEMPERATURE),
        Some(MAX_NEW_TOKENS),
        N_CHOICES,
        Some(format!("{log_prefix}-patch")),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-patch")),
    ).await;

    match response {
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
    }
}