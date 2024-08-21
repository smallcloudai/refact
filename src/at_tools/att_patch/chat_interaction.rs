use std::collections::HashMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokenizers::Tokenizer;
use tokio::sync::Mutex as AMutex;
use tracing::{info, warn};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::{at_file_repair_candidates, context_file_from_file_path};
use crate::at_tools::att_patch::args_parser::PatchArguments;
use crate::at_tools::att_patch::tool::{DefaultToolPatch, N_CHOICES};
use crate::at_tools::att_patch::ast_interaction::{get_signatures_by_imports_traversal };
use crate::at_tools::subchat::subchat_single;
use crate::cached_tokenizers;
use crate::call_validation::{ChatMessage, ChatUsage};
use crate::caps::get_model_record;
use crate::call_validation::{ChatMessage, ChatToolCall, ChatToolFunction, ChatUsage, ContextFile};
use crate::scratchpads::pp_utils::count_tokens;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocateItem {
    pub file_path: String,
    pub reason: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocateData {
    pub files: Vec<LocateItem>,
    pub symbols: Vec<String>,
}

async fn read_file(
    ccx: Arc<AMutex<AtCommandsContext>>,
    file_path: String,
) -> Option<ContextFile> {
    let candidates = at_file_repair_candidates(ccx.clone(), &file_path, false).await;
    match context_file_from_file_path(ccx.clone(), candidates, file_path.clone()).await {
        Ok(x) => Some(x),
        Err(e) => None
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

async fn get_locate_data(
    ccx: Arc<AMutex<AtCommandsContext>>,
) -> Option<LocateData> {
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
                let locate_data: LocateData = match serde_json::from_str(&data.content) {
                    Ok(res) => res,
                    Err(err) => {
                        warn!("failed to parse locate data: {}", err);
                        continue;
                    }
                };
                locate_data
            }
            None => {
                continue
            }
        };
        locate_data.push(data);
    }
    locate_data.first().cloned()
}

async fn create_extra_context(
    ccx: Arc<AMutex<AtCommandsContext>>,
    paths: Vec<(String, Option<String>)>,
    extra_paths_mb: Option<Vec<String>>,
    extra_symbols_mb: Option<Vec<String>>,
    messages: &Vec<ChatMessage>,
    tool_call_id: &String,
    usage: &mut ChatUsage,
) -> Result<Vec<ChatMessage>, String> {
    let gcx = ccx.lock().await.global_context.clone();
    let mut new_messages = messages.clone();
    let cat_args = if extra_paths_mb.is_some() && extra_symbols_mb.is_some() {
        let mut cat_args = HashMap::new();
        cat_args.insert("paths".to_string(), extra_paths_mb
            .expect("checked above")
            .join(",")
        );
        if let Some(extra_symbols) = extra_symbols_mb {
            if !extra_symbols.is_empty() {
                cat_args.insert("symbols".to_string(), extra_symbols.join(","));
            }
        }
        cat_args.insert("skeleton".to_string(), "true".to_string());
        Some(cat_args)
    } else {
        if let Some(paths) = get_signatures_by_imports_traversal(
            &paths.iter().map(|x| x.0.clone()).collect(), gcx.clone()
        ).await {
            let mut cat_args = HashMap::new();
            cat_args.insert("paths".to_string(), paths
                .iter()
                .map(|x| x.to_string_lossy())
                .join(",")
            );
            cat_args.insert("skeleton".to_string(), "true".to_string());
            Some(cat_args)
        } else {
            None
        }
    };
    if let Some(cat_args) = cat_args {
        new_messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: "".to_string(),
            tool_calls: Some(vec![ChatToolCall {
                id: "patch_cat_42".to_string(),
                function: ChatToolFunction {
                    arguments: serde_json::to_string(&cat_args).unwrap().to_string(),
                    name: "cat".to_string()
                },
                tool_type: "function".to_string(),
            }]),
            tool_call_id: "".to_string(),
            ..Default::default()
        });
        let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let messages = subchat_single(
            ccx.clone(),
            &DEFAULT_MODEL_NAME,
            new_messages,
            vec!["cat".to_string()],
            None,
            true,
            Some(TEMPERATURE),
            Some(MAX_NEW_TOKENS),
            N_CHOICES,
            Some(usage),
            Some(format!("{log_prefix}-patch")),
            Some(tool_call_id.clone()),
            Some(format!("{log_prefix}-patch")),
        )
            .await?
            .get(0)
            .ok_or("relevant_files: tree deterministic message was empty. Try again later".to_string())?
            .clone();
        Ok(messages)
    } else {
        warn!("no extra context for the patch is using");
        Ok(new_messages)
    }
}

