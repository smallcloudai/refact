use std::collections::HashMap;
use std::path::PathBuf;
use std::string::ToString;
use std::sync::Arc;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;
use axum::http::StatusCode;
use crate::subchat::subchat_single;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ChatUsage, ContextEnum, SubchatParameters, ContextFile, PostprocessSettings};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::caps::resolve_chat_model;
use crate::custom_error::ScratchError;
use crate::files_correction::{canonicalize_normalized_path, get_project_dirs, preprocess_path_for_normalization};
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::global_context::try_load_caps_quickly_if_not_present;
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::tokens::count_text_tokens_with_fallback;

pub struct ToolStrategicPlanning {
    pub config_path: String,
}


static TOKENS_EXTRA_BUDGET_PERCENT: f32 = 0.06;

static SOLVER_PROMPT: &str = r#"Your task is to identify and solve the problem by the given conversation and context files.
The solution must be robust and complete and adressing all corner cases.
Also make a couple of alternative ways to solve the problem, if the initial solution doesn't work."#;


static GUARDRAILS_PROMPT: &str = r#"💿 Now confirm the plan with the user"#;

async fn _make_prompt(
    ccx: Arc<AMutex<AtCommandsContext>>,
    subchat_params: &SubchatParameters,
    problem_statement: &String, 
    important_paths: &Vec<PathBuf>,
    previous_messages: &Vec<ChatMessage>,
) -> Result<String, String> {
    let gcx = ccx.lock().await.global_context.clone();
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await.map_err(|x| x.message)?;
    let model_rec = resolve_chat_model(caps, &subchat_params.subchat_model)?;
    let tokenizer = crate::tokens::cached_tokenizer(gcx.clone(), &model_rec.base).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e)).map_err(|x| x.message)?;
    let tokens_extra_budget = (subchat_params.subchat_n_ctx as f32 * TOKENS_EXTRA_BUDGET_PERCENT) as usize;
    let mut tokens_budget: i64 = (subchat_params.subchat_n_ctx - subchat_params.subchat_max_new_tokens - subchat_params.subchat_tokens_for_rag - tokens_extra_budget) as i64;
    let final_message = problem_statement.to_string();
    tokens_budget -= count_text_tokens_with_fallback(tokenizer.clone(), &final_message) as i64;
    let mut context = "".to_string();
    let mut context_files = vec![];
    for p in important_paths.iter() {
        context_files.push(match get_file_text_from_memory_or_disk(gcx.clone(), &p).await {
            Ok(text) => {
                let total_lines = text.lines().count();
                tracing::info!("adding file '{:?}' to the context", p);
                ContextFile {
                    file_name: p.to_string_lossy().to_string(),
                    file_content: "".to_string(),
                    line1: 1,
                    line2: total_lines.max(1),
                    symbols: vec![],
                    gradient_type: 4,
                    usefulness: 100.0,
                }
            },
            Err(_) => {
                tracing::warn!("failed to read file '{:?}'. Skipping...", p);
                continue;
            }
        })
    }
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
            "tool" => {
                format!("📎:\n{}\n\n", &message.content.content_text_only())
            }
            _ => {
                tracing::info!("skip adding message to the context: {}", crate::nicer_logs::first_n_chars(&message.content.content_text_only(), 40));
                continue;
            }
        };
        let left_tokens = tokens_budget - count_text_tokens_with_fallback(tokenizer.clone(), &message_row) as i64;
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
        Ok(format!("{final_message}\n\n# Conversation\n{context}\n\n# Files context\n{files_context}"))
    } else {
        Ok(format!("{final_message}\n\n# Conversation\n{context}"))
    }
}

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
    crate::tools::tools_execute::update_usage_from_message(usage_collector, &reply);

    Ok((session, reply))
}


#[async_trait]
impl Tool for ToolStrategicPlanning {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "strategic_planning".to_string(),
            display_name: "Strategic Planning".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Strategically plan a solution for a complex problem or create a comprehensive approach.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "important_paths".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated list of all filenames which are required to be considered for resolving the problem. More files - better, include them even if you are not sure.".to_string(),
                }
            ],
            parameters_required: vec!["important_paths".to_string()],
        }
    }
    
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let gcx = ccx.lock().await.global_context.clone();
        let important_paths = match args.get("important_paths") {
            Some(Value::String(s)) => {
                let mut paths = vec![];
                for s in s.split(",") {
                    let s_raw = s.trim().to_string();
                    let candidates_file = file_repair_candidates(gcx.clone(), &s_raw, 3, false).await;
                    paths.push(match return_one_candidate_or_a_good_error(gcx.clone(), &s_raw, &candidates_file, &get_project_dirs(gcx.clone()).await, false).await {
                        Ok(f) => canonicalize_normalized_path(PathBuf::from(preprocess_path_for_normalization(f.trim().to_string()))),
                        Err(_) => {
                            tracing::info!("cannot find a good file candidate for `{s_raw}`");
                            continue;
                        }
                    })
                }
                paths
            },
            Some(v) => return Err(format!("argument `paths` is not a string: {:?}", v)),
            None => return Err("Missing argument `paths`".to_string())
        };
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
        let prompt = _make_prompt(
            ccx.clone(),
            &subchat_params,
            &SOLVER_PROMPT.to_string(),
            &important_paths,
            &external_messages
        ).await?;
        let history: Vec<ChatMessage> = vec![ChatMessage::new("user".to_string(), prompt)];
        tracing::info!("FIRST ITERATION: Get the initial solution");
        let (_, initial_solution) = _execute_subchat_iteration(
            ccx_subchat.clone(),
            &subchat_params,
            history.clone(),
            subchat_params.subchat_max_new_tokens / 3,
            &mut usage_collector,
            tool_call_id,
            "get-initial-solution",
            &log_prefix,
        ).await?;

        let final_message = format!("# Solution\n{}", initial_solution.content.content_text_only());
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
        // results.push(ContextEnum::ChatMessage(ChatMessage {
        //     role: "cd_instruction".to_string(),
        //     content: ChatContent::SimpleText(GUARDRAILS_PROMPT.to_string()),
        //     ..Default::default()
        // }));

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}
