use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use serde_json::{json, Value};
use tracing::info;
use uuid::Uuid;

use crate::caps::resolve_chat_model;
use crate::tools::tools_description::ToolDesc;
use crate::tools::tools_list::get_available_tools;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMeta, ChatMode, SamplingParameters, ChatMessage, ChatUsage, ChatToolCall, ReasoningEffort};
use crate::global_context::try_load_caps_quickly_if_not_present;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::scratchpads::multimodality::chat_content_raw_from_value;
use crate::chat::prepare::{prepare_chat_passthrough, ChatPrepareOptions};


const MAX_NEW_TOKENS: usize = 4096;


async fn subchat_non_stream(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model_id: &str,
    messages: Vec<ChatMessage>,
    tools: Vec<ToolDesc>,
    prepend_system_prompt: bool,
    temperature: Option<f32>,
    max_new_tokens: usize,
    n: usize,
    reasoning_effort: Option<ReasoningEffort>,
    only_deterministic_messages: bool,
) -> Result<Vec<Vec<ChatMessage>>, String> {
    let gcx = {
        let ccx_locked = ccx.lock().await;
        ccx_locked.global_context.clone()
    };

    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await
        .map_err(|e| format!("no caps: {:?}", e))?;
    let model_rec = resolve_chat_model(caps, model_id)?;

    let tokenizer_arc = crate::tokens::cached_tokenizer(gcx.clone(), &model_rec.base).await?;
    let t = HasTokenizerAndEot::new(tokenizer_arc);

    let meta = ChatMeta {
        chat_id: Uuid::new_v4().to_string(),
        chat_mode: ChatMode::AGENT,
        chat_remote: false,
        current_config_file: String::new(),
        context_tokens_cap: Some(model_rec.base.n_ctx),
        include_project_info: true,
        request_attempt_id: Uuid::new_v4().to_string(),
        use_compression: false,
    };

    let mut parameters = SamplingParameters {
        max_new_tokens,
        temperature,
        n: Some(n),
        reasoning_effort,
        ..Default::default()
    };

    let options = ChatPrepareOptions {
        prepend_system_prompt,
        allow_at_commands: false,
        allow_tool_prerun: false,
        supports_tools: model_rec.supports_tools,
        use_compression: false,
    };

    if only_deterministic_messages {
        return Ok(vec![messages]);
    }

    let prepared = prepare_chat_passthrough(
        gcx.clone(),
        ccx.clone(),
        &t,
        messages.clone(),
        model_id,
        tools,
        &meta,
        &mut parameters,
        &options,
        &None,
    ).await?;

    let (client, slowdown_arc) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.http_client.clone(), gcx_locked.http_client_slowdown.clone())
    };

    let _ = slowdown_arc.acquire().await;

    let t1 = std::time::Instant::now();
    let j = crate::forward_to_openai_endpoint::forward_to_openai_style_endpoint(
        &model_rec.base,
        &prepared.prompt,
        &client,
        &parameters,
        if model_rec.base.support_metadata { Some(meta) } else { None },
    ).await.map_err(|e| format!("network error: {:?}", e))?;
    info!("non stream generation took {:?}ms", t1.elapsed().as_millis() as i32);

    parse_llm_response(&j, messages)
}

fn parse_llm_response(j: &Value, original_messages: Vec<ChatMessage>) -> Result<Vec<Vec<ChatMessage>>, String> {
    if let Some(err) = j.get("error") {
        return Err(format!("model error: {}", err));
    }
    if let Some(msg) = j.get("detail") {
        return Err(format!("model error: {}", msg));
    }
    if let Some(msg) = j.get("human_readable_message") {
        return Err(format!("model error: {}", msg));
    }

    let usage_mb = j.get("usage")
        .and_then(|value| value.as_object())
        .and_then(|o| serde_json::from_value::<ChatUsage>(Value::Object(o.clone())).ok());

    let choices = j.get("choices")
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("error parsing model's output: choices doesn't exist, response: {}", j))?;

    if choices.is_empty() {
        return Ok(vec![original_messages]);
    }

    let mut indexed_choices: Vec<(usize, &Value)> = choices.iter()
        .map(|c| {
            let idx = c.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
            (idx, c)
        })
        .collect();
    indexed_choices.sort_by_key(|(idx, _)| *idx);

    let mut results = vec![];
    for (_, choice) in indexed_choices {
        let message = choice.get("message")
            .ok_or("error parsing model's output: choice.message doesn't exist")?;

        let role = message.get("role")
            .and_then(|v| v.as_str())
            .ok_or("error parsing model's output: role doesn't exist")?.to_string();

        let content_value = message.get("content").cloned().unwrap_or(json!(null));
        let content = chat_content_raw_from_value(content_value)
            .and_then(|c| c.to_internal_format())
            .map_err(|e| format!("error parsing model's output: {}", e))?;

        let tool_calls = message.get("tool_calls")
            .and_then(|v| v.as_array())
            .and_then(|arr| serde_json::from_value::<Vec<ChatToolCall>>(Value::Array(arr.clone())).ok());

        let tool_call_id = message.get("tool_call_id")
            .and_then(|v| v.as_str())
            .unwrap_or("").to_string();

        let thinking_blocks = message.get("thinking_blocks")
            .and_then(|v| v.as_array())
            .cloned();

        let msg = ChatMessage {
            role,
            content,
            tool_calls,
            tool_call_id,
            thinking_blocks,
            usage: usage_mb.clone(),
            ..Default::default()
        };

        let mut extended = original_messages.clone();
        extended.push(msg);
        results.push(extended);
    }

    if results.is_empty() {
        results.push(original_messages);
    }

    Ok(results)
}