async fn make_chat_history(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model: &str,
    max_tokens: usize,
    max_new_tokens: usize,
    args: &PatchArguments,
    tool_call_id: &String,
    usage: &mut ChatUsage,
) -> Result<Vec<ChatMessage>, String> {
    let tokenizer = match load_tokenizer(ccx.clone(), model).await {
        Ok(t) => t,
        Err(e) => return Err(e),
    };

    let mut tokens: usize = 0;
    let max_tokens: usize = max_tokens.saturating_sub(max_new_tokens);
    let system_prompt = format_diff_prompt(ccx.clone()).await;
    let tokenizer_ref = tokenizer.read().unwrap().clone();
    let task_message = args.todo.clone();
    let mut chat_messages = vec![
        ChatMessage::new(
            "system".to_string(),
            system_prompt.to_string(),
        )
    ];
    tokens += 3 + count_tokens(&tokenizer_ref, &system_prompt);
    tokens += 3 + count_tokens(&tokenizer_ref, &task_message);
    if tokens > max_tokens {
        return Err(format!("too many tokens for the todo message: {tokens} > {max_tokens}, reduce the todo message length"));
    }
    
    let (paths, extra_paths_mb, extra_symbols_mb) = {
        if args.use_locate_for_context {
            if let Some(locate_data) = get_locate_data(ccx.clone()).await {
                let paths = locate_data.files.iter()
                    .filter(|x| x.reason.to_lowercase() == "to_change")
                    .map(|x| (x.file_path.clone(), Some(x.description.clone())))
                    .collect::<Vec<_>>();
                let extra_paths = locate_data.files.iter()
                    .filter(|x| x.reason.to_lowercase() != "to_change")
                    .map(|x| x.file_path.clone())
                    .collect::<Vec<_>>();
                (paths, Some(extra_paths), Some(locate_data.symbols))
            } else {
                return Err("locate data has not found though it was asked to use, call `locate` tool or pass the exact filenames".to_string());
            }
        } else {
            (
                args.paths.iter().map(|x| (x.clone(), None)).collect::<Vec<_>>(), 
                None,
                None
            )
        }
    };
    
    let has_single_file = paths.len() == 1;
    for (idx, (file, description_mb)) in paths.iter().enumerate() {
        match read_file(ccx.clone(), file.clone()).await {
            Some(res) => {
                let message = if let Some(description) = description_mb {
                    format!(
                        "File to modify: {}\nDescription: {}\nContent:\n```\n{}\n```",
                        res.file_name,
                        description,
                        res.file_content
                    ).to_string()
                } else {
                    format!(
                        "File to modify: {}\nContent:\n```\n{}\n```",
                        res.file_name,
                        res.file_content
                    ).to_string()
                };
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
            None => {
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

    let mut chat_messages = create_extra_context(
        ccx.clone(), paths, extra_paths_mb, extra_symbols_mb, &mut chat_messages, tool_call_id, usage
    ).await?
        .iter()
        .map(|x| if x.role != "tool" { x.clone() } else { 
            let mut x = x.clone();
            x.content = "Files for extra context (do not modify them!):".to_string();
            x
        })
        .collect::<Vec<_>>();
    
    chat_messages.push(ChatMessage::new("user".to_string(), task_message));


    Ok(chat_messages)
}

pub async fn execute_chat_model(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model: &str,
    max_tokens: usize,
    temperature: Option<f32>,
    max_new_tokens: usize,
    tool_call_id: &String,
    args: &PatchArguments,
    usage: &mut ChatUsage,
) -> Result<Vec<String>, String> {
    let messages = make_chat_history(
        ccx.clone(), model, max_tokens, max_new_tokens, args, tool_call_id, usage
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