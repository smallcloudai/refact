use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ContextFile};
use crate::caps::strip_model_from_finetune;
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;

const TEMPERATURE: f32 = 0.2;


fn _make_prompt(
    previous_messages: &Vec<ChatMessage>,
) -> String {
    let mut context = "".to_string();
    for message in previous_messages.iter().rev() {
        let message_row = match message.role.as_str() {
            "user" => format!("👤:\n{}\n\n", &message.content.content_text_only()),
            "assistant" => format!("🤖:\n{}\n\n", &message.content.content_text_only()),
            "tool" => format!("🔨:\n{}\n\n", &message.content.content_text_only()),
            "context_file" => {
                let mut files = String::new();
                match serde_json::from_str::<Vec<ContextFile>>(&message.content.content_text_only()) {
                    Ok(vector_of_context_files) => {
                        for context_file in vector_of_context_files {
                            files.push_str(
                                format!("📎:{}:{}-{}\n```\n{}```\n\n",
                                        context_file.file_name,
                                        context_file.line1,
                                        context_file.line2,
                                        crate::nicer_logs::first_n_chars(&context_file.file_content, 40)).as_str()
                            )
                        }
                    }
                    _ => {}
                }
                files
            }
            _ => {
                continue;
            }
        };
        context.insert_str(0, &message_row);
    }
    format!("# Conversation\n{context}")
}


pub async fn compress_trajectory(
    gcx: Arc<ARwLock<GlobalContext>>,
    messages: &Vec<ChatMessage>,
) -> Result<String, String> {
    if messages.is_empty() {
        return Err("The provided chat is empty".to_string());
    }
    let (model_id, n_ctx) = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => {
            let model_id = caps.defaults.chat_default_model.clone();
            if let Some(model_rec) = caps.chat_models.get(&strip_model_from_finetune(&model_id)) {
                Ok((model_id, model_rec.base.n_ctx))
            } else {
                Err(format!(
                    "Model '{}' not found, server has these models: {:?}",
                    model_id, caps.chat_models.keys()
                ))
            }
        },
        Err(_) => Err("No caps available".to_string()),
    }?;
    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        n_ctx,
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
        "compress_trajectory:1.0",
        vec![ChatMessage {
            role: "user".to_string(),
            content: ChatContent::SimpleText(_make_prompt(&messages)),
            ..Default::default()
        }],
        Some(TEMPERATURE),
        Some(8192),
        None
    ).await.map_err(|e| format!("Error: {}", e))?;
    let content = new_messages
        .into_iter()
        .last()
        .map(|last_m| last_m.content.content_text_only())
        .ok_or("No message have been found".to_string())?;
    let compressed_message = format!("{content}\n\nPlease, continue the conversation based on the provided summary");
    Ok(compressed_message)
}
