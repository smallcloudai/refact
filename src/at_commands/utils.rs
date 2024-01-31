use serde_json::{json, Value};
use crate::at_commands::structs::{AtCommand, AtCommandCall, AtCommandsContext};
use tracing::info;


pub fn compose_context_file_msg_from_result(
    in_json: &Value,
) -> Value {
    return json!({
        "choices": [{
            "delta": {
                "content": in_json.clone(),
                "role": "context_file"
            },
            "finish_reason": null,
            "index": 0
        }],
    });
}

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
        if !cmd.lock().await.can_execute(&q_cmd_args, context).await {
            info!("command {:?} is not executable with arguments {:?}", q_cmd, q_cmd_args);
            continue;
        }
        info!("command {:?} is perfectly good", q_cmd);
        results.push(AtCommandCall::new(cmd.clone(), q_cmd_args.clone()));
        valid_command_lines.push(idx);
    }
    // remove the lines that are valid commands from query
    *query = query.lines().enumerate()
        .filter(|(idx, _line)| !valid_command_lines.contains(idx))
        .map(|(_idx, line)| line)
        .collect::<Vec<_>>().join("\n");
    results
}
