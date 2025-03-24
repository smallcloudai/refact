use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;
use axum::http::StatusCode;
use crate::caps::resolve_chat_model;
use crate::subchat::subchat_single;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatContent, ChatUsage, ContextEnum, SubchatParameters, ContextFile, PostprocessSettings};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::cached_tokenizers;
use crate::custom_error::ScratchError;
use crate::global_context::try_load_caps_quickly_if_not_present;
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::scratchpads::scratchpad_utils::count_tokens;

pub struct ToolDeepAnalysis;


static TOKENS_EXTRA_BUDGET_PERCENT: f32 = 0.06;


async fn _make_prompt(
    ccx: Arc<AMutex<AtCommandsContext>>,
    subchat_params: &SubchatParameters,
    problem_statement: &String, 
    previous_messages: &Vec<ChatMessage>
) -> Result<String, String> {
    let gcx = ccx.lock().await.global_context.clone();
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await.map_err(|x| x.message)?;
    let model_rec = resolve_chat_model(caps, &subchat_params.subchat_model)?;
    let tokenizer = cached_tokenizers::cached_tokenizer(gcx.clone(), &model_rec.base).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error loading tokenizer: {}", e))).map_err(|x| x.message)?;
    let tokens_extra_budget = (subchat_params.subchat_n_ctx as f32 * TOKENS_EXTRA_BUDGET_PERCENT) as usize;
    let mut tokens_budget: i64 = (subchat_params.subchat_n_ctx - subchat_params.subchat_max_new_tokens - subchat_params.subchat_tokens_for_rag - tokens_extra_budget) as i64;
    let final_message = format!("***Problem:***\n{problem_statement}\n\n***Problem context:***\n");
    tokens_budget -= count_tokens(&tokenizer.read().unwrap(), &final_message) as i64;
    let mut context = "".to_string(); 
    let mut context_files: Vec<ContextFile> = vec![];
    for message in previous_messages.iter().rev() {
        let message_row = match message.role.as_str() {
            "system" => {
                // just skipping it
                continue;
            }
            "user" => {
                format!("👤:\n{}\n\n", &message.content.content_text_only())
            }
            "assistant" => {
                format!("🤖:\n{}\n\n", &message.content.content_text_only())
            }
            "context_file" => {
                context_files.extend(serde_json::from_str::<Vec<ContextFile>>(&message.content.content_text_only())
                    .map_err(|e| format!("Failed to decode context_files JSON: {:?}", e))?);
                continue;
            }
            "tool" => {
                format!("📎:\n{}\n\n", &message.content.content_text_only())
            }
            _ => {
                tracing::info!("skip adding message to the context: {}", message.content.content_text_only());
                continue;
            }
        };
        let left_tokens = tokens_budget - count_tokens(&tokenizer.read().unwrap(), &message_row) as i64;
        if left_tokens < 0 {
            // we do not end here, maybe there are smaller useful messages at the beginning
            continue;
        } else {
            tokens_budget = left_tokens;
            context.insert_str(0, &message_row);
        }
    }
    if !context_files.is_empty() {
        let mut pp_settings = PostprocessSettings::new();
        pp_settings.max_files_n = context_files.len();
        let mut files_context = "".to_string();
        for context_file in postprocess_context_files(
            gcx.clone(),
            &mut context_files,
            tokenizer.clone(),
            subchat_params.subchat_tokens_for_rag + tokens_budget.max(0) as usize,
            false,
            &pp_settings,
        ).await {
            files_context.push_str(
                &format!("📎 {}:{}-{}\n```\n{}```\n\n",
                         context_file.file_name,
                         context_file.line1,
                         context_file.line2,
                         context_file.file_content)
            );
        }
        Ok(format!("{final_message}{context}\n***Files context:***\n{files_context}"))
    } else {
        Ok(format!("{final_message}{context}"))
    }
}


#[async_trait]
impl Tool for ToolDeepAnalysis {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let mut usage_collector = ChatUsage { ..Default::default() };
        let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();

        let problem_statement = match args.get("problem_statement") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `problem_statement` is not a string: {:?}", v)),
            None => return Err("Missing argument `problem_statement`".to_string())
        };

        let subchat_params: SubchatParameters = crate::tools::tools_execute::unwrap_subchat_params(ccx.clone(), "deep_analysis").await?;

        let external_messages = {
            let ccx_lock = ccx.lock().await;
            ccx_lock.messages.clone()
        };
        let prompt = _make_prompt(ccx.clone(), &subchat_params, &problem_statement, &external_messages).await?;
        tracing::info!("deep analysis prompt:\n{}", prompt);

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
            vec![ChatMessage::new("user".to_string(), prompt)],
            Some(vec![]),
            None,
            false,
            subchat_params.subchat_temperature,
            Some(subchat_params.subchat_max_new_tokens),
            1,
            subchat_params.subchat_reasoning_effort,
            false,
            Some(&mut usage_collector),
            Some(tool_call_id.clone()),
            Some(format!("{log_prefix}-deep-analysis")),
        ).await?[0].clone();

        let final_message = model_says.last()
            .ok_or("No messages from model")?
            .content
            .content_text_only();
        tracing::info!("deep analysis response:\n{}", final_message);

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(final_message),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            usage: Some(usage_collector),
            ..Default::default()
        }));

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}

