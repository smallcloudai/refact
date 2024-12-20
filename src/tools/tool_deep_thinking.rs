use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;
use crate::subchat::subchat_single;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatContent, ChatUsage, ContextEnum, SubchatParameters, ContextFile};
use crate::at_commands::at_commands::AtCommandsContext;


pub struct ToolDeepThinking;


#[async_trait]
impl Tool for ToolDeepThinking {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let mut usage_collector = ChatUsage { ..Default::default() };
        let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();

        let what_to_think_about = match args.get("what_to_think_about") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `what_to_think_about` is not a string: {:?}", v)),
            None => return Err("Missing argument `what_to_think_about`".to_string())
        };

        let subchat_params: SubchatParameters = crate::tools::tools_execute::unwrap_subchat_params(ccx.clone(), "deep_thinking").await?;

        let add_those_up = {
            let ccx_lock = ccx.lock().await;
            ccx_lock.messages.clone()
        };
        let mut previous_stuff = String::new();
        for message in add_those_up {
            match message.role.as_str() {
                "user" => {
                    previous_stuff.push_str("👤 says:\n");
                    previous_stuff.push_str(&message.content.content_text_only());
                    previous_stuff.push_str("\n\n");
                }
                "assistant" => {
                    previous_stuff.push_str("🤖 says:\n");
                    previous_stuff.push_str(&message.content.content_text_only());
                    previous_stuff.push_str("\n\n");
                }
                "context_file" => {
                    let context_files: Vec<ContextFile> = serde_json::from_str(&message.content.content_text_only())
                        .map_err(|e| format!("Failed to decode context_files JSON: {:?}", e))?;
                    for context_file in context_files {
                        previous_stuff.push_str("📎 ");
                        previous_stuff.push_str(&context_file.file_name);
                        previous_stuff.push_str("\n```\n");
                        previous_stuff.push_str(&context_file.file_content);
                        previous_stuff.push_str("\n```\n\n");
                    }
                }
                _ => {
                    tracing::error!("unknown role in message: {:?}, skipped", message);
                }
            }
        }
        tracing::info!("previous_stuff:\n{}", previous_stuff);

        let mut msgs = vec![];
        msgs.push(ChatMessage::new("user".to_string(), previous_stuff));
        msgs.push(ChatMessage::new("user".to_string(), what_to_think_about));

        let ccx_subchat = {
            let ccx_lock = ccx.lock().await;
            let mut t = AtCommandsContext::new(
                ccx_lock.global_context.clone(),
                subchat_params.subchat_n_ctx,
                0,
                false,
                ccx_lock.messages.clone(),
                ccx_lock.chat_id.clone(),
                ccx_lock.should_execute_remotely,
            ).await;
            t.subchat_tx = ccx_lock.subchat_tx.clone();
            t.subchat_rx = ccx_lock.subchat_rx.clone();
            Arc::new(AMutex::new(t))
        };

        let model_says: Vec<ChatMessage> = subchat_single(
            ccx_subchat.clone(),
            subchat_params.subchat_model.as_str(),
            msgs,
            vec![],
            None,
            false,
            None,
            Some(subchat_params.subchat_max_new_tokens),
            1,
            Some(&mut usage_collector),
            Some(tool_call_id.clone()),
            Some(format!("{log_prefix}-deep-thinking")),
        ).await?[0].clone();

        let final_message = model_says.last()
            .ok_or("No messages from model")?
            .content
            .content_text_only();
        tracing::info!("deep thinking response1:\n{:?}", model_says.last());
        tracing::info!("deep thinking response2:\n{}", final_message);

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(final_message),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            usage: Some(usage_collector),
            ..Default::default()
        }));

        // results.push(ContextEnum::ChatMessage(ChatMessage {
        //     role: "cd_instruction".to_string(),
        //     content: ChatContent::SimpleText(cd_instruction),
        //     tool_calls: None,
        //     tool_call_id: "".to_string(),
        //     usage: None,
        //     ..Default::default()
        // }));

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}

