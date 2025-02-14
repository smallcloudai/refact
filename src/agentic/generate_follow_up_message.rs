use std::sync::Arc;
use serde::Deserialize;
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex};

use crate::global_context::GlobalContext;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::subchat::subchat_single;
use crate::call_validation::{ChatContent, ChatMessage};

const PROMPT: &str = r#"
Super simple job: generate follow-up messages and determine if the topic has drastically changed!

**Input:**  
You receive a conversation between a user and an assistant. Your task is twofold:  
1. Create up to three distinct, concise follow-up messages that the user might send in response to the robot’s (assistant’s) last message. Each follow-up should be only a few words long and clearly meaningful.  
2. Decide whether the user's message represents a drastic change of topic in the conversation.

**Requirements:**  
- **Follow-ups:**  
  1. Generate up to 3 follow-up messages.  
  2. The first follow-up should encourage the robot to continue the conversation.  
  3. Each follow-up must convey a different meaning (avoid three variations of a simple “yes”).  
  4. If no straightforward follow-up is possible or if the conversation does not include a question, return an empty list for follow-ups.

- **Topic Change:**  
  - Evaluate if the user's latest message indicates a drastic change of topic compared to the previous conversation.
  
**Output:**  
Return your results in the following JSON format:  
```
{
  "follow_ups": ["Follow up 1", "Follow up 2"],
  "topic_changed": true
}
```  
Here, `topic_changed` should be `true` if the conversation topic has drastically changed, or `false` otherwise.

Do not include any backticks or extra formatting in your output.
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
    for m in messages.iter().rev() {
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
        16000,
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