fn update_usage_from_messages(usage: &mut ChatUsage, messages: &Vec<Vec<ChatMessage>>) {
    // even if n_choices > 1, usage is identical in each Vec<ChatMessage>, so we could take the first one
    if let Some(message_0) = messages.get(0) {
        if let Some(last_message) = message_0.last() {
            if let Some(u) = last_message.usage.as_ref() {
                usage.total_tokens += u.total_tokens;
                usage.completion_tokens += u.completion_tokens;
                usage.prompt_tokens += u.prompt_tokens;
            }
        }
    }
}

pub async fn subchat_single(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model_id: &str,
    messages: Vec<ChatMessage>,
    tools_subset: Option<Vec<String>>,
    _tool_choice: Option<String>,
    only_deterministic_messages: bool,
    temperature: Option<f32>,
    max_new_tokens: Option<usize>,
    n: usize,
    reasoning_effort: Option<ReasoningEffort>,
    prepend_system_prompt: bool,
    usage_collector_mb: Option<&mut ChatUsage>,
    tx_toolid_mb: Option<String>,
    tx_chatid_mb: Option<String>,
) -> Result<Vec<Vec<ChatMessage>>, String> {
    let gcx = {
        let ccx_locked = ccx.lock().await;
        ccx_locked.global_context.clone()
    };

    info!("tools_subset {:?}", tools_subset);

    let tools_desclist: Vec<ToolDesc> = {
        let tools_turned_on_by_cmdline = get_available_tools(gcx.clone()).await.iter().map(|tool| {
            tool.tool_description()
        }).collect::<Vec<_>>();

        info!("tools_turned_on_by_cmdline {:?}", tools_turned_on_by_cmdline.iter().map(|tool| {
            &tool.name
        }).collect::<Vec<_>>());

        match tools_subset {
            Some(ref tools_subset) => {
                tools_turned_on_by_cmdline.into_iter().filter(|tool| {
                    tools_subset.contains(&tool.name)
                }).collect()
            }
            None => tools_turned_on_by_cmdline,
        }
    };

    info!("tools_on_intersection {:?}", tools_desclist.iter().map(|tool| {
        &tool.name
    }).collect::<Vec<_>>());

    let tools = tools_desclist.into_iter().filter(|x| x.is_supported_by(model_id)).collect::<Vec<_>>();

    let max_new_tokens = max_new_tokens.unwrap_or(MAX_NEW_TOKENS);

    let results = subchat_non_stream(
        ccx.clone(),
        model_id,
        messages.clone(),
        tools,
        prepend_system_prompt,
        temperature,
        max_new_tokens,
        n,
        reasoning_effort,
        only_deterministic_messages,
    ).await?;

    if let Some(usage_collector) = usage_collector_mb {
        update_usage_from_messages(usage_collector, &results);
    }

    if let Some(tx_chatid) = tx_chatid_mb {
        if let Some(tx_toolid) = tx_toolid_mb {
            let subchat_tx = ccx.lock().await.subchat_tx.clone();
            for (i, choice) in results.iter().enumerate() {
                let cid = if results.len() > 1 {
                    format!("{}-choice{}", tx_chatid, i)
                } else {
                    tx_chatid.clone()
                };
                if let Some(last_msg) = choice.last() {
                    let message = json!({"tool_call_id": tx_toolid, "subchat_id": cid, "add_message": last_msg});
                    let _ = subchat_tx.lock().await.send(message);
                }
            }
        }
    }

    Ok(results)
}

