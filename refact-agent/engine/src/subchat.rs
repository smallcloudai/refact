use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use serde_json::{json, Value};
use tracing::{error, info, warn};

use crate::caps::resolve_chat_model;
use crate::caps::ChatModelRecord;
use crate::tools::tools_description::{tools_merged_and_filtered, tool_description_list_from_yaml};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{SamplingParameters, PostprocessSettings, ChatPost, ChatMessage, ChatUsage, ChatToolCall, ReasoningEffort};
use crate::global_context::{GlobalContext, try_load_caps_quickly_if_not_present};
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::scratchpads::multimodality::chat_content_raw_from_value;
use crate::yaml_configs::customization_loader::load_customization;


const MAX_NEW_TOKENS: usize = 4096;


pub async fn create_chat_post_and_scratchpad(
    global_context: Arc<ARwLock<GlobalContext>>,
    ccx: Arc<AMutex<AtCommandsContext>>,
    model_name: &str,
    messages: Vec<&ChatMessage>,
    temperature: Option<f32>,
    max_new_tokens: usize,
    n: usize,
    reasoning_effort: Option<ReasoningEffort>,
    prepend_system_prompt: bool,
    tools: Option<Vec<Value>>,
    tool_choice: Option<String>,
    only_deterministic_messages: bool,
    _should_execute_remotely: bool,
) -> Result<(ChatPost, Box<dyn ScratchpadAbstract>, Arc<ChatModelRecord>), String> {
    let caps = try_load_caps_quickly_if_not_present(
        global_context.clone(), 0,
    ).await.map_err(|e| {
        warn!("no caps: {:?}", e);
        "no caps".to_string()
    })?;
    let mut error_log = Vec::new();
    let tconfig = load_customization(global_context.clone(), true, &mut error_log).await;
    for e in error_log.iter() {
        tracing::error!("{e}");
    }

    let mut chat_post = ChatPost {
        messages: messages.iter().map(|x|json!(x)).collect(),
        parameters: SamplingParameters {
            max_new_tokens,
            temperature,
            top_p: None,
            stop: vec![],
            n: Some(n),
            reasoning_effort,
            ..Default::default()  // TODO
        },
        model: model_name.to_string(),
        scratchpad: "".to_string(),
        stream: Some(false),
        temperature,
        n: Some(n),
        tools,
        tool_choice,
        only_deterministic_messages,
        subchat_tool_parameters: tconfig.subchat_tool_parameters.clone(),
        postprocess_parameters: PostprocessSettings::new(),
        ..Default::default()
    };

    let model_rec = resolve_chat_model(caps, model_name)?;

    if !model_rec.supports_tools {
        tracing::warn!("supports_tools is false");
    }

    chat_post.max_tokens = Some(model_rec.base.n_ctx);
    (chat_post.scratchpad, _) = model_rec.scratchpads.resolve("")?;

    {
        let mut ccx_locked = ccx.lock().await;
        ccx_locked.current_model = model_name.to_string();
    }

    let scratchpad = crate::scratchpads::create_chat_scratchpad(
        global_context.clone(),
        &mut chat_post,
        &messages.into_iter().cloned().collect::<Vec<_>>(),
        prepend_system_prompt,
        &model_rec,
        false,
    ).await?;

    Ok((chat_post, scratchpad, model_rec))
}

#[allow(dead_code)]
async fn chat_interaction_stream() {
    todo!();
}

