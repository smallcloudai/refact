use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tracing::info;

use crate::at_commands::at_commands::{AtCommandCall, AtCommandsContext, AtParam};
use crate::files_in_workspace::{DocumentInfo, pathbuf_to_url};
use crate::global_context::GlobalContext;

pub async fn find_valid_at_commands_in_query(
    query: &mut String,
    context: &AtCommandsContext,
) -> Vec<AtCommandCall> {
    let mut results = vec![];
    let mut valid_command_lines = vec![];
    for (idx, line) in query.lines().enumerate() {
        let line_words: Vec<&str> = line.split_whitespace().collect();
        let q_cmd_args = line_words.iter().skip(1).map(|x| x.to_string()).collect::<Vec<String>>();

        let q_cmd = match line_words.first() {
            Some(x) => x,
            None => continue,
        };

        let (_, cmd) = match context.at_commands.iter().find(|&(k, _v)| k == q_cmd) {
            Some(x) => x,
            None => continue,
        };
        let can_execute = cmd.lock().await.can_execute(&q_cmd_args, context).await;
        let q_cmd_args = match correct_arguments_if_needed(cmd.lock().await.params(), &q_cmd_args, can_execute, context).await {
            Ok(x) => x,
            Err(e) => {
                info!("command {:?} is not executable with arguments {:?}; error: {:?}", q_cmd, q_cmd_args, e);
                continue;
            }
        };

        info!("command {:?} is perfectly good", q_cmd);
        results.push(AtCommandCall::new(Arc::clone(&cmd), q_cmd_args.clone()));
        valid_command_lines.push(idx);
    }
    // remove the lines that are valid commands from query
    *query = query.lines().enumerate()
        .filter(|(idx, _line)| !valid_command_lines.contains(idx))
        .map(|(_idx, line)| line)
        .collect::<Vec<_>>().join("\n");
    results
}

pub async fn correct_arguments_if_needed(
    params: &Vec<Arc<AMutex<dyn AtParam>>>,
    args: &Vec<String>,
    can_execute: bool,
    context: &AtCommandsContext,
) -> Result<Vec<String>, String> {
    if can_execute {
        return Ok(args.clone());
    }
    if params.len() != args.len() {
        return Err(format!("incorrect number of arguments: {} given; {} required", args.len(), params.len()));
    }
    let mut args_new = vec![];
    for (param, arg) in params.iter().zip(args.iter()) {
        let param = param.lock().await;
        if param.is_value_valid(arg, context).await {
            args_new.push(arg.clone());
            continue;
        }
        let completion = param.complete(arg, context, 1).await;
        let arg_completed = match completion.get(0) {
            Some(x) => x,
            None => return Err(format!("arg '{}' is not valid and correction failed", arg)),
        };
        if !param.is_value_valid(arg_completed, context).await {
            return Err(format!("arg '{}' is not valid and correction failed", arg));
        }
        info!("arg '{}' is corrected as '{}'", arg, arg_completed);
        args_new.push(arg_completed.clone());
    }
    Ok(args_new)
}


pub async fn get_file_text_from_vecdb(global_context: Arc<ARwLock<GlobalContext>>, file_path: &String) -> Result<String, String> {
    let cx = global_context.read().await;

    // if you write pathbuf_to_url(&PathBuf::from(file_path)) without unwrapping it gives: future cannot be sent between threads safe
    let url_mb = pathbuf_to_url(&PathBuf::from(file_path)).map(|x| Some(x)).unwrap_or(None);
    if let Some(url) = url_mb {
        let document_mb = cx.documents_state.document_map.read().await.get(&url).cloned();
        if document_mb.is_some() {
            return Ok(document_mb.unwrap().text.to_string());
        }
    }

    return match *cx.vec_db.lock().await {
        Some(ref db) => Ok(db.get_file_orig_text(file_path.clone()).await.file_text),
        None => Err("vecdb is not available && no file in memory was found".to_string())
    };
}


pub async fn get_file_text_from_disk(global_context: Arc<ARwLock<GlobalContext>>, file_path: &String) -> Result<String, String> {
    let cx = global_context.read().await;

    // if you write pathbuf_to_url(&PathBuf::from(file_path)) without unwrapping it gives: future cannot be sent between threads safe
    let url_mb = pathbuf_to_url(&PathBuf::from(file_path)).map(|x| Some(x)).unwrap_or(None);
    if let Some(url) = url_mb {
        let document_mb = cx.documents_state.document_map.read().await.get(&url).cloned();
        if document_mb.is_some() {
            return Ok(document_mb.unwrap().text.to_string());
        }
    }

    let doc_info = match DocumentInfo::from_pathbuf(&PathBuf::from(file_path)) {
        Ok(doc) => doc.read_file().await,
        Err(_) => {
            return Err(format!("cannot parse filepath: {file_path}"))
        }
    };
    match doc_info {
        Ok(text) => Ok(text),
        Err(err) => Err(err.to_string())
    }
}
