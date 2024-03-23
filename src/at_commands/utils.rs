use std::sync::Arc;

use tokio::sync::Mutex as AMutex;
use tracing::info;

use crate::at_commands::at_commands::{AtCommandCall, AtCommandsContext, AtParam};


pub async fn find_valid_at_commands_in_query(
    query: &mut String,
    context: &AtCommandsContext,
) -> Vec<AtCommandCall> {
    let mut results = vec![];
    let mut valid_command_lines = vec![];
    for (idx, line) in query.lines().enumerate() {
        let line_words: Vec<&str> = line.split_whitespace().collect();
        let q_cmd_args = line_words.iter().skip(1).map(|x|x.to_string()).collect::<Vec<String>>();

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
    // TODO: this function is a bad idea
    // if a parameter is correctable, it's already valid!
    // we only need to look up symbol once, not two or three times (is_valid, complete, is_valid again)
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
            None => return Err(format!("arg '{}' correction failed", arg)),
        };
        if !param.is_value_valid(arg_completed, context).await {
            return Err(format!("arg '{}' is not valid even after a typo correction attempt", arg_completed));
        }
        info!("arg '{}' is corrected as '{}'", arg, arg_completed);
        args_new.push(arg_completed.clone());
    }
    Ok(args_new)
}

pub fn split_file_into_chunks_from_line_inside(
    cursor_line: usize,
    file_lines: &mut Vec<String>,
    chunk_size_lines: usize
) -> (
    Vec<((usize, usize), String)>,
    Vec<((usize, usize), String)>
) {
    if cursor_line == file_lines.len() {  // otherwise the last block doesn't get added
        file_lines.push("".to_string());
    }

    // Line number start from 1 in ContextFile
    let cursor_line = cursor_line + 1;
    let (
        mut result_above, mut result_below, mut buffer_above, mut buffer_below
    ) = (
        Vec::new(), Vec::new(), Vec::new(), Vec::new()
    );

    for (idx, line) in file_lines.iter().enumerate() {
        let idx = idx + 2;
        if idx <= cursor_line {
            buffer_above.push(line.clone());
            if buffer_above.len() >= chunk_size_lines {
                result_above.push(((idx - buffer_above.len(), idx - 1), buffer_above.join("\n").to_string()));
                buffer_above.clear();
            }
        } else if idx > cursor_line {
            if !buffer_above.is_empty() {
                result_above.push(((idx - 1 - buffer_above.len(), idx - 2), buffer_above.join("\n")));
                buffer_above.clear();
            }

            buffer_below.push(line.clone());
            if buffer_below.len() >= chunk_size_lines {
                result_below.push(((idx - buffer_below.len(), idx - 1), buffer_below.join("\n")));
                buffer_below.clear();
            }
        }
    }

    if !buffer_below.is_empty() {
        result_below.push(((file_lines.len() - buffer_below.len() + 1, file_lines.len()), buffer_below.join("\n")));
    }

    (result_above, result_below)
}
