use std::collections::HashMap;
use std::path::PathBuf;
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
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::caps::resolve_chat_model;
use crate::custom_error::ScratchError;
use crate::files_correction::{canonicalize_normalized_path, get_project_dirs, preprocess_path_for_normalization};
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::global_context::try_load_caps_quickly_if_not_present;
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::tokens::count_text_tokens_with_fallback;

pub struct ToolStrategicPlanning;


static TOKENS_EXTRA_BUDGET_PERCENT: f32 = 0.06;

static ROOT_CAUSE_ANALYSIS_PROMPT: &str = r#"Based on the conversation and context below, please perform a thorough root cause analysis of the problem. Pay attention on provided debugging report.
Identify all possible causes, including:
1. Direct causes - immediate factors that could lead to this issue.
2. Underlying causes - deeper systemic issues that might be contributing.
3. Environmental factors - external conditions that might influence the problem.
4. Edge cases - unusual scenarios that could trigger this issue.
5. Focus on special methods and implicit conversions and investigate cross-component interactions.

For each potential cause, explain:
- Why it might be causing the problem
- How it could manifest in the observed symptoms
- What evidence supports or refutes this cause

Rank the causes from most to least likely, and explain your reasoning.

Your final goal is to find the root cause of the problem! Take as many time as needed.
Do not create code, just write text."#;

static SOLVER_PROMPT: &str = r#"Your task is to identify and solve the problem by the given root cause analysis of the problem, conversation and context files
# Solution rules
1. **Contract First**  
   Never change public signatures, return types, log formats, or exception classes unless tests demand it; add features via defaults or **kwargs, never break old calls.
2. **Fix Upstream, Validate Up-Front**  
   Centralize input sanitization/normalization at the highest shared layer and let every caller flow through it; patch the root cause, not the symptom.
3. **Type & Shape Fidelity**  
   Preserve incoming container kinds and indices (list→list, set→set, DataFrame index untouched); don’t coerce mutables silently.
4. **Loud, Consistent Failure**  
   Replace `assert` with framework-specific exceptions and keep canonical wording; log or warn exactly as the library already does, use `DeprecationWarning` before behavior flips.
5. **State-Safe Sequencing**  
   Build internal structures before triggering callbacks or side-effects; after each `yield`, re-check container size to catch in-loop mutations.
6. **Canonical Values & Hashes**  
   Use the framework’s canonical sentinels (`S.One`, ISO-8601 w/ microseconds, etc.); when you add `__eq__`, add `__hash__`, and include every identity-changing field (email, password) in hashes/tokens.
7. **Deterministic Ordering**  
   Deduplicate with order-preserving tools (`OrderedDict`, `sorted(key=…)`) so outputs stay stable across runs and Python versions.
8. **Holistic Serialization**  
   Serialize original, unscaled state (e.g., pre-device-DPI) and restore verbatim; ensure round-trips leave objects functionally identical.
9. **Feature Flags & Compatibility**  
   Ship new behavior behind opt-in flags; keep fallbacks for custom models, backends, or deprecated APIs until the deprecation window closes.
"#;


static GUARDRAILS_PROMPT: &str = r#"Reminders:
- Do not create documents, README.md, or other files which are non-related to fixing the problem. 
- Convert generated changes into the `update_textdoc()` or `create_textdoc()` tools calls. Do not create patches!
- Do not modify existing tests.
- Create new test files only using `create_textdoc()`."#;

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
        
        // ZERO ITERATION: Root Cause Analysis
        tracing::info!("ZERO ITERATION: Root Cause Analysis");
        let prompt = _make_prompt(
            ccx.clone(),
            &subchat_params,
            &ROOT_CAUSE_ANALYSIS_PROMPT.to_string(),
            &important_paths,
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
            &format!("{SOLVER_PROMPT} # Root cause analysis:\n{}", root_cause_reply.content.content_text_only()),
            &important_paths,
            &external_messages
        ).await?;
        let mut history: Vec<ChatMessage> = vec![ChatMessage::new("user".to_string(), prompt)];
        tracing::info!("FIRST ITERATION: Get the initial solution");
        let (sol_session, initial_solution) = _execute_subchat_iteration(
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
        // tracing::info!("THIRD ITERATION: Ask for a critique");
        // history.push(ChatMessage::new("user".to_string(), CRITIQUE_PROMPT.to_string()));
        // let (crit_session, critique) = _execute_subchat_iteration(
        //     ccx_subchat.clone(),
        //     &subchat_params,
        //     history.clone(),
        //     subchat_params.subchat_max_new_tokens / 3,
        //     &mut usage_collector,
        //     tool_call_id,
        //     "critique",
        //     &log_prefix,
        // ).await?;
        // history = crit_session.clone();
        // 
        // // THIRD ITERATION: Ask for an improved solution
        // tracing::info!("FOURTH ITERATION: Ask for an improved solution");
        // let improve_prompt = "Please improve the original solution based on the critique. Provide a refined solution that addresses the weaknesses identified in the critique.";
        // history.push(ChatMessage::new("user".to_string(), improve_prompt.to_string()));
        // let (_imp_session, improved_solution) = _execute_subchat_iteration(
        //     ccx_subchat,
        //     &subchat_params,
        //     history.clone(),
        //     subchat_params.subchat_max_new_tokens / 3,
        //     &mut usage_collector,
        //     tool_call_id,
        //     "solution-refinement",
        //     &log_prefix,
        // ).await?;

        let final_message = format!(
            "# Root cause analysis:\n\n{}\n\n# Initial Solution\n\n{}\n{}",
            root_cause_reply.content.content_text_only(),
            initial_solution.content.content_text_only(),
            // critique.content.content_text_only(),
            // improved_solution.content.content_text_only(),
            GUARDRAILS_PROMPT.to_string()
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
