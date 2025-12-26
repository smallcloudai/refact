use std::sync::Arc;
use std::collections::HashSet;
use serde_json::{json, Value};
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::execute_at::run_at_commands_locally;
use crate::call_validation::{ChatMessage, ChatMeta, ReasoningEffort, SamplingParameters};
use crate::caps::{resolve_chat_model, ChatModelRecord};
use crate::global_context::GlobalContext;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::scratchpads::scratchpad_utils::HasRagResults;
use crate::call_validation::ChatMode;
use crate::tools::tools_description::ToolDesc;
use super::tools::execute_tools;
use super::types::ThreadParams;

use super::history_limit::fix_and_limit_messages_history;
use super::prompts::prepend_the_right_system_prompt_and_maybe_more_initial_messages;
use super::openai_convert::convert_messages_to_openai_format;

const MIN_BUDGET_TOKENS: usize = 1024;

pub struct PreparedChat {
    pub prompt: String,
}

pub struct ChatPrepareOptions {
    pub prepend_system_prompt: bool,
    pub allow_at_commands: bool,
    pub allow_tool_prerun: bool,
    pub supports_tools: bool,
    pub use_compression: bool,
}

impl Default for ChatPrepareOptions {
    fn default() -> Self {
        Self {
            prepend_system_prompt: true,
            allow_at_commands: true,
            allow_tool_prerun: true,
            supports_tools: true,
            use_compression: true,
        }
    }
}

pub async fn prepare_chat_passthrough(
    gcx: Arc<ARwLock<GlobalContext>>,
    ccx: Arc<AMutex<AtCommandsContext>>,
    t: &HasTokenizerAndEot,
    messages: Vec<ChatMessage>,
    model_id: &str,
    tools: Vec<ToolDesc>,
    meta: &ChatMeta,
    sampling_parameters: &mut SamplingParameters,
    options: &ChatPrepareOptions,
    style: &Option<String>,
) -> Result<PreparedChat, String> {
    let mut has_rag_results = HasRagResults::new();
    let tool_names: HashSet<String> = tools.iter().map(|x| x.name.clone()).collect();

    // 1. Resolve model early to get reasoning params before history limiting
    let caps = crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0).await
        .map_err(|e| e.message)?;
    let model_record = resolve_chat_model(caps, model_id)?;

    let effective_n_ctx = if let Some(cap) = meta.context_tokens_cap {
        if cap == 0 {
            model_record.base.n_ctx
        } else {
            cap.min(model_record.base.n_ctx)
        }
    } else {
        model_record.base.n_ctx
    };

    // 2. Adapt sampling parameters for reasoning models BEFORE history limiting
    adapt_sampling_for_reasoning_models(sampling_parameters, &model_record);

    // 3. System prompt injection (decoupled from allow_at_commands)
    let prompt_tool_names = if options.allow_at_commands { tool_names.clone() } else { HashSet::new() };
    let messages = if options.prepend_system_prompt {
        prepend_the_right_system_prompt_and_maybe_more_initial_messages(
            gcx.clone(),
            messages,
            meta,
            &mut has_rag_results,
            prompt_tool_names,
        ).await
    } else {
        messages
    };

    // 4. Run @-commands
    let (mut messages, _) = if options.allow_at_commands {
        run_at_commands_locally(
            ccx.clone(),
            t.tokenizer.clone(),
            sampling_parameters.max_new_tokens,
            messages,
            &mut has_rag_results,
        ).await
    } else {
        (messages, false)
    };

    // 5. Tool prerun - restricted to allowed tools only
    if options.supports_tools && options.allow_tool_prerun {
        if let Some(last_msg) = messages.last() {
            if last_msg.role == "assistant" {
                if let Some(ref tool_calls) = last_msg.tool_calls {
                    let filtered_calls: Vec<_> = tool_calls.iter()
                        .filter(|tc| tool_names.contains(&tc.function.name))
                        .cloned()
                        .collect();
                    if !filtered_calls.is_empty() {
                        let thread = ThreadParams {
                            id: meta.chat_id.clone(),
                            model: model_id.to_string(),
                            context_tokens_cap: Some(effective_n_ctx),
                            ..Default::default()
                        };
                        let (tool_results, _) = execute_tools(
                            gcx.clone(),
                            &filtered_calls,
                            &messages,
                            &thread,
                            ChatMode::AGENT,
                            super::tools::ExecuteToolsOptions::default(),
                        ).await;
                        messages.extend(tool_results);
                    }
                }
            }
        }
    }

    // 6. Build tools JSON - only insert key if there are tools
    let mut big_json = json!({});
    let filtered_tools: Vec<ToolDesc> = if options.supports_tools {
        tools.iter()
            .filter(|x| x.is_supported_by(model_id))
            .cloned()
            .collect()
    } else {
        vec![]
    };
    let openai_tools: Vec<Value> = filtered_tools.iter()
        .map(|tool| tool.clone().into_openai_style())
        .collect();
    let tools_str_for_limit = if openai_tools.is_empty() {
        None
    } else {
        big_json["tools"] = json!(openai_tools);
        serde_json::to_string(&openai_tools).ok()
    };

    // 7. History limiting with correct token budget
    let (limited_msgs, compression_strength) = fix_and_limit_messages_history(
        t,
        &messages,
        sampling_parameters,
        effective_n_ctx,
        tools_str_for_limit,
        model_id,
        options.use_compression,
    )?;

    // 8. Strip thinking blocks if thinking is disabled
    let limited_adapted_msgs = strip_thinking_blocks_if_disabled(limited_msgs, sampling_parameters, &model_record);

    // 9. Convert to OpenAI format
    let converted_messages = convert_messages_to_openai_format(
        limited_adapted_msgs,
        style,
        &model_record.base.id,
    );

    big_json["messages"] = json!(converted_messages);
    big_json["compression_strength"] = json!(compression_strength);

    // 10. Serialize without panic
    let body = serde_json::to_string(&big_json).map_err(|e| format!("JSON serialization error: {}", e))?;
    let prompt = format!("PASSTHROUGH {}", body);

    Ok(PreparedChat { prompt })
}

