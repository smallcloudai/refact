use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use serde_json::json;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tracing::{info, warn};
use uuid::Uuid;
use futures::StreamExt;
use reqwest_eventsource::Event;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ChatMeta, ChatMode, ChatUsage, SamplingParameters};
use crate::global_context::GlobalContext;
use crate::scratchpad_abstract::{FinishReason, HasTokenizerAndEot};
use crate::constants::CHAT_TOP_N;
use crate::http::routers::v1::knowledge_enrichment::enrich_messages_with_knowledge;

use super::types::*;
use super::openai_merge::merge_tool_call;
use super::trajectories::{maybe_save_trajectory, check_external_reload_pending};
use super::tools::check_tool_calls_and_continue;
use super::prepare::{prepare_chat_passthrough, ChatPrepareOptions};
use super::prompts::prepend_the_right_system_prompt_and_maybe_more_initial_messages;

pub fn parse_chat_mode(mode: &str) -> ChatMode {
    match mode.to_uppercase().as_str() {
        "AGENT" => ChatMode::AGENT,
        "NO_TOOLS" => ChatMode::NO_TOOLS,
        "EXPLORE" => ChatMode::EXPLORE,
        "CONFIGURE" => ChatMode::CONFIGURE,
        "PROJECT_SUMMARY" => ChatMode::PROJECT_SUMMARY,
        _ => ChatMode::AGENT,
    }
}

pub fn start_generation(
    gcx: Arc<ARwLock<GlobalContext>>,
    session_arc: Arc<AMutex<ChatSession>>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
    Box::pin(async move {
        let (messages, thread, chat_id) = {
            let session = session_arc.lock().await;
            (session.messages.clone(), session.thread.clone(), session.chat_id.clone())
        };

        let abort_flag = {
            let mut session = session_arc.lock().await;
            match session.start_stream() {
                Some((_message_id, abort_flag)) => abort_flag,
                None => {
                    warn!("Cannot start generation for {}: already generating", chat_id);
                    return;
                }
            }
        };

        if let Err(e) = run_llm_generation(gcx.clone(), session_arc.clone(), messages, thread, chat_id.clone(), abort_flag).await {
            let mut session = session_arc.lock().await;
            if !session.abort_flag.load(Ordering::SeqCst) {
                session.finish_stream_with_error(e);
            }
        }

        maybe_save_trajectory(gcx.clone(), session_arc.clone()).await;

        {
            let session = session_arc.lock().await;
            session.queue_notify.notify_one();
        }
    })
}