async fn chat_interaction_non_stream(
    ccx: Arc<AMutex<AtCommandsContext>>,
    mut spad: Box<dyn ScratchpadAbstract>,
    model_rec: &ChatModelRecord,
    prompt: &String,
    chat_post: &ChatPost,
) -> Result<Vec<Vec<ChatMessage>>, String> {
    let meta = if model_rec.base.support_metadata {
        Some(chat_post.meta.clone())
    } else {
        None
    };
    
    let t1 = std::time::Instant::now();
    let j = crate::restream::scratchpad_interaction_not_stream_json(
        ccx.clone(),
        &mut spad,
        "chat".to_string(),
        prompt,
        &model_rec.base,
        &chat_post.parameters,   // careful: includes n
        chat_post.only_deterministic_messages,
        meta
    ).await.map_err(|e| {
        warn!("network error communicating with the model (2): {:?}", e);
        format!("network error communicating with the model (2): {:?}", e)
    })?;
    info!("non stream generation took {:?}ms", t1.elapsed().as_millis() as i32);

    let usage_mb = j.get("usage")
        .and_then(|value| match value {
            Value::Object(o) => Some(o),
            v => {
                warn!("usage is not a dict: {:?}; Metering is lost", v);
                None
            }
        })
        .and_then(|o| match serde_json::from_value::<ChatUsage>(Value::Object(o.clone())) {
            Ok(usage) => Some(usage),
            Err(e) => {
                warn!("Failed to parse usage object: {:?}; Metering is lost", e);
                None
            }
        });

    let det_messages = if let Some(det_messages) = j.get("deterministic_messages") {
        if let Value::Array(arr) = det_messages {
            let mut d_messages = vec![];
            for a in arr {
                let m = serde_json::from_value(a.clone()).map_err(|e| {
                    warn!("error parsing det message's output: {}", e);
                    format!("error parsing det message's output: {}", e)
                })?;
                d_messages.push(m);
            }
            d_messages
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let mut results = vec![];

    let choices = j.get("choices").and_then(|value| value.as_array()).ok_or("error parsing model's output: choices doesn't exist".to_string())?;
    for choice in choices {
        // XXX: bug 'index' is ignored in scratchpad_interaction_not_stream_json, important when n>1
        let message = choice.get("message").ok_or("error parsing model's output: choice.message doesn't exist".to_string())?;

        // convert choice to a ChatMessage (we don't have code like this in any other place in rust, only in python and typescript)
        let (role, content_value, tool_calls, tool_call_id) = {
            (
                message.get("role")
                    .and_then(|v| v.as_str())
                    .ok_or("error parsing model's output: choice0.message.role doesn't exist or is not a string".to_string())?.to_string(),
                message.get("content")
                    .ok_or("error parsing model's output: choice0.message.content doesn't exist".to_string())?
                    .clone(),
                message.get("tool_calls")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| {
                        serde_json::from_value::<Vec<ChatToolCall>>(Value::Array(arr.clone()))
                            .map_err(|_| "error parsing model's output: choice0.message.tool_calls is not a valid ChatToolCall array".to_string())
                            .ok()
                    }),
                message.get("tool_call_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("").to_string()
            )
        };

        let content = chat_content_raw_from_value(content_value).and_then(|c|c.to_internal_format())
            .map_err(|e| format!("error parsing model's output: {}", e))?;

        let mut ch_results = vec![];
        let msg = ChatMessage {
            role,
            content,
            tool_calls,
            tool_call_id,
            usage: usage_mb.clone(),
            ..Default::default()
        };
        ch_results.extend(det_messages.clone());
        ch_results.push(msg);
        results.push(ch_results)
    }
    if results.is_empty() && !det_messages.is_empty() {
        results.push(det_messages);
    }

    Ok(results)
}


pub async fn chat_interaction(
    ccx: Arc<AMutex<AtCommandsContext>>,
    mut spad: Box<dyn ScratchpadAbstract>,
    model_rec: &ChatModelRecord,
    chat_post: &mut ChatPost,
) -> Result<Vec<Vec<ChatMessage>>, String> {
    let prompt = spad.prompt(ccx.clone(), &mut chat_post.parameters).await?;
    let stream = chat_post.stream.unwrap_or(false);
    if stream {
        warn!("subchats doesn't support streaming, fallback to non-stream communications");
    }
    Ok(chat_interaction_non_stream(
        ccx.clone(),
        spad,
        model_rec,
        &prompt,
        chat_post,
    ).await?)
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
    model_name: &str,
    messages: Vec<ChatMessage>,
    tools_subset: Option<Vec<String>>,
    tool_choice: Option<String>,
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
    let (gcx, should_execute_remotely) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.global_context.clone(), ccx_locked.should_execute_remotely)
    };
    let tools_turned_on_by_cmdline = tools_merged_and_filtered(gcx.clone(), false).await?;
    let tools_turned_on_by_cmdline_set: HashSet<String> = tools_turned_on_by_cmdline.keys().cloned().collect();
    let tools_on_intersection: Vec<String> = if let Some(tools_s) = &tools_subset {
        let tools_turn_on_set: HashSet<String> = tools_s.iter().cloned().collect();
        tools_turn_on_set.intersection(&tools_turned_on_by_cmdline_set).cloned().collect()
    } else {
        tools_turned_on_by_cmdline_set.iter().cloned().collect()
    };
    let allow_experimental = gcx.read().await.cmdline.experimental;
    let tools_desclist = tool_description_list_from_yaml(tools_turned_on_by_cmdline, Some(&tools_on_intersection), allow_experimental).await.unwrap_or_else(|e|{
        error!("Error loading compiled_in_tools: {:?}", e);
        vec![]
    });
    let tools = tools_desclist.into_iter().filter(|x| x.is_supported_by(model_name)).map(|x|x.into_openai_style()).collect::<Vec<_>>();
    info!("tools_subset {:?}", tools_subset);
    info!("tools_turned_on_by_cmdline_set {:?}", tools_turned_on_by_cmdline_set);
    info!("tools_on_intersection {:?}", tools_on_intersection);

    let max_new_tokens = max_new_tokens.unwrap_or(MAX_NEW_TOKENS);
    let (mut chat_post, spad, model_rec) = create_chat_post_and_scratchpad(
        gcx.clone(),
        ccx.clone(),
        model_name,
        messages.iter().collect::<Vec<_>>(),
        temperature,
        max_new_tokens,
        n,
        reasoning_effort,
        prepend_system_prompt,
        Some(tools),
        tool_choice.clone(),
        only_deterministic_messages,
        should_execute_remotely,
    ).await?;

    let chat_response_msgs = chat_interaction(ccx.clone(), spad, &model_rec, &mut chat_post).await?;

    let old_messages = messages.clone();
    // no need to remove user from old_messages here, because allow_at is false

    let results = chat_response_msgs.iter().map(|new_msgs| {
        let mut extended_msgs = old_messages.clone();
        extended_msgs.extend(new_msgs.clone());
        extended_msgs
    }).collect::<Vec<Vec<ChatMessage>>>();

    if let Some(usage_collector) = usage_collector_mb {
        update_usage_from_messages(usage_collector, &results);
    }

    if let Some(tx_chatid) = tx_chatid_mb {
        assert!(tx_toolid_mb.is_some());
        let tx_toolid = tx_toolid_mb.unwrap();
        let subchat_tx = ccx.lock().await.subchat_tx.clone();
        for (i, choice) in chat_response_msgs.iter().enumerate() {
            // XXX: ...-choice will not work to store in chat_client.py
            let cid = if chat_response_msgs.len() > 1 {
                format!("{}-choice{}", tx_chatid, i)
            } else {
                tx_chatid.clone()
            };
            for msg_in_choice in choice {
                let message = serde_json::json!({"tool_call_id": tx_toolid, "subchat_id": cid, "add_message": msg_in_choice});
                let _ = subchat_tx.lock().await.send(message);
            }
        }
    }

    Ok(results)
}

pub async fn subchat(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model_name: &str,
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
                model_name,
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
            step_n += 1;
        }
        // result => session
    }
    let last_message = messages.last().unwrap();
    if let Some(tool_calls) = &last_message.tool_calls {
        if !tool_calls.is_empty() {
            messages = subchat_single(
                ccx.clone(),
                model_name,
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
        model_name,
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
                    model_name,
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