fn adapt_sampling_for_reasoning_models(
    sampling_parameters: &mut SamplingParameters,
    model_record: &ChatModelRecord,
) {
    let Some(ref supports_reasoning) = model_record.supports_reasoning else {
        sampling_parameters.reasoning_effort = None;
        sampling_parameters.thinking = None;
        sampling_parameters.enable_thinking = None;
        return;
    };

    match supports_reasoning.as_ref() {
        "openai" => {
            if model_record.supports_boost_reasoning && sampling_parameters.boost_reasoning {
                sampling_parameters.reasoning_effort = Some(ReasoningEffort::Medium);
            }
            if sampling_parameters.max_new_tokens <= 8192 {
                sampling_parameters.max_new_tokens *= 2;
            }
            sampling_parameters.temperature = model_record.default_temperature;
        },
        "anthropic" => {
            let budget_tokens = if sampling_parameters.max_new_tokens > MIN_BUDGET_TOKENS {
                (sampling_parameters.max_new_tokens / 2).max(MIN_BUDGET_TOKENS)
            } else {
                0
            };
            let should_enable_thinking = (model_record.supports_boost_reasoning && sampling_parameters.boost_reasoning)
                || sampling_parameters.reasoning_effort.is_some();
            if should_enable_thinking && budget_tokens > 0 {
                sampling_parameters.thinking = Some(json!({
                    "type": "enabled",
                    "budget_tokens": budget_tokens,
                }));
            }
            sampling_parameters.reasoning_effort = None;
        },
        "qwen" => {
            sampling_parameters.enable_thinking = Some(
                model_record.supports_boost_reasoning && sampling_parameters.boost_reasoning
            );
            sampling_parameters.temperature = model_record.default_temperature;
        },
        _ => {
            sampling_parameters.temperature = model_record.default_temperature;
        }
    };
}

fn is_thinking_enabled(sampling_parameters: &SamplingParameters) -> bool {
    sampling_parameters.thinking
        .as_ref()
        .and_then(|t| t.get("type"))
        .and_then(|t| t.as_str())
        .map(|t| t == "enabled")
        .unwrap_or(false)
        || sampling_parameters.reasoning_effort.is_some()
        || sampling_parameters.enable_thinking == Some(true)
}