pub async fn run_llm_generation(
    gcx: Arc<ARwLock<GlobalContext>>,
    session_arc: Arc<AMutex<ChatSession>>,
    messages: Vec<ChatMessage>,
    thread: ThreadParams,
    chat_id: String,
    abort_flag: Arc<AtomicBool>,
) -> Result<(), String> {
    let chat_mode = parse_chat_mode(&thread.mode);

    let mut messages = messages;
    let last_is_user = messages.last().map(|m| m.role == "user").unwrap_or(false);
    if chat_mode == ChatMode::AGENT && last_is_user {
        let _ = enrich_messages_with_knowledge(gcx.clone(), &mut messages).await;
    }

    let tools: Vec<crate::tools::tools_description::ToolDesc> =
        crate::tools::tools_list::get_available_tools_by_chat_mode(gcx.clone(), chat_mode).await
            .into_iter()
            .map(|tool| tool.tool_description())
            .collect();

    info!("session generation: tools count = {}", tools.len());

    let caps = crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0).await
        .map_err(|e| e.message)?;
    let model_rec = crate::caps::resolve_chat_model(caps, &thread.model)?;

    let effective_n_ctx = thread.context_tokens_cap.unwrap_or(model_rec.base.n_ctx);
    let tokenizer_arc = crate::tokens::cached_tokenizer(gcx.clone(), &model_rec.base).await?;
    let t = HasTokenizerAndEot::new(tokenizer_arc);

    let meta = ChatMeta {
        chat_id: chat_id.clone(),
        chat_mode,
        chat_remote: false,
        current_config_file: String::new(),
        context_tokens_cap: thread.context_tokens_cap,
        include_project_info: thread.include_project_info,
        request_attempt_id: Uuid::new_v4().to_string(),
        use_compression: thread.use_compression,
    };

    let session_has_system = {
        let session = session_arc.lock().await;
        session.messages.first().map(|m| m.role == "system").unwrap_or(false)
    };

    if !session_has_system {
        let tool_names: std::collections::HashSet<String> = tools.iter()
            .map(|t| t.name.clone())
            .collect();
        let mut has_rag_results = crate::scratchpads::scratchpad_utils::HasRagResults::new();
        let messages_with_system = prepend_the_right_system_prompt_and_maybe_more_initial_messages(
            gcx.clone(),
            messages.clone(),
            &meta,
            &mut has_rag_results,
            tool_names,
        ).await;

        let prepended_count = messages_with_system.len().saturating_sub(messages.len());
        if prepended_count > 0 {
            let mut session = session_arc.lock().await;
            for (i, msg) in messages_with_system.iter().take(prepended_count).enumerate() {
                session.messages.insert(i, msg.clone());
                session.emit(ChatEvent::MessageAdded {
                    message: msg.clone(),
                    index: i,
                });
            }
            session.increment_version();
            info!("Saved {} prepended messages to session at index 0", prepended_count);
        }
        messages = messages_with_system;
    }

    let mut parameters = SamplingParameters {
        temperature: Some(0.0),
        max_new_tokens: 4096.min(effective_n_ctx / 4),
        boost_reasoning: thread.boost_reasoning,
        ..Default::default()
    };

    let ccx = AtCommandsContext::new(
        gcx.clone(),
        effective_n_ctx,
        CHAT_TOP_N,
        false,
        messages.clone(),
        chat_id.clone(),
        false,
        model_rec.base.id.clone(),
    ).await;
    let ccx_arc = Arc::new(AMutex::new(ccx));

    let options = ChatPrepareOptions {
        prepend_system_prompt: false,
        allow_at_commands: true,
        allow_tool_prerun: true,
        supports_tools: model_rec.supports_tools,
        use_compression: thread.use_compression,
    };

    let prepared = prepare_chat_passthrough(
        gcx.clone(),
        ccx_arc.clone(),
        &t,
        messages,
        &model_rec.base.id,
        tools,
        &meta,
        &mut parameters,
        &options,
        &None,
    ).await?;

    run_streaming_generation(
        gcx,
        session_arc,
        prepared.prompt,
        model_rec.base.clone(),
        parameters,
        abort_flag,
        chat_mode,
    ).await
}

