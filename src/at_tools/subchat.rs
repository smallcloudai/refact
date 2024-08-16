use std::sync::Arc;
use std::collections::HashSet;
use std::fs::{self};
use textwrap::wrap;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use serde_json::Value;
use tracing::{error, info, warn};
use crate::at_tools::tools::{at_tools_merged_and_filtered, tools_compiled_in};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatPost, ChatToolCall, ChatUsage, SamplingParameters};
use crate::global_context::{GlobalContext, try_load_caps_quickly_if_not_present};
use crate::http::routers::v1::chat::lookup_chat_scratchpad;
use crate::scratchpad_abstract::ScratchpadAbstract;


const TEMPERATURE: f32 = 0.2;
const MAX_NEW_TOKENS: usize = 4096;


async fn create_chat_post_and_scratchpad(
    global_context: Arc<ARwLock<GlobalContext>>,
    model_name: &str,
    messages: Vec<&ChatMessage>,
    temperature: Option<f32>,
    max_new_tokens: usize,
    n: usize,
    tools: Option<Vec<Value>>,
    tool_choice: Option<String>,
    only_deterministic_messages: bool,
) -> Result<(ChatPost, Box<dyn ScratchpadAbstract>), String> {
    let caps = try_load_caps_quickly_if_not_present(
        global_context.clone(), 0,
    ).await.map_err(|e| {
        warn!("no caps: {:?}", e);
        "no caps".to_string()
    })?;

    let mut chat_post = ChatPost {
        messages: messages.iter().cloned().cloned().collect::<Vec<_>>(),
        parameters: SamplingParameters {
            max_new_tokens,
            temperature,
            top_p: None,
            stop: vec![],
            n: Some(n),
        },
        model: model_name.to_string(),
        scratchpad: "".to_string(),
        stream: Some(false),
        temperature,
        max_tokens: 0,
        n: Some(n),
        tools,
        tool_choice,
        only_deterministic_messages,
        chat_id: "".to_string(),
    };

    let (model_name, scratchpad_name, scratchpad_patch, n_ctx, supports_tools) = lookup_chat_scratchpad(
        caps.clone(),
        &chat_post,
    ).await?;

    if !supports_tools {
        warn!("supports_tools is false");
    }

    chat_post.max_tokens = n_ctx;
    chat_post.scratchpad = scratchpad_name.clone();

    let scratchpad = crate::scratchpads::create_chat_scratchpad(
        global_context.clone(),
        caps,
        model_name.to_string(),
        &chat_post,
        &scratchpad_name,
        &scratchpad_patch,
        false,
        supports_tools,
    ).await?;

    Ok((chat_post, scratchpad))
}

#[allow(dead_code)]
async fn chat_interaction_stream() {
    todo!();
}

