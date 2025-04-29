use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;
use axum::http::StatusCode;
use crate::subchat::subchat_single;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatContent, ChatUsage, ContextEnum, SubchatParameters, ContextFile, PostprocessSettings};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::cached_tokenizers;
use crate::custom_error::ScratchError;
use crate::global_context::try_load_caps_quickly_if_not_present;
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::scratchpads::scratchpad_utils::count_tokens;

pub struct ToolStrategicPlanning;


static TOKENS_EXTRA_BUDGET_PERCENT: f32 = 0.06;

static ROOT_CAUSE_ANALYSIS_PROMPT: &str = r#"Based on the conversation and context below, please perform a thorough root cause analysis of the problem. Identify all possible causes, including:
1. Direct causes - immediate factors that could lead to this issue.
2. Underlying causes - deeper systemic issues that might be contributing.
3. Environmental factors - external conditions that might influence the problem.
4. Edge cases - unusual scenarios that could trigger this issue.
For each potential cause, explain:
- Why it might be causing the problem
- How it could manifest in the observed symptoms
- What evidence supports or refutes this cause

Finally, rank the causes from most to least likely, and explain your reasoning.
Do not create code, just write text."#;


async fn _make_prompt(
    ccx: Arc<AMutex<AtCommandsContext>>,
    subchat_params: &SubchatParameters,
    problem_statement: &String, 
    previous_messages: &Vec<ChatMessage>
) -> Result<String, String> {
    let gcx = ccx.lock().await.global_context.clone();
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await.map_err(|x| x.message)?;
    let tokenizer = cached_tokenizers::cached_tokenizer(caps, gcx.clone(), subchat_params.subchat_model.to_string()).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error loading tokenizer: {}", e))).map_err(|x| x.message)?;
    let tokens_extra_budget = (subchat_params.subchat_n_ctx as f32 * TOKENS_EXTRA_BUDGET_PERCENT) as usize;
    let mut tokens_budget: i64 = (subchat_params.subchat_n_ctx - subchat_params.subchat_max_new_tokens - subchat_params.subchat_tokens_for_rag - tokens_extra_budget) as i64;
    let final_message = problem_statement.to_string();
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

/// Executes a subchat iteration and returns the last message from the response
async fn _execute_subchat_iteration(
    ccx_subchat: Arc<AMutex<AtCommandsContext>>,
    subchat_params: &SubchatParameters,
    history: Vec<ChatMessage>,
    iter_max_new_tokens: usize,
    usage_collector: &mut ChatUsage,
    tool_call_id: &String,
    log_suffix: &str,
    log_prefix: &str,
) -> Result<(Vec<ChatMessage>, ChatMessage), String> {
    let choices = subchat_single(
        ccx_subchat.clone(),
        subchat_params.subchat_model.as_str(),
        history,
        Some(vec![]),
        None,
        false,
        subchat_params.subchat_temperature,
        Some(iter_max_new_tokens),
        1,
        subchat_params.subchat_reasoning_effort.clone(),
        false,
        Some(usage_collector),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-strategic-planning-{log_suffix}")),
    ).await?;

    let session = choices.into_iter().next().unwrap();
    let reply = session.last().unwrap().clone();
    
    Ok((session, reply))
}


#[async_trait]
impl Tool for ToolStrategicPlanning {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        _args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let mut usage_collector = ChatUsage { ..Default::default() };
        let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let subchat_params: SubchatParameters = crate::tools::tools_execute::unwrap_subchat_params(ccx.clone(), "strategic_planning").await?;
        let external_messages = {
            let ccx_lock = ccx.lock().await;
            ccx_lock.messages.clone()
        };
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
                ccx_lock.current_model.clone(),
            ).await;
            t.subchat_tx = ccx_lock.subchat_tx.clone();
            t.subchat_rx = ccx_lock.subchat_rx.clone();
            Arc::new(AMutex::new(t))
        };
        
        // ZERO ITERATION: Root Cause Analysis
        tracing::info!("ZERO ITERATION: Root Cause Analysis");
        let prompt = _make_prompt(
            ccx.clone(),
            &subchat_params,
            &ROOT_CAUSE_ANALYSIS_PROMPT.to_string(),
            &external_messages
        ).await?;
        let history: Vec<ChatMessage> = vec![ChatMessage::new("user".to_string(), prompt)];
        let (_, root_cause_reply) = _execute_subchat_iteration(
            ccx_subchat.clone(),
            &subchat_params,
            history.clone(),
            subchat_params.subchat_max_new_tokens,
            &mut usage_collector,
            tool_call_id,
            "root-cause-analysis",
            &log_prefix,
        ).await?;
        
        // FIRST ITERATION: Get the initial solution
        let prompt = _make_prompt(
            ccx.clone(),
            &subchat_params,
            &format!("***Problem:***\n{}\n\n***Problem context:***\n", root_cause_reply.content.content_text_only()),
            &external_messages
        ).await?;
        let mut history: Vec<ChatMessage> = vec![ChatMessage::new("user".to_string(), prompt)];
        tracing::info!("FIRST ITERATION: Get the initial solution");
        let (sol_session, first_reply) = _execute_subchat_iteration(
            ccx_subchat.clone(),
            &subchat_params,
            history.clone(),
            subchat_params.subchat_max_new_tokens / 3,
            &mut usage_collector,
            tool_call_id,
            "get-initial-solution",
            &log_prefix,
        ).await?;
        history = sol_session.clone();

        // SECOND ITERATION: Ask for a critique
        tracing::info!("THIRD ITERATION: Ask for a critique");
        let critique_prompt = "Please critique the solution above. Identify any weaknesses, limitations, or bugs. Be specific and thorough in your analysis. Remember, that the final solution must be minimal, robust, effective. We can create new tests but cannot modify existing.";
        history.push(ChatMessage::new("user".to_string(), critique_prompt.to_string()));
        let (crit_session, critique_reply) = _execute_subchat_iteration(
            ccx_subchat.clone(),
            &subchat_params,
            history.clone(),
            subchat_params.subchat_max_new_tokens / 3,
            &mut usage_collector,
            tool_call_id,
            "critique",
            &log_prefix,
        ).await?;
        history = crit_session.clone();

        // THIRD ITERATION: Ask for an improved solution
        tracing::info!("FOURTH ITERATION: Ask for an improved solution");
        let improve_prompt = "Please improve the original solution based on the critique. Provide a refined solution that addresses the weaknesses identified in the critique.";
        history.push(ChatMessage::new("user".to_string(), improve_prompt.to_string()));
        let (_imp_session, improved_reply) = _execute_subchat_iteration(
            ccx_subchat,
            &subchat_params,
            history.clone(),
            subchat_params.subchat_max_new_tokens / 3,
            &mut usage_collector,
            tool_call_id,
            "solution-refinement",
            &log_prefix,
        ).await?;

        let final_message = format!(
            "# Root cause analysis:\n\n{}\n\n# Initial Solution\n\n{}\n\n# Critique\n\n{}\n\n# Improved Solution\n\n{}",
            root_cause_reply.content.content_text_only(),
            first_reply.content.content_text_only(),
            critique_reply.content.content_text_only(),
            improved_reply.content.content_text_only()
        );
        tracing::info!("strategic planning response (combined):\n{}", final_message);
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
