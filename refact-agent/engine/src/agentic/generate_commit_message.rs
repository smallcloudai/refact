use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage};
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;

const N_CTX: usize = 32000;
const TEMPERATURE: f32 = 0.2;

pub fn remove_fencing(message: &String) -> Vec<String> {
    let trimmed_message = message.trim();
    if !trimmed_message.contains("```") {
        return Vec::new();
    }
    if trimmed_message.contains("``````") {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut in_code_block = false;

    for (_i, part) in trimmed_message.split("```").enumerate() {
        if in_code_block {
            let part_lines: Vec<&str> = part.lines().collect();
            if !part_lines.is_empty() {
                let start_idx = if part_lines[0].trim().split_whitespace().count() <= 1 && part_lines.len() > 1 {
                    1
                } else {
                    0
                };
                if start_idx < part_lines.len() {
                    let code_block = part_lines[start_idx..].join("\n");
                    if !code_block.is_empty() {
                        results.push(code_block.trim().to_string());
                    }
                }
            }
        }

        in_code_block = !in_code_block;
    }
    if !results.is_empty() {
        results
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_fencing() {
        let input = "Simple text without fencing".to_string();
        assert_eq!(remove_fencing(&input), Vec::<String>::new());
    }

    #[test]
    fn test_simple_fencing() {
        let input = "```\nCode block\n```".to_string();
        assert_eq!(remove_fencing(&input), vec!["Code block".to_string()]);
    }

    #[test]
    fn test_language_tag() {
        let input = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```".to_string();
        assert_eq!(remove_fencing(&input), vec!["fn main() {\n    println!(\"Hello\");\n}".to_string()]);
    }

    #[test]
    fn test_text_before_and_after() {
        let input = "Text before\nText before\n```\nCode block\n```\nText after".to_string();
        assert_eq!(remove_fencing(&input), vec!["Code block".to_string()]);
    }

    #[test]
    fn test_multiple_code_blocks() {
        let input = "First paragraph\n```\nFirst code\n```\nMiddle text\n```python\ndef hello():\n    print('world')\n```\nLast paragraph".to_string();
        assert_eq!(remove_fencing(&input), vec!["First code".to_string(), "def hello():\n    print('world')".to_string()]);
    }

    #[test]
    fn test_empty_code_block() {
        let input = "Text with `````` empty block".to_string();
        assert_eq!(remove_fencing(&input), Vec::<String>::new());
    }
}

pub async fn generate_commit_message_by_diff(
    gcx: Arc<ARwLock<GlobalContext>>,
    diff: &String,
    commit_message_prompt: &Option<String>,
) -> Result<String, String> {
    if diff.is_empty() {
        return Err("The provided diff is empty".to_string());
    }
    let (messages, expert_name) = if let Some(text) = commit_message_prompt {
        (vec![
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::SimpleText(format!(
                    "Initial commit message:\n```\n{}\n```\nDiff:\n```\n{}\n```\n",
                    text, diff
                )),
                ..Default::default()
            },
        ], "generate_commit_message_with_prompt:1.0")
    } else {
        (vec![
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::SimpleText(format!("Diff:\n```\n{}\n```\n", diff)),
                ..Default::default()
            },
        ], "generate_commit_message:1.0")
    };
    let model_id = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => Ok(caps.defaults.chat_default_model.clone()),
        Err(_) => Err("No caps available".to_string()),
    }?;
    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        N_CTX,
        1,
        false,
        messages.clone(),
        "".to_string(),
        false,
        Some(model_id.clone()),
    ).await));
    
    let new_messages = crate::cloud::subchat::subchat(
        ccx.clone(),
        &model_id,
        expert_name,
        messages,
        Some(TEMPERATURE),
        Some(2048),
        None,
    ).await?;
    let content = new_messages
        .into_iter()
        .last()
        .map(|last_m| last_m.content.content_text_only())
        .ok_or("No message have been found".to_string())?;
    let code_blocks = remove_fencing(&content);
    if !code_blocks.is_empty() {
        Ok(code_blocks[0].clone())
    } else {
        Ok(content)
    }
}
