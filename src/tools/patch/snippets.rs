use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tracing::warn;
use crate::at_commands::at_commands::AtCommandsContext;
use tokio::sync::Mutex as AMutex;

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub enum PatchAction {
    #[default]
    PartialEdit,
    FullRewrite,
    NewFile,
    Other,
}

impl PatchAction {
    pub fn from_string(action: &str) -> Result<PatchAction, String> {
        match action {
            "📍PARTIAL_EDIT" => Ok(PatchAction::PartialEdit),
            "📍FULL_REWRITE" => Ok(PatchAction::FullRewrite),
            "📍NEW_FILE" => Ok(PatchAction::NewFile),
            "📍OTHER" => Ok(PatchAction::Other),
            _ => Err(format!("invalid action: {}", action)),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct CodeSnippet {
    pub action: PatchAction,
    pub ticket: String,
    pub filename_before: String,
    pub filename_after: String,
    pub code: String,
}

fn parse_snippets(content: &str) -> Vec<CodeSnippet> {
    fn process_snippet(lines: &[&str], line_num: usize) -> Result<(usize, CodeSnippet), String> {
        let mut snippet = CodeSnippet::default();
        let command_line = lines[line_num];
        let info_elements = command_line.trim().split(" ").collect::<Vec<&str>>();
        if info_elements.len() < 3 {
            return Err("failed to parse snippet, invalid command line: {}".to_string());
        }
        snippet.action = match PatchAction::from_string(info_elements[0]) {
            Ok(a) => a,
            Err(e) => {
                return Err(format!("failed to parse snippet, {e}"));
            }
        };
        snippet.ticket = info_elements[1].to_string();
        snippet.filename_before = info_elements[2].to_string();

        if let Some(code_block_fence_line) = lines.get(line_num + 1) {
            if !code_block_fence_line.contains("```") {
                return Err("failed to parse snippet, invalid code block fence".to_string());
            }
            for (idx, line) in lines.iter().enumerate().skip(line_num + 2) {
                if line.contains("```") {
                    return Ok((2 + idx, snippet));
                }
                snippet.code.push_str(format!("{}\n", line).as_str());
            }
            Err("failed to parse snippet, no ending fence for the code block".to_string())
        } else {
            Err("failed to parse snippet, no code block".to_string())
        }
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut line_num = 0;
    let mut blocks: Vec<CodeSnippet> = vec![];
    while line_num < lines.len() {
        let line = lines[line_num];
        if line.contains("📍") {
            match process_snippet(&lines, line_num) {
                Ok((new_line_num, snippet)) => {
                    line_num = new_line_num;
                    blocks.push(snippet);
                }
                Err(err) => {
                    warn!("Skipping the snippet due to the error: {err}");
                    line_num += 1;
                    continue;
                }
            };
        } else {
            line_num += 1;
        }
    }
    blocks
}

pub async fn get_code_snippets(
    ccx: Arc<AMutex<AtCommandsContext>>,
) -> HashMap<String, CodeSnippet> {
    let messages = ccx.lock().await.messages.clone();
    let mut code_snippets: HashMap<String, CodeSnippet> = HashMap::new();
    for message in messages
        .iter()
        .filter(|x| x.role == "assistant") {
        for snippet in parse_snippets(&message.content).into_iter() {
            code_snippets.insert(snippet.ticket.clone(), snippet);
        }
    }
    code_snippets
}