async fn chat_interaction_non_stream(
    ccx: Arc<AMutex<AtCommandsContext>>,
    mut spad: Box<dyn ScratchpadAbstract>,
    prompt: &String,
    chat_post: &ChatPost,
) -> Result<Vec<Vec<ChatMessage>>, String> {
    let t1 = std::time::Instant::now();
    let j = crate::restream::scratchpad_interaction_not_stream_json(
        ccx.clone(),
        &mut spad,
        "chat".to_string(),
        prompt,
        chat_post.model.clone(),
        &chat_post.parameters,   // careful: includes n
        chat_post.only_deterministic_messages,
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

    let det_messages = j.get("deterministic_messages")
        .and_then(|value| value.as_array())
        .and_then(|arr| {
            serde_json::from_value::<Vec<ChatMessage>>(Value::Array(arr.clone())).ok()
        }).unwrap_or_else(Vec::new);

    let mut results = vec![];

    let choices = j.get("choices").and_then(|value| value.as_array()).ok_or("error parsing model's output: choices doesn't exist".to_string())?;
    for choice in choices {
        // XXX: bug 'index' is ignored in scratchpad_interaction_not_stream_json, important when n>1
        let message = choice.get("message").ok_or("error parsing model's output: choice.message doesn't exist".to_string())?;

        // convert choice to a ChatMessage (we don't have code like this in any other place in rust, only in python and typescript)
        let (role, content, tool_calls, tool_call_id) = {
            (
                message.get("role")
                    .and_then(|v| v.as_str())
                    .ok_or("error parsing model's output: choice0.message.role doesn't exist or is not a string".to_string())?.to_string(),
                message.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("").to_string(),
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
        let mut ch_results = vec![];
        let msg = ChatMessage {
            role,
            content,
            tool_calls,
            tool_call_id,
            usage: usage_mb.clone(),
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

async fn chat_interaction(
    ccx: Arc<AMutex<AtCommandsContext>>,
    mut spad: Box<dyn ScratchpadAbstract>,
    chat_post: &mut ChatPost,
) -> Result<Vec<Vec<ChatMessage>>, String> {
    let prompt = spad.prompt(ccx.clone(), &mut chat_post.parameters).await?;
    let stream = chat_post.stream.unwrap_or(false);
    return if stream {
        todo!();
    } else {
        Ok(chat_interaction_non_stream(
            ccx.clone(),
            spad,
            &prompt,
            chat_post,
        ).await?)
    }
}

async fn write_dumps(
    gcx: Arc<ARwLock<GlobalContext>>,
    logfn: String,
    content: &str,
) {
    let cache_dir = {
        let gcx_locked = gcx.read().await;
        gcx_locked.cache_dir.clone()
    };
    let dump_dir = cache_dir.join("dumps");
    let _ = fs::create_dir_all(&dump_dir);
    let pathbuf = dump_dir.join(logfn);
    let _ = fs::write(&pathbuf, content);
}

fn format_messages_as_markdown(messages: &[ChatMessage]) -> String {
    const MAX_WIDTH: usize = 100;
    let mut formatted = String::new();

    for message in messages {
        formatted.push_str(&format!("\n\n --------- {} ---------\n\n", message.role.to_uppercase()));
        if message.content.len() > 10*MAX_WIDTH && message.content.matches('\n').count() < 2 {
            formatted.push_str(&message.content.chars().take(100).collect::<String>());
            formatted.push_str(&"...\n".to_string());
        } else {
            let content = message.content.clone();
            let wrapped_content = wrap(&content, MAX_WIDTH);
            for line in wrapped_content {
                formatted.push_str(&line);
                formatted.push('\n');
            }
        }

        if let Some(tool_calls) = &message.tool_calls {
            for tool_call in tool_calls {
                formatted.push_str(&format!("\n $ {}({})", tool_call.function.name, tool_call.function.arguments));
            }
        }
    }

    formatted
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
    tools_subset: Vec<String>,
    tool_choice: Option<String>,
    only_deterministic_messages: bool,
    temperature: Option<f32>,
    max_new_tokens: Option<usize>,
    n: usize,
    usage_collector_mb: Option<&mut ChatUsage>,
    logfn_mb: Option<String>,
    tx_toolid_mb: Option<String>,
    tx_chatid_mb: Option<String>,
) -> Result<Vec<Vec<ChatMessage>>, String> {
    let gcx = ccx.lock().await.global_context.clone();

    // this ignores customized tools
    let tools_turned_on_by_cmdline = at_tools_merged_and_filtered(gcx.clone()).await.keys().cloned().collect::<Vec<_>>();
    let tools_turn_on_set: HashSet<String> = tools_subset.iter().cloned().collect();
    let tools_turned_on_by_cmdline_set: HashSet<String> = tools_turned_on_by_cmdline.into_iter().collect();
    let tools_on_intersection: Vec<String> = tools_turn_on_set.intersection(&tools_turned_on_by_cmdline_set).cloned().collect();
    let tools_compiled_in_only = tools_compiled_in(&tools_on_intersection).unwrap_or_else(|e|{
        error!("Error loading compiled_in_tools: {:?}", e);
        vec![]
    });
    let tools = tools_compiled_in_only.into_iter().map(|x|x.into_openai_style()).collect::<Vec<_>>();
    info!("tools_subset {:?}", tools_subset);
    info!("tools_turned_on_by_cmdline_set {:?}", tools_turned_on_by_cmdline_set);
    info!("tools_on_intersection {:?}", tools_on_intersection);

    let temperature = Some(temperature.unwrap_or(TEMPERATURE));
    let max_new_tokens = max_new_tokens.unwrap_or(MAX_NEW_TOKENS);
    let (mut chat_post, spad) = create_chat_post_and_scratchpad(
        gcx.clone(),
        model_name,
        messages.iter().collect::<Vec<_>>(),
        temperature,
        max_new_tokens,
        n,
        Some(tools),
        tool_choice.clone(),
        only_deterministic_messages,
    ).await?;

    let chat_response_msgs = chat_interaction(ccx.clone(), spad, &mut chat_post).await?;

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

    if let Some(logfn) = logfn_mb {
        if results.len() > 1 {
            for (i, choice) in results.iter().enumerate() {
                let choice_logfn_json = format!("{}_choice{}.json", logfn, i);
                write_dumps(gcx.clone(), choice_logfn_json, serde_json::to_string_pretty(&choice).unwrap().as_str()).await;
                let choice_logfn_md = format!("{}_choice{}.log", logfn, i);
                let formatted_md = format_messages_as_markdown(&choice);
                write_dumps(gcx.clone(), choice_logfn_md, &formatted_md).await;
            }
        } else if results.len() == 1 {
            let choice0_logfn_json = format!("{}.json", logfn);
            write_dumps(gcx.clone(), choice0_logfn_json, serde_json::to_string_pretty(&results[0]).unwrap().as_str()).await;
            let choice0_logfn_md = format!("{}.log", logfn);
            let formatted_md = format_messages_as_markdown(&results[0]);
            write_dumps(gcx.clone(), choice0_logfn_md, &formatted_md).await;
        }
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
    temperature: Option<f32>,
    logfn_mb: Option<String>,
    tx_toolid_mb: Option<String>,
    tx_chatid_mb: Option<String>,
) -> Result<Vec<ChatMessage>, String> {
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
                tools_subset.clone(),
                Some("auto".to_string()),
                false,
                temperature,
                None,
                1,
                Some(&mut usage_collector),
                logfn_mb.clone(),
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
                vec![],
                Some("none".to_string()),
                true,   // <-- only runs tool calls
                temperature,
                None,
                1,
                Some(&mut usage_collector),
                logfn_mb.clone(),
                tx_toolid_mb.clone(),
                tx_chatid_mb.clone(),
            ).await?[0].clone();
        }
    }
    messages.push(ChatMessage::new("user".to_string(), wrap_up_prompt.to_string()));
    messages = subchat_single(
        ccx.clone(),
        model_name,
        messages,
        vec![],
        Some("none".to_string()),
        false,
        temperature,
        None,
        1,
        Some(&mut usage_collector),
        logfn_mb.clone(),
        tx_toolid_mb.clone(),
        tx_chatid_mb.clone(),
    ).await?[0].clone();
    
    if let Some(last_message) = messages.last_mut() {
        last_message.usage = Some(usage_collector);
    }
    Ok(messages)
}
