use std::collections::HashMap;
use regex::Regex;

use crate::at_commands::at_commands::{AtCommandCall, AtCommandsContext};
use crate::call_validation::{ChatMessage, ContextFile, ContextTool};


async fn correct_call_if_needed(
    call: &mut AtCommandCall,
    highlights_local: &mut Vec<AtCommandHighlight>,
    context: &AtCommandsContext,
    command_names: &Vec<String>,
) {
    let params = call.command.lock().await.params().iter().cloned().collect::<Vec<_>>();
    if params.len() != call.args.len() {
        highlights_local.iter_mut().for_each(|h| {
            h.ok = false; h.reason = Some("incorrect number of arguments".to_string());
        });
        return;
    }

    let mut corrected = vec![];
    for ((param, arg), h) in params.iter().zip(call.args.iter()).zip(highlights_local.iter_mut()) {
        if command_names.contains(arg) {
            h.ok = false; h.reason = Some("incorrect argument; is a command name".to_string());
            return;
        }
        let param = param.lock().await;
        if param.is_value_valid(arg, context).await {
            corrected.push(arg.clone());
            continue;
        }
        let completion = match param.complete(arg, context, 1).await.get(0) {
            Some(x) => x.clone(),
            None => {
                h.ok = false; h.reason = Some("incorrect argument; failed to complete".to_string());
                return;
            }
        };
        if !param.is_value_valid(&completion, context).await {
            h.ok = false; h.reason = Some("incorrect argument; completion did not help".to_string());
            return;
        }
        corrected.push(completion);
    }
    call.args = corrected;
}

async fn execute_at_commands_from_query_line(
    line_n: usize,
    line: &mut String,
    query: &String,
    context: &AtCommandsContext,
    remove_valid_from_query: bool,
    msgs: &mut Vec<ContextFile>,
    highlights: &mut Vec<AtCommandHighlight>,
    top_n: usize,
) -> bool {
    let pos1_start = {
        let mut lines = query.lines().collect::<Vec<_>>();
        lines.truncate(line_n);

        let mut pos1_start = 0;
        for line_before in  lines {
            pos1_start += line_before.len() + 1;
        }
        pos1_start
    };
    
    let at_command_names = context.at_commands.keys().map(|x|x.clone()).collect::<Vec<_>>();
    let line_words = parse_words_from_line(line);
    let mut line_words_cloned = line_words.iter().map(|(x, _, _)|x.clone()).collect::<Vec<_>>();
    let mut another_pass_needed = false;
    
    for (w_idx, (word, pos1, pos2)) in line_words.iter().enumerate() {
        if let Some(cmd) = context.at_commands.get(word) {
            let mut call = AtCommandCall::new(cmd.clone(), vec![]);
            let mut highlights_local = vec![];

            let cmd_params_cnt = cmd.lock().await.params().len();
            let mut q_cmd_args = line_words.iter().skip(w_idx + 1).collect::<Vec<_>>();
            q_cmd_args.truncate(cmd_params_cnt);
            call.args = q_cmd_args.iter().map(|(text, _, _)|text.clone()).collect();

            highlights_local.push(AtCommandHighlight::new("cmd".to_string(), line_n, w_idx, pos1_start + *pos1, pos1_start + *pos2));
            
            for (i, (_, pos1, pos2)) in q_cmd_args.iter().enumerate().map(|(i, arg)| (i + 1, arg)) {
                highlights_local.push(AtCommandHighlight::new("arg".to_string(), line_n, w_idx + i, pos1_start + *pos1, pos1_start + *pos2));
            }
            
            correct_call_if_needed(&mut call, &mut highlights_local, context, &at_command_names).await;

            let mut executed = false;
            let mut text_on_clip = String::new();
            if highlights_local.iter().all(|x|x.ok) {
                match call.command.lock().await.execute(query, &call.args, top_n, context).await {
                    Ok((m_res, m_text_on_clip)) =>
                        { 
                            executed = true; 
                            text_on_clip = m_text_on_clip;
                            msgs.extend(m_res) 
                        },
                    Err(e) => {
                        tracing::warn!("can't execute command that indicated it can execute: {}", e);
                    }
                }
            }

            // not on preview
            if remove_valid_from_query {
                if executed {
                    let mut indices_to_remove = vec![];
                    for h in highlights_local.iter() {
                        indices_to_remove.push(h.word_n);
                    }
                    line_words_cloned.insert(*indices_to_remove.iter().max().unwrap_or(&0usize) + 1, text_on_clip);
                    for i in indices_to_remove.iter().rev() {
                        line_words_cloned.remove(*i);
                    }
                    *line = line_words_cloned.join(" ");
                    // need to extract second time because indexes were shifted
                    another_pass_needed = true;
                }
            }

            highlights.extend(highlights_local);

            if another_pass_needed {
                break;
            }
        }
    }
    another_pass_needed
}

