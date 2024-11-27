use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use tracing::{error, info};
use crate::at_commands::execute_at::run_at_commands;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ChatPost, SamplingParameters};
use crate::global_context::GlobalContext;
use crate::scratchpad_abstract::{FinishReason, HasTokenizerAndEot, ScratchpadAbstract};
use crate::scratchpads::chat_utils_limit_history::limit_messages_history;
use crate::scratchpads::scratchpad_utils::HasRagResults;
use crate::scratchpads::chat_utils_prompts::{get_default_system_prompt, get_default_system_prompt_from_remote, system_prompt_add_workspace_info};
use crate::scratchpads::passthrough_convert_messages::convert_messages_to_openai_format;
use crate::tools::tools_description::{tool_description_list_from_yaml, tools_merged_and_filtered};
use crate::tools::tools_execute::{run_tools_locally, run_tools_remotely};


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

    pub fn feed_delta(&mut self, role: &str, _json: &Value, finish_reason: &FinishReason, tool_calls: Option<Value>) -> Value {
        // TODO: correctly implement it
        let x = json!([{
            "index": 0,
            "delta": {
                "role": if role != self.role_sent.as_str() { Value::String(role.to_string()) } else { Value::Null },
                "content": "",
                "tool_calls": tool_calls.unwrap_or(Value::Null),
            },
            "finish_reason": finish_reason.to_json_val()
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
    pub supports_clicks: bool,
}

impl ChatPassthrough {
    pub fn new(
        tokenizer: Arc<StdRwLock<Tokenizer>>,
        post: &ChatPost,
        messages: &Vec<ChatMessage>,
        global_context: Arc<ARwLock<GlobalContext>>,
        allow_at: bool,
        supports_tools: bool,
        supports_clicks: bool,
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
            supports_clicks,
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
        should_execute_remotely: bool,
    ) -> Result<(), String> {
        self.default_system_message = if should_execute_remotely {
            get_default_system_prompt_from_remote(self.global_context.clone(), exploration_tools, agentic_tools, &self.post.chat_id).await?
        } else {
            get_default_system_prompt(self.global_context.clone(), exploration_tools, agentic_tools).await
        };
        Ok(())
    }

    async fn prompt(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        sampling_parameters_to_patch: &mut SamplingParameters,
    ) -> Result<String, String> {
        let (gcx, n_ctx, should_execute_remotely) = {
            let ccx_locked = ccx.lock().await;
            (ccx_locked.global_context.clone(), ccx_locked.n_ctx, ccx_locked.should_execute_remotely)
        };
        let style = self.post.style.clone();
        let at_tools = tools_merged_and_filtered(gcx.clone(), self.supports_clicks).await?;

        // TODO? Maybe we should execute at commands remotely.
        let (mut messages, undroppable_msg_n, _any_context_produced) = if self.allow_at && !should_execute_remotely {
            run_at_commands(ccx.clone(), self.t.tokenizer.clone(), sampling_parameters_to_patch.max_new_tokens, &self.messages, &mut self.has_rag_results).await
        } else {
            (self.messages.clone(), self.messages.len(), false)
        };
        if self.supports_tools {
            (messages, _) = if should_execute_remotely {
                run_tools_remotely(ccx.clone(), &self.post.model, sampling_parameters_to_patch.max_new_tokens, &messages, &mut self.has_rag_results, &style).await?
            } else {
                run_tools_locally(ccx.clone(), at_tools.clone(), self.t.tokenizer.clone(), sampling_parameters_to_patch.max_new_tokens, &messages, &mut self.has_rag_results, &style).await?
            }
        };
        let mut limited_msgs = limit_messages_history(&self.t, &messages, undroppable_msg_n, sampling_parameters_to_patch.max_new_tokens, n_ctx, &self.default_system_message).unwrap_or_else(|e| {
            error!("error limiting messages: {}", e);
            vec![]
        });
        if let Some(first_msg) = limited_msgs.first_mut() {
            if first_msg.role == "system" {
                first_msg.content = ChatContent::SimpleText(system_prompt_add_workspace_info(gcx.clone(), &first_msg.content.content_text_only()).await);
            }
            if self.post.model == "o1-mini" && first_msg.role == "system" {
                limited_msgs.remove(0);
            }
        }
        if DEBUG {
            info!("chat passthrough {} messages -> {} messages after applying at-commands and limits, possibly adding the default system message", messages.len(), limited_msgs.len());
        }

        let converted_messages = convert_messages_to_openai_format(limited_msgs, &style);

        let mut big_json = serde_json::json!({
            "messages": converted_messages,
        });

        if self.supports_tools {
            let post_tools = self.post.tools.as_ref().and_then(|tools| {
                if tools.is_empty() {
                    None
                } else {
                    Some(tools.clone())
                }
            });

            let mut tools = if let Some(t) = post_tools {
                // here we only use names from the tools in `post`
                let turned_on = t.iter().filter_map(|x| {
                    if let Value::Object(map) = x {
                        map.get("function").and_then(|f| f.get("name")).and_then(|name| name.as_str().map(|s| s.to_string()))
                    } else {
                        None
                    }
                }).collect::<Vec<String>>();
                let allow_experimental = gcx.read().await.cmdline.experimental;
                // and take descriptions of tools from the official source
                let tool_descriptions = tool_description_list_from_yaml(at_tools, &turned_on, allow_experimental).await?;
                Some(tool_descriptions.into_iter().map(|x|x.into_openai_style()).collect::<Vec<_>>())
            } else {
                None
            };

            // remove "agentic"
            if let Some(tools) = &mut tools {
                for tool in tools {
                    if let Some(function) = tool.get_mut("function") {
                        function.as_object_mut().unwrap().remove("agentic");
                    }
                }
            }

            big_json["tools"] = json!(tools);
            big_json["tool_choice"] = json!(self.post.tool_choice);
            if DEBUG {
                info!("PASSTHROUGH TOOLS ENABLED CNT: {:?}", tools.unwrap_or(vec![]).len());
            }
        } else {
            if DEBUG {
                info!("PASSTHROUGH TOOLS NOT SUPPORTED");
            }
        }
        let prompt = "PASSTHROUGH ".to_string() + &serde_json::to_string(&big_json).unwrap();
        Ok(prompt.to_string())
    }

    fn response_n_choices(
        &mut self,
        _choices: Vec<String>,
        _finish_reasons: Vec<FinishReason>,
    ) -> Result<Value, String> {
        Err("not implemented".to_string())
    }

    fn response_streaming(
        &mut self,
        _delta: String,
        _finish_reason: FinishReason
    ) -> Result<(Value, FinishReason), String> {
        Err("not implemented".to_string())
    }

    fn response_message_n_choices(
        &mut self,
        _choices: Vec<String>,
        _finish_reasons: Vec<FinishReason>,
    ) -> Result<Value, String> {
        Err("not implemented".to_string())
    }

    fn response_message_streaming(
        &mut self,
        json: &Value,
        finish_reason: FinishReason,
    ) -> Result<(Value, FinishReason), String> {
        Ok((json.clone(), finish_reason))
    }

    fn response_spontaneous(&mut self) -> Result<Vec<Value>, String>  {
        self.has_rag_results.response_streaming()
    }

    fn streaming_finished(&mut self, finish_reason: FinishReason) -> Result<Value, String> {
        let json_choices = self.delta_sender.feed_delta("assistant", &json!({}), &finish_reason, None);
        Ok(json!({
            "choices": json_choices,
            "object": "chat.completion.chunk",
        }))
    }
}
