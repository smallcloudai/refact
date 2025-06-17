use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::ChatMessage;
use crate::custom_error::MapErrToString;
use crate::global_context::GlobalContext;
use crate::json_utils;

#[derive(Deserialize, Clone)]
pub struct FollowUpResponse {
    pub follow_ups: Vec<String>,
    pub topic_changed: bool,
}

fn _make_conversation(
    messages: &Vec<ChatMessage>
) -> Vec<ChatMessage> {
    let mut history_message = "# Conversation\n".to_string();
    for m in messages.iter().rev().take(2) {
        let content = m.content.content_text_only();
        let limited_content = if content.chars().count() > 5000 {
            let skip_count = content.chars().count() - 5000;
            format!("...{}", content.chars().skip(skip_count).collect::<String>())
        } else {
            content
        };
        let message_row = match m.role.as_str() {
            "user" => {
                format!("👤:{}\n\n", limited_content)
            }
            "assistant" => {
                format!("🤖:{}\n\n", limited_content)
            }
            _ => {
                continue;
            }
        };
        history_message.insert_str(0, &message_row);
    }
    vec![
        ChatMessage::new("user".to_string(), history_message),
    ]
}

pub async fn generate_follow_up_message(
    messages: Vec<ChatMessage>,
    gcx: Arc<ARwLock<GlobalContext>>,
    model_id: &str,
    chat_id: &str,
) -> Result<FollowUpResponse, String> {
    let ccx = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        32000,
        1,
        false,
        messages.clone(),
        chat_id.to_string(),
        false,
        Some(model_id.to_string()),
    ).await));
    let new_messages = crate::cloud::subchat::subchat(
        ccx.clone(),
        "id:generate_follow_up_message:1.0",
        _make_conversation(&messages),
        Some(0.0),
        Some(512),
        None
    ).await?;
    let content = new_messages
        .into_iter()
        .last()
        .map(|last_m| last_m.content.content_text_only())
        .ok_or("No message have been found".to_string())?;
    let response: FollowUpResponse = json_utils::extract_json_object(&content)
        .map_err_with_prefix("Failed to parse json:")?;
    Ok(response)
}
