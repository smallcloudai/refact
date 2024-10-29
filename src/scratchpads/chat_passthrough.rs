use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use async_trait::async_trait;
use serde_json::Value;
use tokenizers::Tokenizer;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use tracing::{error, info, warn};

use crate::at_commands::execute_at::run_at_commands;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_execute::run_tools;
use crate::call_validation::{ChatContent, ChatMessage, ChatPost, ContextFile, SamplingParameters};
use crate::global_context::GlobalContext;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::scratchpads::chat_utils_limit_history::limit_messages_history;
use crate::scratchpads::scratchpad_utils::HasRagResults;
use crate::scratchpads::chat_utils_prompts::{get_default_system_prompt, system_prompt_add_workspace_info};


const DEBUG: bool = false;


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
    pub messages: Vec<ChatMessage>,
    pub default_system_message: String,
    pub has_rag_results: HasRagResults,
    pub delta_sender: DeltaSender,
    pub global_context: Arc<ARwLock<GlobalContext>>,
    pub allow_at: bool,
    pub supports_tools: bool,
}

impl ChatPassthrough {
    pub fn new(
        tokenizer: Arc<StdRwLock<Tokenizer>>,
        post: &ChatPost,
        messages: &Vec<ChatMessage>,
        global_context: Arc<ARwLock<GlobalContext>>,
        allow_at: bool,
        supports_tools: bool,
    ) -> Self {
        ChatPassthrough {
            t: HasTokenizerAndEot::new(tokenizer),
            post: post.clone(),
            messages: messages.clone(),
            default_system_message: "".to_string(),
            has_rag_results: HasRagResults::new(),
            delta_sender: DeltaSender::new(),
            global_context,
            allow_at,
            supports_tools,
        }
    }
}

#[async_trait]
impl ScratchpadAbstract for ChatPassthrough {
    async fn apply_model_adaptation_patch(
        &mut self,
        _patch: &Value,
        exploration_tools: bool,
        agentic_tools: bool,
    ) -> Result<(), String> {
        self.default_system_message = get_default_system_prompt(self.global_context.clone(), exploration_tools, agentic_tools).await;
        Ok(())
    }

    async fn prompt(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        sampling_parameters_to_patch: &mut SamplingParameters,
    ) -> Result<String, String> {
        let (n_ctx, gcx) = {
            let ccx_locked = ccx.lock().await;
            (ccx_locked.n_ctx, ccx_locked.global_context.clone())
        };
        let style = self.post.style.clone();
        let (mut messages, undroppable_msg_n, _any_context_produced) = if self.allow_at {
            run_at_commands(ccx.clone(), self.t.tokenizer.clone(), sampling_parameters_to_patch.max_new_tokens, &self.messages, &mut self.has_rag_results).await
        } else {
            (self.messages.clone(), self.messages.len(), false)
        };
        if self.supports_tools {
            (messages, _) = run_tools(ccx.clone(), self.t.tokenizer.clone(), sampling_parameters_to_patch.max_new_tokens, &messages, &mut self.has_rag_results, &style).await?;
        };
        let mut limited_msgs = limit_messages_history(&self.t, &messages, undroppable_msg_n, sampling_parameters_to_patch.max_new_tokens, n_ctx, &self.default_system_message).unwrap_or_else(|e| {
            error!("error limiting messages: {}", e);
            vec![]
        });
        if let Some(first_msg) = limited_msgs.first_mut() {
            if first_msg.role == "system" {
                first_msg.content = ChatContent::SimpleText(system_prompt_add_workspace_info(gcx.clone(), &first_msg.content.content_text_only()).await);
            }
        }
        if DEBUG {
            info!("chat passthrough {} messages -> {} messages after applying at-commands and limits, possibly adding the default system message", messages.len(), limited_msgs.len());
        }
        let mut filtered_msgs = vec![];
        for msg in &limited_msgs {
            if msg.role == "tool" {
                match &msg.content {
                    ChatContent::Multimodal(multimodal_content) => {
                        let texts = multimodal_content.iter().filter(|x|x.is_text()).collect::<Vec<_>>();
                        let images = multimodal_content.iter().filter(|x|x.is_image()).collect::<Vec<_>>();
                        let text = if texts.is_empty() {
                            "attached images below".to_string()
                        } else {
                            texts.iter().map(|x|x.m_content.clone()).collect::<Vec<_>>().join("\n")
                        };
                        let mut msg_cloned = msg.clone();
                        msg_cloned.content = ChatContent::SimpleText(text);
                        filtered_msgs.push(msg_cloned.into_value(&style));
                        if !images.is_empty() {
                            let msg_img = ChatMessage {
                                role: "user".to_string(),
                                content: ChatContent::Multimodal(images.into_iter().cloned().collect()),
                               ..Default::default()
                            };
                            filtered_msgs.push(msg_img.into_value(&style));
                        }
                    },
                    ChatContent::SimpleText(_) => {
                        filtered_msgs.push(msg.into_value(&style));
                    }
                }
            }
            if msg.role == "assistant" || msg.role == "system" || msg.role == "user" {
                filtered_msgs.push(msg.into_value(&style));

            } else if msg.role == "diff" {
                let tool_msg = ChatMessage {
                    role: "tool".to_string(),
                    content: msg.content.clone(),
                    tool_calls: None,
                    tool_call_id: msg.tool_call_id.clone(),
                    ..Default::default()
                };
                filtered_msgs.push(tool_msg.into_value(&style));

            } else if msg.role == "plain_text" || msg.role == "cd_instruction" {
                filtered_msgs.push(ChatMessage::new(
                    "user".to_string(),
                    msg.content.content_text_only(),
                ).into_value(&style));

            } else if msg.role == "context_file" {
                match serde_json::from_str::<Vec<ContextFile>>(&msg.content.content_text_only()) {
                    Ok(vector_of_context_files) => {
                        for context_file in vector_of_context_files {
                            filtered_msgs.push(ChatMessage::new(
                                "user".to_string(),
                                format!("{}:{}-{}\n```\n{}```",
                                        context_file.file_name,
                                        context_file.line1,
                                        context_file.line2,
                                        context_file.file_content),
                            ).into_value(&style));
                        }
                    },
                    Err(e) => { error!("error parsing context file: {}", e); }
                }
            } else {
                warn!("unknown role: {}", msg.role);
            }
        }
        let mut big_json = serde_json::json!({
            "messages": filtered_msgs,
        });
        if self.supports_tools {
            let tools = if let Some(tools) = &self.post.tools {
                // if tools.is_empty() || any_context_produced {
                if tools.is_empty() {
                        None
                } else {
                    Some(tools)
                }
            } else {
                None
            };
            big_json["tools"] = serde_json::json!(tools);
            big_json["tool_choice"] = serde_json::json!(self.post.tool_choice);
            if DEBUG {
                info!("PASSTHROUGH TOOLS ENABLED CNT: {:?}", tools.unwrap_or(&vec![]).len());
            }
        } else {
            if DEBUG {
                info!("PASSTHROUGH TOOLS NOT SUPPORTED");
            }
        }
        let prompt = "PASSTHROUGH ".to_string() + &serde_json::to_string(&big_json).unwrap();
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
