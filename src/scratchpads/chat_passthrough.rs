use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use async_trait::async_trait;
use serde_json::Value;
use tokenizers::Tokenizer;
use tokio::sync::RwLock as ARwLock;
use tracing::{error, info, warn};

use crate::at_commands::execute_at::run_at_commands;
use crate::at_tools::execute_att::run_tools;
use crate::call_validation::{ChatMessage, ChatPost, ContextFile, ContextMemory, SamplingParameters};
use crate::global_context::GlobalContext;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::scratchpads::chat_generic::default_system_message_from_patch;
use crate::scratchpads::chat_utils_limit_history::limit_messages_history;
use crate::scratchpads::chat_utils_rag::HasRagResults;

const DEBUG: bool = true;


pub struct DeltaSender {
    pub role_sent: String,
}

impl DeltaSender {
    pub fn new() -> Self {
        DeltaSender {
            role_sent: "".to_string(),
        }
    }

    pub fn feed_delta(&mut self, role: &str, delta: &str, finish_reason: &str, tool_calls: Option<Value>) -> Value {
        let x = serde_json::json!([{
            "index": 0,
            "delta": {
                "role": if role != self.role_sent.as_str() { serde_json::Value::String(role.to_string()) } else { serde_json::Value::Null },
                "content": delta,
                "tool_calls": tool_calls.unwrap_or(serde_json::Value::Null),
            },
            "finish_reason": if finish_reason == "" { serde_json::Value::Null } else { serde_json::Value::String(finish_reason.to_string()) }
        }]);
        self.role_sent = role.to_string();
        x
    }
}


// #[derive(Debug)]
pub struct ChatPassthrough {
    pub t: HasTokenizerAndEot,
    pub post: ChatPost,
    pub default_system_message: String,
    pub has_rag_results: HasRagResults,
    pub delta_sender: DeltaSender,
    pub global_context: Arc<ARwLock<GlobalContext>>,
    pub allow_at: bool,
}

impl ChatPassthrough {
    pub fn new(
        tokenizer: Arc<StdRwLock<Tokenizer>>,
        post: ChatPost,
        global_context: Arc<ARwLock<GlobalContext>>,
        allow_at: bool,
    ) -> Self {
        ChatPassthrough {
            t: HasTokenizerAndEot::new(tokenizer),
            post,
            default_system_message: "".to_string(),
            has_rag_results: HasRagResults::new(),
            delta_sender: DeltaSender::new(),
            global_context,
            allow_at,
        }
    }
}

#[async_trait]
impl ScratchpadAbstract for ChatPassthrough {
    async fn apply_model_adaptation_patch(
        &mut self,
        patch: &Value,
    ) -> Result<(), String> {
        self.default_system_message = default_system_message_from_patch(&patch, self.global_context.clone()).await;
        Ok(())
    }

    async fn prompt(
        &mut self,
        context_size: usize,
        sampling_parameters_to_patch: &mut SamplingParameters,
        tools_mb: Option<Vec<serde_json::Value>>,
    ) -> Result<String, String> {
        info!("chat passthrough {} messages at start", &self.post.messages.len());
        let top_n: usize = 7;
        let (messages, undroppable_msg_n) = if self.allow_at {
            run_at_commands(self.global_context.clone(), self.t.tokenizer.clone(), sampling_parameters_to_patch.max_new_tokens, context_size, &self.post.messages, top_n, &mut self.has_rag_results).await
        } else {
            (self.post.messages.clone(), self.post.messages.len())
        };
        let (messages, _tools_success) = run_tools(self.global_context.clone(), self.t.tokenizer.clone(), sampling_parameters_to_patch.max_new_tokens, context_size, &messages, top_n, &mut self.has_rag_results).await;
        let limited_msgs: Vec<ChatMessage> = limit_messages_history(&self.t, &messages, undroppable_msg_n, sampling_parameters_to_patch.max_new_tokens, context_size, &self.default_system_message).unwrap_or_else(|e| {
            error!("error limiting messages: {}", e);
            vec![]
        });
        info!("chat passthrough {} messages -> {} messages after applying at-commands and limits, possibly adding the default system message", messages.len(), limited_msgs.len());
        let mut filtered_msgs: Vec<ChatMessage> = Vec::<ChatMessage>::new();
        for msg in &limited_msgs {
            if msg.role == "assistant" || msg.role == "system" || msg.role == "user" || msg.role == "tool" {
                filtered_msgs.push(msg.clone());
            } else if msg.role == "context_file" {
                match serde_json::from_str(&msg.content) {
                    Ok(res) => {
                        let vector_of_context_files: Vec<ContextFile> = res;
                        for context_file in &vector_of_context_files {
                            filtered_msgs.push(ChatMessage::new(
                                "user".to_string(),
                                format!("{}:{}-{}\n```\n{}```",
                                    context_file.file_name,
                                    context_file.line1,
                                    context_file.line2,
                                    context_file.file_content),
                            ));
                        }
                    },
                    Err(e) => { error!("error parsing context file: {}", e); }
                }
            } else if msg.role == "context_memory" {
                match serde_json::from_str(&msg.content) {
                    Ok(res) => {
                        let mems: Vec<ContextMemory> = res;
                        for mem in mems.iter() {
                            filtered_msgs.push(ChatMessage::new(
                                "assistant".to_string(),
                                format!("Note to self: {}", mem.memo_text.clone())
                            ));
                        }
                    }
                    Err(e) => { error!("error parsing context memory: {}", e); }
                }
            } else {
                warn!("unknown role: {}", msg.role);
            }
        }
        let big_json = serde_json::json!({
            "messages": filtered_msgs,
            "tools": if let Some(tools) = tools_mb {
                if tools.is_empty() {
                    None
                } else {
                    Some(tools)
                }
            } else {
                None
            },
        });
        let prompt = "PASSTHROUGH ".to_string() + &serde_json::to_string(&big_json).unwrap();
        if DEBUG {
            for msg in &filtered_msgs {
                info!("keep role={} {:?}", msg.role, crate::nicer_logs::first_n_chars(&msg.content, 30));
            }
        }
        Ok(prompt.to_string())
    }

    fn response_n_choices(  // result of old-school OpenAI with text (not messages) which is not possible when using passthrough (means messages)
        &mut self,
        _choices: Vec<String>,
        _stopped: Vec<bool>,
    ) -> Result<serde_json::Value, String> {
        todo!();
    }

    fn response_streaming(
        &mut self,
        delta: String,
        stop_toks: bool,
        stop_length: bool,
    ) -> Result<(serde_json::Value, bool), String> {
        let finished = stop_toks || stop_length;
        let finish_reason = if finished {
            if stop_toks { "stop".to_string() } else { "length".to_string() }
        } else {
            "".to_string()
        };
        let json_choices = self.delta_sender.feed_delta("assistant", &delta, &finish_reason, None);
        let ans = serde_json::json!({
            "choices": json_choices,
            "object": "chat.completion.chunk",
        });
        Ok((ans, finished))
    }

    fn response_spontaneous(&mut self) -> Result<Vec<Value>, String>  {
        self.has_rag_results.response_streaming()
    }
}