fn strip_thinking_blocks_if_disabled(
    messages: Vec<ChatMessage>,
    sampling_parameters: &SamplingParameters,
    model_record: &ChatModelRecord,
) -> Vec<ChatMessage> {
    if model_record.supports_reasoning.is_none() || !is_thinking_enabled(sampling_parameters) {
        messages.into_iter().map(|mut msg| {
            msg.thinking_blocks = None;
            msg
        }).collect()
    } else {
        messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_validation::ChatContent;

    fn make_model_record(supports_reasoning: Option<&str>) -> ChatModelRecord {
        ChatModelRecord {
            base: Default::default(),
            default_temperature: Some(0.7),
            supports_reasoning: supports_reasoning.map(|s| s.to_string()),
            supports_boost_reasoning: true,
            ..Default::default()
        }
    }

    fn make_sampling_params() -> SamplingParameters {
        SamplingParameters {
            max_new_tokens: 4096,
            temperature: Some(1.0),
            reasoning_effort: None,
            thinking: None,
            enable_thinking: None,
            boost_reasoning: false,
            ..Default::default()
        }
    }

    #[test]
    fn test_is_thinking_enabled_with_thinking_json() {
        let mut params = make_sampling_params();
        params.thinking = Some(serde_json::json!({"type": "enabled", "budget_tokens": 1024}));
        assert!(is_thinking_enabled(&params));
    }

    #[test]
    fn test_is_thinking_enabled_with_thinking_disabled() {
        let mut params = make_sampling_params();
        params.thinking = Some(serde_json::json!({"type": "disabled"}));
        assert!(!is_thinking_enabled(&params));
    }

    #[test]
    fn test_is_thinking_enabled_with_reasoning_effort() {
        let mut params = make_sampling_params();
        params.reasoning_effort = Some(ReasoningEffort::Medium);
        assert!(is_thinking_enabled(&params));
    }

    #[test]
    fn test_is_thinking_enabled_with_enable_thinking_true() {
        let mut params = make_sampling_params();
        params.enable_thinking = Some(true);
        assert!(is_thinking_enabled(&params));
    }

    #[test]
    fn test_is_thinking_enabled_with_enable_thinking_false() {
        let mut params = make_sampling_params();
        params.enable_thinking = Some(false);
        assert!(!is_thinking_enabled(&params));
    }

    #[test]
    fn test_is_thinking_enabled_all_none() {
        let params = make_sampling_params();
        assert!(!is_thinking_enabled(&params));
    }

    #[test]
    fn test_strip_thinking_blocks_when_no_reasoning_support() {
        let model = make_model_record(None);
        let params = make_sampling_params();
        let msgs = vec![ChatMessage {
            thinking_blocks: Some(vec![serde_json::json!({"type": "thinking"})]),
            content: ChatContent::SimpleText("hello".into()),
            ..Default::default()
        }];
        let result = strip_thinking_blocks_if_disabled(msgs, &params, &model);
        assert!(result[0].thinking_blocks.is_none());
    }

    #[test]
    fn test_strip_thinking_blocks_when_thinking_disabled() {
        let model = make_model_record(Some("anthropic"));
        let params = make_sampling_params();
        let msgs = vec![ChatMessage {
            thinking_blocks: Some(vec![serde_json::json!({"type": "thinking"})]),
            content: ChatContent::SimpleText("hello".into()),
            ..Default::default()
        }];
        let result = strip_thinking_blocks_if_disabled(msgs, &params, &model);
        assert!(result[0].thinking_blocks.is_none());
    }

    #[test]
    fn test_strip_thinking_blocks_preserves_when_enabled() {
        let model = make_model_record(Some("anthropic"));
        let mut params = make_sampling_params();
        params.thinking = Some(serde_json::json!({"type": "enabled", "budget_tokens": 1024}));
        let msgs = vec![ChatMessage {
            thinking_blocks: Some(vec![serde_json::json!({"type": "thinking"})]),
            content: ChatContent::SimpleText("hello".into()),
            ..Default::default()
        }];
        let result = strip_thinking_blocks_if_disabled(msgs, &params, &model);
        assert!(result[0].thinking_blocks.is_some());
    }

    #[test]
    fn test_strip_thinking_blocks_preserves_other_fields() {
        let model = make_model_record(None);
        let params = make_sampling_params();
        let msgs = vec![ChatMessage {
            role: "assistant".into(),
            content: ChatContent::SimpleText("hello".into()),
            reasoning_content: Some("reasoning".into()),
            thinking_blocks: Some(vec![serde_json::json!({"type": "thinking"})]),
            citations: vec![serde_json::json!({"url": "http://x"})],
            ..Default::default()
        }];
        let result = strip_thinking_blocks_if_disabled(msgs, &params, &model);
        assert_eq!(result[0].role, "assistant");
        assert_eq!(result[0].reasoning_content, Some("reasoning".into()));
        assert_eq!(result[0].citations.len(), 1);
        assert!(result[0].thinking_blocks.is_none());
    }

    #[test]
    fn test_adapt_sampling_openai_boost_reasoning() {
        let mut params = make_sampling_params();
        params.boost_reasoning = true;
        let model = make_model_record(Some("openai"));
        adapt_sampling_for_reasoning_models(&mut params, &model);
        assert_eq!(params.reasoning_effort, Some(ReasoningEffort::Medium));
        assert_eq!(params.temperature, Some(0.7));
    }

    #[test]
    fn test_adapt_sampling_openai_doubles_tokens() {
        let mut params = make_sampling_params();
        params.max_new_tokens = 4096;
        let model = make_model_record(Some("openai"));
        adapt_sampling_for_reasoning_models(&mut params, &model);
        assert_eq!(params.max_new_tokens, 8192);
    }

    #[test]
    fn test_adapt_sampling_openai_no_double_above_8192() {
        let mut params = make_sampling_params();
        params.max_new_tokens = 16384;
        let model = make_model_record(Some("openai"));
        adapt_sampling_for_reasoning_models(&mut params, &model);
        assert_eq!(params.max_new_tokens, 16384);
    }

    #[test]
    fn test_adapt_sampling_anthropic_sets_thinking() {
        let mut params = make_sampling_params();
        params.boost_reasoning = true;
        params.max_new_tokens = 4096;
        let model = make_model_record(Some("anthropic"));
        adapt_sampling_for_reasoning_models(&mut params, &model);
        assert!(params.thinking.is_some());
        let thinking = params.thinking.unwrap();
        assert_eq!(thinking["type"], "enabled");
        assert_eq!(thinking["budget_tokens"], 2048);
        assert!(params.reasoning_effort.is_none());
    }

    #[test]
    fn test_adapt_sampling_anthropic_min_budget() {
        let mut params = make_sampling_params();
        params.boost_reasoning = true;
        params.max_new_tokens = 2048;
        let model = make_model_record(Some("anthropic"));
        adapt_sampling_for_reasoning_models(&mut params, &model);
        let thinking = params.thinking.unwrap();
        assert_eq!(thinking["budget_tokens"], MIN_BUDGET_TOKENS);
    }

    #[test]
    fn test_adapt_sampling_anthropic_no_thinking_if_too_small() {
        let mut params = make_sampling_params();
        params.boost_reasoning = true;
        params.max_new_tokens = 512;
        let model = make_model_record(Some("anthropic"));
        adapt_sampling_for_reasoning_models(&mut params, &model);
        assert!(params.thinking.is_none());
    }

    #[test]
    fn test_adapt_sampling_qwen_enable_thinking() {
        let mut params = make_sampling_params();
        params.boost_reasoning = true;
        let model = make_model_record(Some("qwen"));
        adapt_sampling_for_reasoning_models(&mut params, &model);
        assert_eq!(params.enable_thinking, Some(true));
        assert_eq!(params.temperature, Some(0.7));
    }

    #[test]
    fn test_adapt_sampling_qwen_no_boost() {
        let mut params = make_sampling_params();
        params.boost_reasoning = false;
        let model = make_model_record(Some("qwen"));
        adapt_sampling_for_reasoning_models(&mut params, &model);
        assert_eq!(params.enable_thinking, Some(false));
    }

    #[test]
    fn test_adapt_sampling_no_reasoning_clears_all() {
        let mut params = make_sampling_params();
        params.reasoning_effort = Some(ReasoningEffort::High);
        params.thinking = Some(serde_json::json!({"type": "enabled"}));
        params.enable_thinking = Some(true);
        let model = make_model_record(None);
        adapt_sampling_for_reasoning_models(&mut params, &model);
        assert!(params.reasoning_effort.is_none());
        assert!(params.thinking.is_none());
        assert!(params.enable_thinking.is_none());
    }

    #[test]
    fn test_adapt_sampling_unknown_provider() {
        let mut params = make_sampling_params();
        params.boost_reasoning = true;
        let model = make_model_record(Some("unknown_provider"));
        adapt_sampling_for_reasoning_models(&mut params, &model);
        assert_eq!(params.temperature, Some(0.7));
        assert!(params.reasoning_effort.is_none());
    }

    #[test]
    fn test_chat_prepare_options_default() {
        let opts = ChatPrepareOptions::default();
        assert!(opts.prepend_system_prompt);
        assert!(opts.allow_at_commands);
        assert!(opts.allow_tool_prerun);
        assert!(opts.supports_tools);
        assert!(opts.use_compression);
    }
}
