use std::sync::Arc;
use serde::Deserialize;
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex};

use crate::custom_error::MapErrToString;
use crate::global_context::GlobalContext;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::subchat::subchat_single;
use crate::call_validation::{ChatContent, ChatMessage};
use crate::json_utils;

const PROMPT: &str = r#"
Your task is to do two things for a conversation between a user and an assistant:

1. **Follow-Up Messages:**
   - Create up to 3 follow-up messages that the user might send after the assistant's last message.
   - Maximum 3 words each, preferably 1 or 2 words.
   - Each message should have a different meaning.
   - If the assistant's last message contains a question, generate different replies that address that question.
   - If there is no clear follow-up, return an empty list.
   - If assistant's work looks completed, return an empty list.
   - If there is nothing but garbage in the text you see, return an empty list.
   - If not sure, return an empty list.

2. **Topic Change Detection:**
   - Decide if the user's latest message is about a different topic or a different project or a different problem from the previous conversation.
   - A topic change means the new topic is not related to the previous discussion.

Return the result in this JSON format (without extra formatting):

{
  "follow_ups": ["Follow-up 1", "Follow-up 2", "Follow-up 3", "Follow-up 4", "Follow-up 5"],
  "topic_changed": true
}
"#;

#[derive(Deserialize, Clone)]
pub struct FollowUpResponse {
    pub follow_ups: Vec<String>,
    pub topic_changed: bool,
}

fn _make_conversation(
    messages: &Vec<ChatMessage>
) -> Vec<ChatMessage> {
    let mut history_message = "*Conversation:*\n".to_string();
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
        ChatMessage::new("system".to_string(), PROMPT.to_string()),
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
        model_id.to_string(),
    ).await));
    let updated_messages: Vec<Vec<ChatMessage>> = subchat_single(
        ccx.clone(),
        model_id,
        _make_conversation(&messages),
        Some(vec![]),
        None,
        false,
        Some(0.0),
        None,
        1,
        None,
        true,
        None,
        None,
        None,
    ).await?;
    let response = updated_messages
        .into_iter()
        .next()
        .map(|x| {
            x.into_iter().last().map(|last_m| match last_m.content {
                ChatContent::SimpleText(text) => Some(text),
                ChatContent::Multimodal(_) => None,
            })
        })
        .flatten()
        .flatten()
        .ok_or("No follow-up message was generated".to_string())?;

    tracing::info!("follow-up model says {:?}", response);

    let response: FollowUpResponse = json_utils::extract_json_object(&response)
        .map_err_with_prefix("Failed to parse json:")?;
    Ok(response)
}
