use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use indexmap::IndexMap;
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;
use tracing::info;

use crate::at_commands::execute_at::{run_at_commands_locally, run_at_commands_remotely};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ChatPost, ReasoningEffort, SamplingParameters};
use crate::caps::ModelRecord;
use crate::http::http_get_json;
use crate::integrations::docker::docker_container_manager::docker_container_get_host_lsp_port_to_connect;
use crate::scratchpad_abstract::{FinishReason, HasTokenizerAndEot, ScratchpadAbstract};
use crate::scratchpads::chat_utils_limit_history::fix_and_limit_messages_history;
use crate::scratchpads::scratchpad_utils::HasRagResults;
use crate::scratchpads::chat_utils_prompts::prepend_the_right_system_prompt_and_maybe_more_initial_messages;
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
    pub prepend_system_prompt: bool,
    pub has_rag_results: HasRagResults,
    pub delta_sender: DeltaSender,
    pub allow_at: bool,
    pub supports_tools: bool,
    pub supports_clicks: bool,
}

impl ChatPassthrough {
    pub fn new(
        tokenizer: Arc<StdRwLock<Tokenizer>>,
        post: &ChatPost,
        messages: &Vec<ChatMessage>,
        prepend_system_prompt: bool,
        allow_at: bool,
        supports_tools: bool,
        supports_clicks: bool,
    ) -> Self {
        ChatPassthrough {
            t: HasTokenizerAndEot::new(tokenizer),
            post: post.clone(),
            messages: messages.clone(),
            prepend_system_prompt,
            has_rag_results: HasRagResults::new(),
            delta_sender: DeltaSender::new(),
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
        _exploration_tools: bool,
        _agentic_tools: bool,
    ) -> Result<(), String> {
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
        let mut at_tools = if !should_execute_remotely {
            tools_merged_and_filtered(gcx.clone(), self.supports_clicks).await?
        } else {
            IndexMap::new()
        };

        let messages = if self.prepend_system_prompt && self.allow_at {
            prepend_the_right_system_prompt_and_maybe_more_initial_messages(gcx.clone(), self.messages.clone(), &self.post.meta, &mut self.has_rag_results).await
        } else {
            self.messages.clone()
        };
        let (mut messages, _any_context_produced) = if self.allow_at && !should_execute_remotely {
            run_at_commands_locally(ccx.clone(), self.t.tokenizer.clone(), sampling_parameters_to_patch.max_new_tokens, &messages, &mut self.has_rag_results).await
        } else if self.allow_at {
            run_at_commands_remotely(ccx.clone(), &self.post.model, sampling_parameters_to_patch.max_new_tokens, &messages, &mut self.has_rag_results).await?
        } else {
            (messages, false)
        };
        if self.supports_tools {
            (messages, _) = if should_execute_remotely {
                run_tools_remotely(ccx.clone(), &self.post.model, sampling_parameters_to_patch.max_new_tokens, &messages, &mut self.has_rag_results, &style).await?
            } else {
                run_tools_locally(ccx.clone(), &mut at_tools, self.t.tokenizer.clone(), sampling_parameters_to_patch.max_new_tokens, &messages, &mut self.has_rag_results, &style).await?
            }
        };

        let caps = {
            let gcx_locked = gcx.write().await;
            gcx_locked.caps.clone().unwrap()
        };
        let model_record_mb = {
            let caps_locked = caps.read().unwrap();
            caps_locked.code_chat_models.get(&self.post.model).cloned()
        };

        let supports_reasoning = if let Some(model_record) = model_record_mb.clone() {
            !model_record.supports_reasoning.is_none()
        } else {
            false
        };

        let messages = if supports_reasoning {
            let model_record = model_record_mb.unwrap();
            _adapt_for_reasoning_models(
                &messages,
                sampling_parameters_to_patch,
                model_record.supports_reasoning.unwrap(),
                model_record.default_temperature.clone(),
                model_record.supports_boost_reasoning.clone(),
            )
        } else {
            messages
        };

        let limited_msgs = limit_messages_history(&self.t, &messages, undroppable_msg_n, sampling_parameters_to_patch.max_new_tokens, n_ctx).unwrap_or_else(|e| {
            tracing::error!("error limiting messages: {}", e);
            vec![]
        });

        if self.prepend_system_prompt && !supports_reasoning {
            assert_eq!(limited_msgs.first().unwrap().role, "system");
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
                // and take descriptions of tools from the official source
                if should_execute_remotely {
                    let port = docker_container_get_host_lsp_port_to_connect(gcx.clone(), &self.post.meta.chat_id).await?;
                    tracing::info!("Calling tools on port: {}", port);
                    let tool_desclist: Vec<Value> = http_get_json(&format!("http://localhost:{port}/v1/tools")).await?;
                    Some(tool_desclist.into_iter().filter(|tool_desc| {
                        tool_desc.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()).map_or(false, |n| turned_on.contains(&n.to_string()))
                    }).collect::<Vec<_>>())
                } else {
                    let allow_experimental = gcx.read().await.cmdline.experimental;
                    let tool_descriptions = tool_description_list_from_yaml(at_tools, Some(&turned_on), allow_experimental).await?;
                    Some(tool_descriptions.into_iter().filter(|x| x.is_supported_by(&self.post.model)).map(|x| x.into_openai_style()).collect::<Vec<_>>())
                }
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
        } else if DEBUG {
            info!("PASSTHROUGH TOOLS NOT SUPPORTED");
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

fn _adapt_for_reasoning_models(
    messages: &Vec<ChatMessage>,
    sampling_parameters: &mut SamplingParameters,
    supports_reasoning: String,
    default_temperature: Option<f32>,
    supports_boost_reasoning: bool,
) -> Vec<ChatMessage> {
    match supports_reasoning.as_ref() {
        "openai" => {
            if supports_boost_reasoning && sampling_parameters.boost_reasoning {
                sampling_parameters.reasoning_effort = Some(ReasoningEffort::High);
            }
            // NOTE: OpenAI prefer user message over system
            messages.iter().map(|msg| {
                let mut msg = msg.clone();
                if msg.role == "system" {
                    msg.role = "user".to_string();
                }
                msg
            }).collect()
        },
        "anthropic" => {
            // TODO: anthropic models require thinking to be passed in case of tool message in the end
            // also we should compute budget_tokens
            if supports_boost_reasoning && sampling_parameters.boost_reasoning {
                sampling_parameters.thinking = Some(json!({
                    "type": "enabled",
                    "budget_tokens": 1024,  // in range [1024, max max_completion_tokens]
                }));
            }
            messages.clone()
        },
        _ => {
            sampling_parameters.temperature = default_temperature.clone();
            messages.clone()
        }
    }
}