pub async fn subchat(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model_id: &str,
    messages: Vec<ChatMessage>,
    tools_subset: Vec<String>,
    wrap_up_depth: usize,
    wrap_up_tokens_cnt: usize,
    wrap_up_prompt: &str,
    wrap_up_n: usize,
    temperature: Option<f32>,
    reasoning_effort: Option<ReasoningEffort>,
    tx_toolid_mb: Option<String>,
    tx_chatid_mb: Option<String>,
    prepend_system_prompt: Option<bool>,
) -> Result<Vec<Vec<ChatMessage>>, String> {
    let mut messages = messages.clone();
    let mut usage_collector = ChatUsage { ..Default::default() };
    let mut tx_chatid_mb = tx_chatid_mb.clone();
    // for attempt in attempt_n
    {
        // keep session
        let mut step_n = 0;
        loop {
            let last_message = messages.last().unwrap();
            if last_message.role == "assistant" && last_message.tool_calls.is_none() {
                // don't have tool calls, exit the loop unconditionally, model thinks it has finished the work
                break;
            }
            if last_message.role == "assistant" && last_message.tool_calls.is_some() {
                // have tool calls, let's see if we need to wrap up or not
                if step_n >= wrap_up_depth {
                    break;
                }
                if let Some(usage) = &last_message.usage {
                    if usage.prompt_tokens + usage.completion_tokens > wrap_up_tokens_cnt {
                        break;
                    }
                }
            }
            messages = subchat_single(
                ccx.clone(),
                model_id,
                messages.clone(),
                Some(tools_subset.clone()),
                Some("auto".to_string()),
                false,
                temperature,
                None,
                1,
                reasoning_effort.clone(),
                prepend_system_prompt.unwrap_or(false),
                Some(&mut usage_collector),
                tx_toolid_mb.clone(),
                tx_chatid_mb.clone(),
            ).await?[0].clone();
            let last_message = messages.last().unwrap();
            let mut content = format!("🤖:\n{}", &last_message.content.content_text_only());
            if let Some(tool_calls) = &last_message.tool_calls {
                if let Some(tool_call) = tool_calls.get(0) {
                    content = format!("{}\n{}({})", content, tool_call.function.name, tool_call.function.arguments);
                }
            }
            let tx_chatid = format!("{step_n}/{wrap_up_depth}: {content}");
            info!("subchat request {tx_chatid}");
            tx_chatid_mb = Some(tx_chatid);
            step_n += 1;
        }
        // result => session
    }
    let last_message = messages.last().unwrap();
    if let Some(tool_calls) = &last_message.tool_calls {
        if !tool_calls.is_empty() {
            messages = subchat_single(
                ccx.clone(),
                model_id,
                messages,
                Some(vec![]),
                Some("none".to_string()),
                true,   // <-- only runs tool calls
                temperature,
                None,
                1,
                reasoning_effort.clone(),
                prepend_system_prompt.unwrap_or(false),
                Some(&mut usage_collector),
                tx_toolid_mb.clone(),
                tx_chatid_mb.clone(),
            ).await?[0].clone();
        }
    }
    messages.push(ChatMessage::new("user".to_string(), wrap_up_prompt.to_string()));
    let choices = subchat_single(
        ccx.clone(),
        model_id,
        messages,
        Some(tools_subset.clone()),
        Some("auto".to_string()),
        false,
        temperature,
        None,
        wrap_up_n,
        reasoning_effort.clone(),
        prepend_system_prompt.unwrap_or(false),
        Some(&mut usage_collector),
        tx_toolid_mb.clone(),
        tx_chatid_mb.clone(),
    ).await?;
    for messages in choices.iter() {
        let last_message = messages.last().unwrap();
        if let Some(tool_calls) = &last_message.tool_calls {
            if !tool_calls.is_empty() {
                _ = subchat_single(
                    ccx.clone(),
                    model_id,
                    messages.clone(),
                    Some(vec![]),
                    Some("none".to_string()),
                    true,   // <-- only runs tool calls
                    temperature,
                    None,
                    1,
                    reasoning_effort.clone(),
                    prepend_system_prompt.unwrap_or(false),
                    Some(&mut usage_collector),
                    tx_toolid_mb.clone(),
                    tx_chatid_mb.clone(),
                ).await?[0].clone();
            }
        }

    }
    // if let Some(last_message) = messages.last_mut() {
    //     last_message.usage = Some(usage_collector);
    // }
    Ok(choices)
}
