use std::sync::Arc;
use crate::at_commands::at_commands::AtCommandsContext;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use crate::call_validation::{ChatMessage, ReasoningEffort};
use crate::cloud::{threads_req, messages_req};
use crate::global_context::GlobalContext;
use std::time::Duration;
use tokio::time::timeout;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::cloud::threads_sub::{initialize_connection, events_loop, get_basic_info, ThreadPayload, BasicStuff};
use tokio::sync::Mutex;

fn build_preferences(
    model: &str,
    temperature: Option<f32>,
    max_new_tokens: Option<usize>,
    n: usize,
    reasoning_effort: Option<ReasoningEffort>,
) -> serde_json::Value {
    let mut preferences = serde_json::json!({
        "model": model,
        "n": n,
    });
    if let Some(temp) = temperature {
        preferences["temperature"] = serde_json::json!(temp);
    }
    if let Some(max_tokens) = max_new_tokens {
        preferences["max_new_tokens"] = serde_json::json!(max_tokens);
    }
    if let Some(reasoning_effort) = reasoning_effort {
        preferences["reasoning_effort"] = serde_json::json!(reasoning_effort);
    }
    preferences
}

pub async fn subchat_single(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model_id: &str,
    expert_name: &str,
    messages: Vec<ChatMessage>,
    tools_subset: Option<Vec<String>>,
    temperature: Option<f32>,
    max_new_tokens: Option<usize>,
    n: usize,
    reasoning_effort: Option<ReasoningEffort>,
) -> Result<Vec<Vec<ChatMessage>>, String> {
    let gcx = ccx.lock().await.global_context.clone();
    let (api_key, app_searchable_id, located_fgroup_id) = {
        let gcx_read = gcx.read().await;
        let located_fgroup_id = gcx_read.active_group_id.clone()
            .ok_or("No active group ID is set".to_string())?;
        (
            gcx_read.cmdline.api_key.clone(),
            gcx_read.app_searchable_id.clone(),
            located_fgroup_id
        )
    };
    let preferences = build_preferences(model_id, temperature, max_new_tokens, n, reasoning_effort);
    let thread_id = format!("subchat_{}_{}", expert_name, thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect::<String>());
    let toolset = if let Some(tool_names) = &tools_subset {
        let mut tools_json = Vec::new();
        let available_tools = crate::tools::tools_list::get_available_tools(gcx.clone()).await;
        for tool in available_tools {
            let tool_desc = tool.tool_description();
            if tool_names.contains(&tool_desc.name) {
                tools_json.push(tool_desc.into_openai_style());
            }
        }
        Some(tools_json)
    } else {
        None
    };
    
    let thread = threads_req::create_thread(
        api_key.clone(),
        &located_fgroup_id,
        expert_name,
        &thread_id,  // title
        &thread_id,  // app_capture
        &app_searchable_id,
        toolset,
        None,  // TODO: we should find a way to forward the parent thread id
    ).await?;
    let thread_messages = messages_req::convert_messages_to_thread_messages(
        messages,
        100,
        100,
        0,
        &thread.ft_id,
        Some(preferences),
    )?;
    messages_req::create_thread_messages(
        api_key.clone(),
        &thread.ft_id,
        thread_messages,
    ).await?;
    
    let max_wait_time = Duration::from_secs(1200);
    let thread_complete = Arc::new(AtomicBool::new(false));
    let error_msg = Arc::new(Mutex::new(None));
    let api_key_for_messages = api_key.clone();
    let thread_id_for_messages = thread_id.clone();
    
    let result = tokio::task::spawn(async move {
        let connection_result = initialize_connection(api_key.clone(), &located_fgroup_id).await;
        let mut connection = match connection_result {
            Ok(conn) => conn,
            Err(err) => return Err(format!("Failed to initialize WebSocket connection: {}", err)),
        };
        let _basic_info = get_basic_info(api_key.clone()).await?;
        let thread_complete_clone = thread_complete.clone();
        let error_msg_clone = error_msg.clone();
        let thread_id_clone = thread_id.clone();
        let processor = move |
            _gcx: Arc<ARwLock<GlobalContext>>, 
            payload: &ThreadPayload, 
            _basic_info: &BasicStuff, 
            processor_api_key: String, 
            _app_searchable_id: String
        | -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>> {
            let thread_id = thread_id_clone.clone();
            let thread_complete = thread_complete_clone.clone();
            let error_msg = error_msg_clone.clone();
            let payload_ft_id = payload.ft_id.clone();
            let payload_error = payload.ft_error.clone();
            Box::pin(async move {
                if payload_ft_id != thread_id {
                    return Ok(());
                }
                if let Some(err) = payload_error {
                    let mut error_lock = error_msg.lock().await;
                    *error_lock = Some(format!("Thread processing error: {}", err.to_string()));
                    thread_complete.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                let thread = threads_req::get_thread(processor_api_key, &thread_id).await?;
                if thread.ft_need_assistant == 0 {
                    thread_complete.store(true, Ordering::SeqCst);
                }
                Ok(())
            })
        };
        match timeout(max_wait_time, async {
            let events_task = tokio::spawn(async move {
                let _ = events_loop(
                    gcx.clone(), 
                    &mut connection, 
                    api_key,
                    processor
                ).await;
            });
            while !thread_complete.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            events_task.abort();
            
            let error_lock = error_msg.lock().await;
            if let Some(err) = &*error_lock {
                return Err(err.clone());
            }
            
            Ok::<(), String>(())
        }).await {
            Ok(result) => result?,
            Err(_) => return Err("Timeout waiting for thread processing".to_string()),
        }
        
        let all_thread_messages = messages_req::get_thread_messages(
            api_key_for_messages,
            &thread_id_for_messages,
            0,
        ).await?;
        
        let chat_messages = messages_req::convert_thread_messages_to_messages(&all_thread_messages);
        
        Ok(chat_messages)
    }).await.map_err(|e| format!("Task execution error: {}", e))??;
    
    let mut final_result = Vec::new();
    final_result.push(result);
    
    Ok(final_result)
}