pub async fn execute_at_commands_in_query(
    query: &mut String,
    context: &AtCommandsContext,
    remove_valid_from_query: bool,
    top_n: usize,
) -> (Vec<ContextTool>, Vec<AtCommandHighlight>) {
    let mut msgs = vec![];
    let mut highlights = vec![];
    let mut new_lines = vec![];
    
    for (idx, mut line) in query.lines().map(|x|x.to_string()).enumerate() {
        loop {
            let another_pass_needed = execute_at_commands_from_query_line(
                idx, &mut line, query, context, remove_valid_from_query, &mut msgs, &mut highlights, top_n
            ).await;

            if !another_pass_needed {
                new_lines.push(line);
                break;
            }
        }
    }
    *query = new_lines.join("\n");
    (msgs.into_iter().map(|x|ContextTool::ContextFile(x)).collect::<Vec<_>>(), highlights)
}

pub async fn execute_at_commands_from_msg(
    msg: &ChatMessage,
    context: &AtCommandsContext,
    top_n: usize,
) -> Result<Vec<ContextTool>, String> {
    let at_command_names = context.at_commands.keys().map(|x|x.clone()).collect::<Vec<_>>();
    let mut msgs = vec![];
    if let Some(ref tool_calls) = msg.tool_calls {
        for t_call in tool_calls {
            if let Some(cmd) = context.at_commands.get(&format!("@{}", t_call.function.name)) {
                let mut call = AtCommandCall::new(cmd.clone(), vec![]);
                let args: HashMap<String, String> = serde_json::from_str(&t_call.function.arguments).map_err(|e| format!("couldn't parse args: {:?}", e))?;
                let args_values = args.iter().map(|(_, v)|v.clone()).collect::<Vec<_>>();
                call.args = args_values;
                
                let mut highlights = vec![];
                highlights.push(AtCommandHighlight::new("cmd".to_string(), 0, 0, 0, 0));
                for _ in call.args.iter(){
                    highlights.push(AtCommandHighlight::new("arg".to_string(), 0, 0, 0, 0));
                }

                correct_call_if_needed(&mut call, &mut highlights, context, &at_command_names).await;

                if highlights.iter().all(|x|x.ok) {
                    match call.command.lock().await.execute(&msg.content, &call.args, top_n, context).await {
                        Ok((m_res, _)) => msgs.extend(m_res),
                        Err(e) => {
                            tracing::warn!("can't execute command that indicated it can execute: {}", e);
                        }
                    }
                }
            }
        }
    }
    Ok(msgs.into_iter().map(|x|ContextTool::ContextFile(x)).collect())
}

#[derive(Debug)]
pub struct AtCommandHighlight {
    pub kind: String,
    pub line_n: usize,
    pub word_n: usize,
    pub pos1: usize,
    pub pos2: usize,
    pub ok: bool,
    pub reason: Option<String>,
}

impl AtCommandHighlight {
    pub fn new(kind: String, line_n: usize, word_n: usize, pos1: usize, pos2: usize) -> Self {
        Self { kind, line_n, word_n, pos1, pos2, ok: true, reason: None}
    }
}

pub fn parse_words_from_line(line: &String) -> Vec<(String, usize, usize)> {
    // TODO: make regex better
    let word_regex = Regex::new(r#"(@?[^ !?@]*)"#).expect("Invalid regex");
    let mut results = vec![];
    for cap in word_regex.captures_iter(line) {
        if let Some(matched) = cap.get(1) {
            results.push((matched.as_str().to_string(), matched.start(), matched.end()));
        }
    }
    results
}
