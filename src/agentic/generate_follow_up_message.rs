use std::sync::Arc;
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex};

use crate::global_context::GlobalContext;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::subchat::subchat_single;
use crate::call_validation::ChatMessage;

pub async fn generate_follow_up_message(
    mut messages: Vec<ChatMessage>, 
    gcx: Arc<ARwLock<GlobalContext>>, 
    model_name: &str, 
    chat_id: &str,
) -> Result<String, String> {
    if messages.first().map(|m| m.role == "system").unwrap_or(false) {
        messages.remove(0);
    }
    messages.insert(0, ChatMessage::new(
        "system".to_string(),
        "Generate a 2-3 word user response, like 'Can you fix it?' for errors or 'Proceed' for plan validation".to_string(),
    ));
    let ccx = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        1024,
        1,
        false,
        messages.clone(),
        chat_id.to_string(),
        false,
    ).await));
    let new_messages = subchat_single(
        ccx.clone(),
        model_name,
        messages,
        vec![],
        None,
        false,
        Some(0.5),
        None,
        1,
        None,
        None,
        None,
    ).await?;
    new_messages.into_iter().next().map(|x| x.into_iter().last().map(|last_m| {
        last_m.content.content_text_only() })).flatten().ok_or("No commit message found".to_string())
}