async fn run_streaming_generation(
    gcx: Arc<ARwLock<GlobalContext>>,
    session_arc: Arc<AMutex<ChatSession>>,
    prompt: String,
    model_rec: crate::caps::BaseModelRecord,
    parameters: SamplingParameters,
    abort_flag: Arc<AtomicBool>,
    chat_mode: ChatMode,
) -> Result<(), String> {
    info!("session generation: prompt length = {}", prompt.len());

    let (client, slowdown_arc) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.http_client.clone(), gcx_locked.http_client_slowdown.clone())
    };

    let _ = slowdown_arc.acquire().await;

    let (chat_id, context_tokens_cap, include_project_info, use_compression) = {
        let session = session_arc.lock().await;
        (
            session.chat_id.clone(),
            session.thread.context_tokens_cap,
            session.thread.include_project_info,
            session.thread.use_compression,
        )
    };

    let meta = Some(ChatMeta {
        chat_id,
        chat_mode,
        chat_remote: false,
        current_config_file: String::new(),
        context_tokens_cap,
        include_project_info,
        request_attempt_id: Uuid::new_v4().to_string(),
        use_compression,
    });

    let mut event_source = crate::forward_to_openai_endpoint::forward_to_openai_style_endpoint_streaming(
        &model_rec,
        &prompt,
        &client,
        &parameters,
        meta,
    ).await.map_err(|e| format!("Failed to connect to LLM: {}", e))?;

    let mut accumulated_content = String::new();
    let mut accumulated_reasoning = String::new();
    let mut accumulated_thinking_blocks: Vec<serde_json::Value> = Vec::new();
    let mut accumulated_tool_calls: Vec<serde_json::Value> = Vec::new();
    let mut accumulated_citations: Vec<serde_json::Value> = Vec::new();
    let mut accumulated_extra: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    let mut last_finish_reason = FinishReason::None;

    let stream_started_at = Instant::now();
    let mut last_event_at = Instant::now();
    let mut heartbeat = tokio::time::interval(STREAM_HEARTBEAT);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        let event = tokio::select! {
            _ = heartbeat.tick() => {
                if abort_flag.load(Ordering::SeqCst) {
                    info!("Generation aborted by user");
                    return Err("Aborted".to_string());
                }
                if stream_started_at.elapsed() > STREAM_TOTAL_TIMEOUT {
                    return Err("LLM stream timeout".to_string());
                }
                if last_event_at.elapsed() > STREAM_IDLE_TIMEOUT {
                    return Err("LLM stream stalled".to_string());
                }
                continue;
            }
            maybe_event = event_source.next() => {
                match maybe_event {
                    Some(e) => e,
                    None => break,
                }
            }
        };
        last_event_at = Instant::now();

        match event {
            Ok(Event::Open) => {},
            Ok(Event::Message(msg)) => {
                if msg.data.starts_with("[DONE]") {
                    break;
                }

                let json: serde_json::Value = serde_json::from_str(&msg.data)
                    .map_err(|e| format!("JSON parse error: {}", e))?;

                if let Some(err) = json.get("error") {
                    return Err(format!("LLM error: {}", err));
                }
                if let Some(detail) = json.get("detail") {
                    return Err(format!("LLM error: {}", detail));
                }

                let mut changed_extra = serde_json::Map::new();
                if let Some(obj) = json.as_object() {
                    for (key, val) in obj {
                        if val.is_null() {
                            continue;
                        }
                        let dominated = key.starts_with("metering_")
                            || key.starts_with("billing_")
                            || key.starts_with("cost_")
                            || key.starts_with("cache_")
                            || key == "system_fingerprint";
                        if dominated && accumulated_extra.get(key) != Some(val) {
                            accumulated_extra.insert(key.clone(), val.clone());
                            changed_extra.insert(key.clone(), val.clone());
                        }
                    }
                }
                if let Some(psf) = json.get("provider_specific_fields") {
                    if !psf.is_null() && accumulated_extra.get("provider_specific_fields") != Some(psf) {
                        accumulated_extra.insert("provider_specific_fields".to_string(), psf.clone());
                        changed_extra.insert("provider_specific_fields".to_string(), psf.clone());
                    }
                }

                let delta = match json.get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|c| c.get("delta"))
                {
                    Some(d) => d,
                    None => continue,
                };

                if let Some(fr) = json.get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|c| c.get("finish_reason"))
                {
                    last_finish_reason = FinishReason::from_json_val(fr).unwrap_or(FinishReason::None);
                }

                let mut ops = Vec::new();

                if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                    if !content.is_empty() {
                        accumulated_content.push_str(content);
                        ops.push(DeltaOp::AppendContent { text: content.to_string() });
                    }
                }

                if let Some(reasoning) = delta.get("reasoning_content").and_then(|c| c.as_str()) {
                    if !reasoning.is_empty() {
                        accumulated_reasoning.push_str(reasoning);
                        ops.push(DeltaOp::AppendReasoning { text: reasoning.to_string() });
                    }
                }

                if let Some(tool_calls) = delta.get("tool_calls").and_then(|tc| tc.as_array()) {
                    for tc in tool_calls {
                        merge_tool_call(&mut accumulated_tool_calls, tc.clone());
                    }
                    if !accumulated_tool_calls.is_empty() {
                        ops.push(DeltaOp::SetToolCalls { tool_calls: accumulated_tool_calls.clone() });
                    }
                }

                let thinking_blocks_raw = delta.get("thinking_blocks").and_then(|tb| tb.as_array())
                    .or_else(|| delta.get("provider_specific_fields")
                        .and_then(|psf| psf.get("thinking_blocks"))
                        .and_then(|tb| tb.as_array()))
                    .or_else(|| json.get("provider_specific_fields")
                        .and_then(|psf| psf.get("thinking_blocks"))
                        .and_then(|tb| tb.as_array()));

                if let Some(thinking) = thinking_blocks_raw {
                    let normalized: Vec<serde_json::Value> = thinking.iter().map(|block| {
                        if block.get("thinking").is_some() {
                            block.clone()
                        } else if let Some(text) = block.get("text") {
                            json!({
                                "type": "thinking",
                                "thinking": text,
                                "signature": block.get("signature").cloned()
                            })
                        } else if let Some(content) = block.get("content") {
                            json!({
                                "type": "thinking",
                                "thinking": content,
                                "signature": block.get("signature").cloned()
                            })
                        } else if block.is_string() {
                            json!({
                                "type": "thinking",
                                "thinking": block,
                                "signature": null
                            })
                        } else {
                            block.clone()
                        }
                    }).collect();
                    accumulated_thinking_blocks = normalized.clone();
                    ops.push(DeltaOp::SetThinkingBlocks { blocks: normalized });
                }

                if let Some(usage) = json.get("usage") {
                    if !usage.is_null() {
                        ops.push(DeltaOp::SetUsage { usage: usage.clone() });
                        if let Ok(parsed_usage) = serde_json::from_value::<ChatUsage>(usage.clone()) {
                            let mut session = session_arc.lock().await;
                            session.draft_usage = Some(parsed_usage);
                        }
                    }
                }

                if let Some(citation) = json.get("provider_specific_fields")
                    .and_then(|psf| psf.get("citation"))
                {
                    if !citation.is_null() {
                        accumulated_citations.push(citation.clone());
                        ops.push(DeltaOp::AddCitation { citation: citation.clone() });
                    }
                }
                if let Some(citation) = delta.get("provider_specific_fields")
                    .and_then(|psf| psf.get("citation"))
                {
                    if !citation.is_null() {
                        accumulated_citations.push(citation.clone());
                        ops.push(DeltaOp::AddCitation { citation: citation.clone() });
                    }
                }

                if !changed_extra.is_empty() {
                    ops.push(DeltaOp::MergeExtra { extra: changed_extra });
                }

                if !ops.is_empty() {
                    let mut session = session_arc.lock().await;
                    session.emit_stream_delta(ops);
                }
            }
            Err(e) => {
                return Err(format!("Stream error: {}", e));
            }
        }
    }
    drop(heartbeat);

    {
        let mut session = session_arc.lock().await;

        if let Some(ref mut draft) = session.draft_message {
            draft.content = ChatContent::SimpleText(accumulated_content);
            if !accumulated_tool_calls.is_empty() {
                info!("Parsing {} accumulated tool calls", accumulated_tool_calls.len());

                let parsed_tool_calls: Vec<crate::call_validation::ChatToolCall> = accumulated_tool_calls
                    .iter()
                    .filter_map(|tc| normalize_tool_call(tc))
                    .collect();

                info!("Successfully parsed {} tool calls", parsed_tool_calls.len());
                if !parsed_tool_calls.is_empty() {
                    draft.tool_calls = Some(parsed_tool_calls);
                }
            }

            if !accumulated_reasoning.is_empty() {
                draft.reasoning_content = Some(accumulated_reasoning.clone());
            }
            if !accumulated_thinking_blocks.is_empty() {
                draft.thinking_blocks = Some(accumulated_thinking_blocks.clone());
            }
            if !accumulated_citations.is_empty() {
                draft.citations = accumulated_citations.clone();
            }
            if !accumulated_extra.is_empty() {
                draft.extra = accumulated_extra.clone();
            }
        }

        let finish_reason_str = match last_finish_reason {
            FinishReason::Stop | FinishReason::ScratchpadStop => Some("stop".to_string()),
            FinishReason::Length => Some("length".to_string()),
            FinishReason::None => None,
        };
        session.finish_stream(finish_reason_str);
    }

    check_tool_calls_and_continue(gcx.clone(), session_arc.clone(), chat_mode).await;
    check_external_reload_pending(gcx, session_arc).await;

    Ok(())
}

fn normalize_tool_call(tc: &serde_json::Value) -> Option<crate::call_validation::ChatToolCall> {
    let function = tc.get("function")?;
    let name = function.get("name").and_then(|n| n.as_str()).filter(|s| !s.is_empty())?;

    let id = tc.get("id")
        .and_then(|i| i.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("call_{}", Uuid::new_v4().to_string().replace("-", "")[..24].to_string()));

    let arguments = match function.get("arguments") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(v) if !v.is_null() => serde_json::to_string(v).unwrap_or_default(),
        _ => String::new(),
    };

    let tool_type = tc.get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("function")
        .to_string();

    let index = tc.get("index").and_then(|i| i.as_u64()).map(|i| i as usize);

    Some(crate::call_validation::ChatToolCall {
        id,
        index,
        function: crate::call_validation::ChatToolFunction {
            name: name.to_string(),
            arguments,
        },
        tool_type,
    })
}
