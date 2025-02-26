use std::sync::Arc;
use serde::Deserialize;
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex};

use crate::global_context::GlobalContext;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::subchat::subchat_single;
use crate::call_validation::{ChatContent, ChatMessage};

const PROMPT: &str = r#"
Your task is to do two things for a conversation between a user and an assistant:

1. **Follow-Up Messages:**
   - Create up to 5 super short follow-up messages that the user might send after the assistant's last message.
   - The first message should invite the assistant to keep talking.
   - Each message should have a different meaning.
   - If the assistant's last message contains a question, generate different replies that address that question.
   - Maybe include a suggestion to think deeper.
   - Maybe include a suggestion to explore more context (e.g., "Can we look at more files?", "Is there additional context I should provide?").
   - Maybe include a suggestion to remember/create knowledge (e.g., "Can you save this solution for future reference?", "Let's document this approach").
   - If there is no clear follow-up or the conversation isn't asking a question, return an empty list.

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
    for m in messages.iter().rev().take(10) {
        let message_row = match m.role.as_str() {
            "user" => {
                format!("👤:{}\n\n", &m.content.content_text_only())
            }
            "assistant" => {
                format!("🤖:{}\n\n", &m.content.content_text_only())
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
    light_model_name: Option<String>,
    current_model_name: &String,
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
    ).await));
    let model_name = light_model_name.unwrap_or(current_model_name.to_string());
    let updated_messages: Vec<Vec<ChatMessage>> = subchat_single(
        ccx.clone(),
        &model_name,
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

    let response: FollowUpResponse = serde_json::from_str(&response).map_err(|e| e.to_string())?;
    Ok(response)
}
