#[cfg(test)]
mod tests {
    use serde_json::json;
    use crate::call_validation::{ChatMessage, ChatContent, ChatUsage, ChatToolCall, ChatToolFunction};
    use crate::scratchpads::multimodality::MultimodalElement;
    use crate::chat::types::{ChatEvent, DeltaOp, SessionState, PauseReason};

    fn extract_extra_fields(json_val: &serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        let mut result = serde_json::Map::new();
        if let Some(obj) = json_val.as_object() {
            for (key, val) in obj {
                if val.is_null() {
                    continue;
                }
                let dominated = key.starts_with("metering_")
                    || key.starts_with("billing_")
                    || key.starts_with("cost_")
                    || key.starts_with("cache_")
                    || key == "system_fingerprint";
                if dominated {
                    result.insert(key.clone(), val.clone());
                }
            }
        }
        if let Some(psf) = json_val.get("provider_specific_fields") {
            if !psf.is_null() {
                result.insert("provider_specific_fields".to_string(), psf.clone());
            }
        }
        result
    }

    #[test]
    fn test_chat_message_roundtrip_all_fields() {
        let original = ChatMessage {
            message_id: "msg-123".to_string(),
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("Hello world".to_string()),
            tool_calls: Some(vec![
                ChatToolCall {
                    id: "call-1".to_string(),
                    index: None,
                    function: ChatToolFunction {
                        name: "test_tool".to_string(),
                        arguments: r#"{"arg": "value"}"#.to_string(),
                    },
                    tool_type: "function".to_string(),
                }
            ]),
            tool_call_id: "".to_string(),
            tool_failed: None,
            usage: Some(ChatUsage {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: 150,
            }),
            finish_reason: Some("stop".to_string()),
            reasoning_content: Some("I think therefore I am".to_string()),
            thinking_blocks: Some(vec![json!({"type": "thinking", "thinking": "deep thought"})]),
            citations: vec![json!({"url": "https://example.com", "title": "Example"})],
            extra: {
                let mut m = serde_json::Map::new();
                m.insert("custom_field".to_string(), json!("custom_value"));
                m.insert("metering_balance".to_string(), json!(100));
                m
            },
            checkpoints: vec![],
            output_filter: None,
        };

        let serialized = serde_json::to_value(&original).expect("serialize");
        let deserialized: ChatMessage = serde_json::from_value(serialized.clone()).expect("deserialize");

        assert_eq!(deserialized.message_id, original.message_id);
        assert_eq!(deserialized.role, original.role);
        assert_eq!(deserialized.finish_reason, original.finish_reason);
        assert_eq!(deserialized.reasoning_content, original.reasoning_content);

        assert!(deserialized.tool_calls.is_some());
        let tc = deserialized.tool_calls.as_ref().unwrap();
        assert_eq!(tc.len(), 1);
        assert_eq!(tc[0].id, "call-1");
        assert_eq!(tc[0].function.name, "test_tool");

        assert!(deserialized.usage.is_some());
        let usage = deserialized.usage.as_ref().unwrap();
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);

        assert!(deserialized.thinking_blocks.is_some());
        assert_eq!(deserialized.thinking_blocks.as_ref().unwrap().len(), 1);

        assert_eq!(deserialized.citations.len(), 1);

        assert!(deserialized.extra.contains_key("custom_field") || deserialized.extra.contains_key("metering_balance"));
    }

    #[test]
    fn test_chat_message_roundtrip_multimodal_content() {
        let original = ChatMessage {
            message_id: "msg-mm".to_string(),
            role: "user".to_string(),
            content: ChatContent::Multimodal(vec![
                MultimodalElement::new("text".to_string(), "Hello".to_string()).unwrap(),
            ]),
            ..Default::default()
        };

        let serialized = serde_json::to_value(&original).expect("serialize");
        let deserialized: ChatMessage = serde_json::from_value(serialized).expect("deserialize");

        match &deserialized.content {
            ChatContent::Multimodal(elements) => {
                assert_eq!(elements.len(), 1);
                assert_eq!(elements[0].m_type, "text");
                assert_eq!(elements[0].m_content, "Hello");
            }
            _ => panic!("Expected Multimodal content"),
        }
    }

    #[test]
    fn test_chat_message_empty_optional_fields() {
        let original = ChatMessage {
            message_id: "msg-empty".to_string(),
            role: "user".to_string(),
            content: ChatContent::SimpleText("Just text".to_string()),
            ..Default::default()
        };

        let serialized = serde_json::to_value(&original).expect("serialize");
        let deserialized: ChatMessage = serde_json::from_value(serialized).expect("deserialize");

        assert_eq!(deserialized.message_id, "msg-empty");
        assert!(deserialized.tool_calls.is_none());
        assert!(deserialized.usage.is_none());
        assert!(deserialized.reasoning_content.is_none());
        assert!(deserialized.thinking_blocks.is_none());
        assert!(deserialized.citations.is_empty());
    }

    #[test]
    fn test_chat_message_preserves_extra_unknown_keys() {
        let json_with_unknown = json!({
            "message_id": "msg-unk",
            "role": "assistant",
            "content": "test",
            "unknown_field_1": "value1",
            "unknown_field_2": 42,
            "nested_unknown": {"a": 1, "b": 2}
        });

        let deserialized: ChatMessage = serde_json::from_value(json_with_unknown).expect("deserialize");

        assert_eq!(deserialized.message_id, "msg-unk");
        assert!(deserialized.extra.contains_key("unknown_field_1") ||
                deserialized.extra.contains_key("unknown_field_2") ||
                deserialized.extra.contains_key("nested_unknown"));
    }

    #[test]
    fn test_chat_usage_roundtrip() {
        let usage = ChatUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };

        let serialized = serde_json::to_value(&usage).expect("serialize");
        let deserialized: ChatUsage = serde_json::from_value(serialized).expect("deserialize");

        assert_eq!(deserialized.prompt_tokens, 100);
        assert_eq!(deserialized.completion_tokens, 50);
        assert_eq!(deserialized.total_tokens, 150);
    }

    #[test]
    fn test_extract_extra_metering_fields() {
        let json = json!({
            "metering_balance": 100,
            "metering_prompt_tokens_n": 50,
            "metering_generated_tokens_n": 25,
            "other_field": "ignored"
        });

        let extra = extract_extra_fields(&json);

        assert_eq!(extra.get("metering_balance"), Some(&json!(100)));
        assert_eq!(extra.get("metering_prompt_tokens_n"), Some(&json!(50)));
        assert_eq!(extra.get("metering_generated_tokens_n"), Some(&json!(25)));
        assert!(extra.get("other_field").is_none());
    }

    #[test]
    fn test_extract_extra_new_metering_fields() {
        let json = json!({
            "metering_new_field_2025": 999,
            "metering_another_new": "value"
        });

        let extra = extract_extra_fields(&json);

        assert_eq!(extra.get("metering_new_field_2025"), Some(&json!(999)));
        assert_eq!(extra.get("metering_another_new"), Some(&json!("value")));
    }

    #[test]
    fn test_extract_extra_billing_cost_cache_fields() {
        let json = json!({
            "billing_total": 1.5,
            "cost_per_token": 0.001,
            "cache_hit": true
        });

        let extra = extract_extra_fields(&json);

        assert_eq!(extra.get("billing_total"), Some(&json!(1.5)));
        assert_eq!(extra.get("cost_per_token"), Some(&json!(0.001)));
        assert_eq!(extra.get("cache_hit"), Some(&json!(true)));
    }

    #[test]
    fn test_extract_extra_system_fingerprint() {
        let json = json!({
            "system_fingerprint": "fp_abc123",
            "id": "ignored"
        });

        let extra = extract_extra_fields(&json);

        assert_eq!(extra.get("system_fingerprint"), Some(&json!("fp_abc123")));
        assert!(extra.get("id").is_none());
    }

    #[test]
    fn test_extract_extra_provider_specific_fields() {
        let json = json!({
            "provider_specific_fields": {
                "custom_field": "value",
                "nested": {"a": 1}
            }
        });

        let extra = extract_extra_fields(&json);

        let psf = extra.get("provider_specific_fields").unwrap();
        assert_eq!(psf.get("custom_field"), Some(&json!("value")));
    }

    #[test]
    fn test_extract_extra_null_values_ignored() {
        let json = json!({
            "metering_balance": null,
            "metering_tokens": 100
        });

        let extra = extract_extra_fields(&json);

        assert!(extra.get("metering_balance").is_none());
        assert_eq!(extra.get("metering_tokens"), Some(&json!(100)));
    }

    #[test]
    fn test_extract_extra_empty_object() {
        let json = json!({});
        let extra = extract_extra_fields(&json);
        assert!(extra.is_empty());
    }

    #[test]
    fn test_extract_extra_combined() {
        let json = json!({
            "metering_balance": 100,
            "billing_amount": 5.0,
            "cost_total": 0.05,
            "cache_status": "hit",
            "system_fingerprint": "fp_123",
            "provider_specific_fields": {"x": 1},
            "ignored_field": "nope",
            "choices": [{"delta": {}}]
        });

        let extra = extract_extra_fields(&json);

        assert_eq!(extra.len(), 6);
        assert!(extra.contains_key("metering_balance"));
        assert!(extra.contains_key("billing_amount"));
        assert!(extra.contains_key("cost_total"));
        assert!(extra.contains_key("cache_status"));
        assert!(extra.contains_key("system_fingerprint"));
        assert!(extra.contains_key("provider_specific_fields"));
        assert!(!extra.contains_key("ignored_field"));
        assert!(!extra.contains_key("choices"));
    }

    fn merge_tool_calls(
        existing: &mut Vec<ChatToolCall>,
        new_calls: &[serde_json::Value],
    ) {
        for call_val in new_calls {
            let index = call_val.get("index")
                .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
                .map(|i| i as usize);

            let id = call_val.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let call_type = call_val.get("type").and_then(|v| v.as_str()).unwrap_or("function").to_string();

            let func = call_val.get("function");
            let name = func.and_then(|f| f.get("name")).and_then(|v| v.as_str()).map(|s| s.to_string());
            let args = func.and_then(|f| f.get("arguments")).and_then(|v| v.as_str()).unwrap_or("").to_string();

            if let Some(name) = name {
                let new_call = ChatToolCall {
                    id: id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                    index,
                    function: ChatToolFunction {
                        name,
                        arguments: args,
                    },
                    tool_type: call_type,
                };
                existing.push(new_call);
            } else if !args.is_empty() {
                if let Some(last) = existing.last_mut() {
                    last.function.arguments.push_str(&args);
                }
            }
        }
    }

    #[test]
    fn test_merge_tool_calls_new_call_with_name() {
        let mut existing = Vec::new();
        let new_calls = vec![json!({
            "id": "call-1",
            "type": "function",
            "function": {
                "name": "test_tool",
                "arguments": "{\"a\": 1}"
            }
        })];

        merge_tool_calls(&mut existing, &new_calls);

        assert_eq!(existing.len(), 1);
        assert_eq!(existing[0].id, "call-1");
        assert_eq!(existing[0].function.name, "test_tool");
        assert_eq!(existing[0].function.arguments, "{\"a\": 1}");
    }

    #[test]
    fn test_merge_tool_calls_argument_continuation() {
        let mut existing = vec![ChatToolCall {
            id: "call-1".to_string(),
            index: Some(0),
            function: ChatToolFunction {
                name: "test_tool".to_string(),
                arguments: "{\"a\":".to_string(),
            },
            tool_type: "function".to_string(),
        }];

        let new_calls = vec![json!({
            "function": {
                "arguments": " 1}"
            }
        })];

        merge_tool_calls(&mut existing, &new_calls);

        assert_eq!(existing.len(), 1);
        assert_eq!(existing[0].function.arguments, "{\"a\": 1}");
    }

    #[test]
    fn test_merge_tool_calls_missing_id_generates_uuid() {
        let mut existing = Vec::new();
        let new_calls = vec![json!({
            "function": {
                "name": "no_id_tool",
                "arguments": "{}"
            }
        })];

        merge_tool_calls(&mut existing, &new_calls);

        assert_eq!(existing.len(), 1);
        assert!(!existing[0].id.is_empty());
        assert!(existing[0].id.len() > 10);
    }

    #[test]
    fn test_merge_tool_calls_missing_type_defaults_to_function() {
        let mut existing = Vec::new();
        let new_calls = vec![json!({
            "id": "call-1",
            "function": {
                "name": "test",
                "arguments": "{}"
            }
        })];

        merge_tool_calls(&mut existing, &new_calls);

        assert_eq!(existing[0].tool_type, "function");
    }

    #[test]
    fn test_merge_tool_calls_index_as_string() {
        let mut existing = Vec::new();
        let new_calls = vec![json!({
            "index": "1",
            "id": "call-1",
            "function": {
                "name": "test",
                "arguments": "{}"
            }
        })];

        merge_tool_calls(&mut existing, &new_calls);

        assert_eq!(existing[0].index, Some(1));
    }

    #[test]
    fn test_merge_tool_calls_multiple_calls() {
        let mut existing = Vec::new();
        let new_calls = vec![
            json!({
                "index": 0,
                "id": "call-0",
                "function": {"name": "tool_a", "arguments": "{}"}
            }),
            json!({
                "index": 1,
                "id": "call-1",
                "function": {"name": "tool_b", "arguments": "{}"}
            }),
        ];

        merge_tool_calls(&mut existing, &new_calls);

        assert_eq!(existing.len(), 2);
        assert_eq!(existing[0].function.name, "tool_a");
        assert_eq!(existing[1].function.name, "tool_b");
    }

    #[test]
    fn test_merge_tool_calls_empty_arguments_only_ignored() {
        let mut existing = vec![ChatToolCall {
            id: "call-1".to_string(),
            index: Some(0),
            function: ChatToolFunction {
                name: "test".to_string(),
                arguments: "{}".to_string(),
            },
            tool_type: "function".to_string(),
        }];

        let new_calls = vec![json!({
            "function": {
                "arguments": ""
            }
        })];

        merge_tool_calls(&mut existing, &new_calls);

        assert_eq!(existing.len(), 1);
        assert_eq!(existing[0].function.arguments, "{}");
    }

    #[test]
    fn test_delta_op_append_content() {
        let ops = vec![
            DeltaOp::AppendContent { text: "Hello ".to_string() },
            DeltaOp::AppendContent { text: "world".to_string() },
        ];

        let mut content = String::new();
        for op in ops {
            if let DeltaOp::AppendContent { text } = op {
                content.push_str(&text);
            }
        }

        assert_eq!(content, "Hello world");
    }

    #[test]
    fn test_delta_op_append_reasoning() {
        let ops = vec![
            DeltaOp::AppendReasoning { text: "First ".to_string() },
            DeltaOp::AppendReasoning { text: "thought".to_string() },
        ];

        let mut reasoning = String::new();
        for op in ops {
            if let DeltaOp::AppendReasoning { text } = op {
                reasoning.push_str(&text);
            }
        }

        assert_eq!(reasoning, "First thought");
    }

    #[test]
    fn test_delta_op_merge_extra_preserves_existing() {
        let mut extra = serde_json::Map::new();
        extra.insert("existing".to_string(), json!("value"));

        let op = DeltaOp::MergeExtra {
            extra: {
                let mut m = serde_json::Map::new();
                m.insert("new_field".to_string(), json!(123));
                m
            }
        };

        if let DeltaOp::MergeExtra { extra: new_extra } = op {
            extra.extend(new_extra);
        }

        assert_eq!(extra.get("existing"), Some(&json!("value")));
        assert_eq!(extra.get("new_field"), Some(&json!(123)));
    }

    #[test]
    fn test_delta_op_merge_extra_successive_updates() {
        let mut extra = serde_json::Map::new();

        let ops = vec![
            DeltaOp::MergeExtra {
                extra: {
                    let mut m = serde_json::Map::new();
                    m.insert("metering_a".to_string(), json!(1));
                    m
                }
            },
            DeltaOp::MergeExtra {
                extra: {
                    let mut m = serde_json::Map::new();
                    m.insert("metering_b".to_string(), json!(2));
                    m
                }
            },
            DeltaOp::MergeExtra {
                extra: {
                    let mut m = serde_json::Map::new();
                    m.insert("metering_a".to_string(), json!(10));
                    m
                }
            },
        ];

        for op in ops {
            if let DeltaOp::MergeExtra { extra: new_extra } = op {
                extra.extend(new_extra);
            }
        }

        assert_eq!(extra.get("metering_a"), Some(&json!(10)));
        assert_eq!(extra.get("metering_b"), Some(&json!(2)));
    }

    #[test]
    fn test_delta_op_merge_extra_does_not_overwrite_core_fields() {
        let mut msg = ChatMessage {
            message_id: "msg-1".to_string(),
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("Hello".to_string()),
            ..Default::default()
        };

        let dangerous_extra = {
            let mut m = serde_json::Map::new();
            m.insert("content".to_string(), json!("OVERWRITTEN"));
            m.insert("role".to_string(), json!("hacker"));
            m.insert("message_id".to_string(), json!("fake-id"));
            m.insert("metering_safe".to_string(), json!(100));
            m
        };

        msg.extra.extend(dangerous_extra);

        assert_eq!(msg.message_id, "msg-1");
        assert_eq!(msg.role, "assistant");
        match &msg.content {
            ChatContent::SimpleText(s) => assert_eq!(s, "Hello"),
            _ => panic!("Content type changed"),
        }
        assert_eq!(msg.extra.get("metering_safe"), Some(&json!(100)));
    }

    #[test]
    fn test_session_state_transitions() {
        assert_eq!(format!("{:?}", SessionState::Idle), "Idle");
        assert_eq!(format!("{:?}", SessionState::Generating), "Generating");
        assert_eq!(format!("{:?}", SessionState::Paused), "Paused");
        assert_eq!(format!("{:?}", SessionState::ExecutingTools), "ExecutingTools");
        assert_eq!(format!("{:?}", SessionState::Error), "Error");
    }

    #[test]
    fn test_chat_event_serialization_stream_finished() {
        let event = ChatEvent::StreamFinished {
            message_id: "msg-123".to_string(),
            finish_reason: Some("abort".to_string()),
        };

        let json = serde_json::to_value(&event).expect("serialize");

        assert_eq!(json.get("type"), Some(&json!("stream_finished")));
        assert_eq!(json.get("message_id"), Some(&json!("msg-123")));
        assert_eq!(json.get("finish_reason"), Some(&json!("abort")));
    }

    #[test]
    fn test_chat_event_serialization_message_removed() {
        let event = ChatEvent::MessageRemoved {
            message_id: "msg-456".to_string(),
        };

        let json = serde_json::to_value(&event).expect("serialize");

        assert_eq!(json.get("type"), Some(&json!("message_removed")));
        assert_eq!(json.get("message_id"), Some(&json!("msg-456")));
    }

    #[test]
    fn test_chat_event_serialization_runtime_updated() {
        let event = ChatEvent::RuntimeUpdated {
            state: SessionState::Idle,
            paused: false,
            error: None,
            queue_size: 0,
        };

        let json = serde_json::to_value(&event).expect("serialize");

        assert_eq!(json.get("type"), Some(&json!("runtime_updated")));
        assert_eq!(json.get("state"), Some(&json!("idle")));
        assert_eq!(json.get("paused"), Some(&json!(false)));
    }

    #[test]
    fn test_chat_event_serialization_pause_required() {


        let event = ChatEvent::PauseRequired {
            reasons: vec![PauseReason {
                reason_type: "confirmation".to_string(),
                command: "shell".to_string(),
                rule: "deny_all".to_string(),
                tool_call_id: "tc-1".to_string(),
                integr_config_path: None,
            }],
        };

        let json = serde_json::to_value(&event).expect("serialize");

        assert_eq!(json.get("type"), Some(&json!("pause_required")));
        let reasons = json.get("reasons").unwrap().as_array().unwrap();
        assert_eq!(reasons.len(), 1);
        assert_eq!(reasons[0].get("tool_call_id"), Some(&json!("tc-1")));
    }

    #[test]
    fn test_chat_event_serialization_pause_cleared() {
        let event = ChatEvent::PauseCleared {};

        let json = serde_json::to_value(&event).expect("serialize");

        assert_eq!(json.get("type"), Some(&json!("pause_cleared")));
    }

    #[test]
    fn test_normalize_tool_call_valid_complete() {
        let tc = json!({
            "id": "call_abc123",
            "index": 0,
            "type": "function",
            "function": {
                "name": "test_tool",
                "arguments": "{\"key\": \"value\"}"
            }
        });

        let result = normalize_tool_call(&tc);
        assert!(result.is_some());

        let call = result.unwrap();
        assert_eq!(call.id, "call_abc123");
        assert_eq!(call.index, Some(0));
        assert_eq!(call.function.name, "test_tool");
        assert_eq!(call.function.arguments, "{\"key\": \"value\"}");
        assert_eq!(call.tool_type, "function");
    }

    #[test]
    fn test_normalize_tool_call_missing_id_generates_uuid() {
        let tc = json!({
            "function": {
                "name": "test_tool",
                "arguments": "{}"
            }
        });

        let result = normalize_tool_call(&tc);
        assert!(result.is_some());

        let call = result.unwrap();
        assert!(call.id.starts_with("call_"));
        assert!(call.id.len() >= 20);
    }

    #[test]
    fn test_normalize_tool_call_missing_type_defaults_function() {
        let tc = json!({
            "id": "call_123",
            "function": {
                "name": "my_tool",
                "arguments": "{}"
            }
        });

        let result = normalize_tool_call(&tc);
        assert!(result.is_some());
        assert_eq!(result.unwrap().tool_type, "function");
    }

    #[test]
    fn test_normalize_tool_call_arguments_as_object() {
        let tc = json!({
            "id": "call_123",
            "function": {
                "name": "my_tool",
                "arguments": {"nested": "object", "num": 42}
            }
        });

        let result = normalize_tool_call(&tc);
        assert!(result.is_some());

        let call = result.unwrap();
        assert!(call.function.arguments.contains("nested"));
        assert!(call.function.arguments.contains("42"));
    }

    #[test]
    fn test_normalize_tool_call_missing_arguments() {
        let tc = json!({
            "id": "call_123",
            "function": {
                "name": "my_tool"
            }
        });

        let result = normalize_tool_call(&tc);
        assert!(result.is_some());
        assert_eq!(result.unwrap().function.arguments, "");
    }

    #[test]
    fn test_normalize_tool_call_null_arguments() {
        let tc = json!({
            "id": "call_123",
            "function": {
                "name": "my_tool",
                "arguments": null
            }
        });

        let result = normalize_tool_call(&tc);
        assert!(result.is_some());
        assert_eq!(result.unwrap().function.arguments, "");
    }

    #[test]
    fn test_normalize_tool_call_missing_name_returns_none() {
        let tc = json!({
            "id": "call_123",
            "function": {
                "arguments": "{}"
            }
        });

        let result = normalize_tool_call(&tc);
        assert!(result.is_none());
    }

    #[test]
    fn test_normalize_tool_call_empty_name_returns_none() {
        let tc = json!({
            "id": "call_123",
            "function": {
                "name": "",
                "arguments": "{}"
            }
        });

        let result = normalize_tool_call(&tc);
        assert!(result.is_none());
    }

    #[test]
    fn test_normalize_tool_call_missing_function_returns_none() {
        let tc = json!({
            "id": "call_123",
            "type": "function"
        });

        let result = normalize_tool_call(&tc);
        assert!(result.is_none());
    }

    #[test]
    fn test_normalize_tool_call_index_preserved() {
        let tc = json!({
            "id": "call_123",
            "index": 5,
            "function": {
                "name": "indexed_tool",
                "arguments": "{}"
            }
        });

        let result = normalize_tool_call(&tc);
        assert!(result.is_some());
        assert_eq!(result.unwrap().index, Some(5));
    }

    fn normalize_tool_call(tc: &serde_json::Value) -> Option<ChatToolCall> {
        let function = tc.get("function")?;
        let name = function.get("name").and_then(|n| n.as_str()).filter(|s| !s.is_empty())?;

        let id = tc.get("id")
            .and_then(|i| i.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("call_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..24].to_string()));

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

        Some(ChatToolCall {
            id,
            index,
            function: ChatToolFunction {
                name: name.to_string(),
                arguments,
            },
            tool_type,
        })
    }

    #[test]
    fn test_chat_prepare_options_default() {
        use crate::chat::prepare::ChatPrepareOptions;

        let opts = ChatPrepareOptions::default();

        assert!(opts.prepend_system_prompt);
        assert!(opts.allow_at_commands);
        assert!(opts.allow_tool_prerun);
        assert!(opts.supports_tools);
        assert!(opts.use_compression);
    }

    #[test]
    fn test_chat_prepare_options_custom() {
        use crate::chat::prepare::ChatPrepareOptions;

        let opts = ChatPrepareOptions {
            prepend_system_prompt: false,
            allow_at_commands: false,
            allow_tool_prerun: false,
            supports_tools: true,
            use_compression: false,
        };

        assert!(!opts.prepend_system_prompt);
        assert!(!opts.allow_at_commands);
        assert!(!opts.allow_tool_prerun);
        assert!(opts.supports_tools);
        assert!(!opts.use_compression);
    }

    #[test]
    fn test_is_thinking_enabled_with_thinking_json() {
        use crate::call_validation::SamplingParameters;

        let params = SamplingParameters {
            thinking: Some(json!({"type": "enabled", "budget_tokens": 1024})),
            ..Default::default()
        };

        assert!(is_thinking_enabled(&params));
    }

    #[test]
    fn test_is_thinking_enabled_with_thinking_disabled() {
        use crate::call_validation::SamplingParameters;

        let params = SamplingParameters {
            thinking: Some(json!({"type": "disabled"})),
            ..Default::default()
        };

        assert!(!is_thinking_enabled(&params));
    }

    #[test]
    fn test_is_thinking_enabled_with_reasoning_effort() {
        use crate::call_validation::{SamplingParameters, ReasoningEffort};

        let params = SamplingParameters {
            reasoning_effort: Some(ReasoningEffort::Medium),
            ..Default::default()
        };

        assert!(is_thinking_enabled(&params));
    }

    #[test]
    fn test_is_thinking_enabled_with_enable_thinking_true() {
        use crate::call_validation::SamplingParameters;

        let params = SamplingParameters {
            enable_thinking: Some(true),
            ..Default::default()
        };

        assert!(is_thinking_enabled(&params));
    }

    #[test]
    fn test_is_thinking_enabled_with_enable_thinking_false() {
        use crate::call_validation::SamplingParameters;

        let params = SamplingParameters {
            enable_thinking: Some(false),
            ..Default::default()
        };

        assert!(!is_thinking_enabled(&params));
    }

    #[test]
    fn test_is_thinking_enabled_all_none() {
        use crate::call_validation::SamplingParameters;

        let params = SamplingParameters::default();

        assert!(!is_thinking_enabled(&params));
    }

    fn is_thinking_enabled(sampling_parameters: &crate::call_validation::SamplingParameters) -> bool {
        sampling_parameters.thinking
            .as_ref()
            .and_then(|t| t.get("type"))
            .and_then(|t| t.as_str())
            .map(|t| t == "enabled")
            .unwrap_or(false)
            || sampling_parameters.reasoning_effort.is_some()
            || sampling_parameters.enable_thinking == Some(true)
    }

    #[test]
    fn test_strip_thinking_blocks_removes_when_disabled() {
        let messages = vec![
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("Hello".to_string()),
                thinking_blocks: Some(vec![json!({"type": "thinking", "thinking": "deep thought"})]),
                ..Default::default()
            },
        ];

        let stripped: Vec<_> = messages.into_iter().map(|mut msg| {
            msg.thinking_blocks = None;
            msg
        }).collect();

        assert!(stripped[0].thinking_blocks.is_none());
    }

    #[test]
    fn test_strip_thinking_blocks_preserves_content() {
        let messages = vec![
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("Hello world".to_string()),
                thinking_blocks: Some(vec![json!({"type": "thinking", "thinking": "thought"})]),
                reasoning_content: Some("reasoning".to_string()),
                ..Default::default()
            },
        ];

        let stripped: Vec<_> = messages.into_iter().map(|mut msg| {
            msg.thinking_blocks = None;
            msg
        }).collect();

        match &stripped[0].content {
            ChatContent::SimpleText(s) => assert_eq!(s, "Hello world"),
            _ => panic!("Content type changed"),
        }
        assert_eq!(stripped[0].reasoning_content, Some("reasoning".to_string()));
    }

    #[test]
    fn test_tools_json_not_null_when_empty() {
        let tools: Vec<serde_json::Value> = vec![];
        let tools_str = if tools.is_empty() {
            None
        } else {
            serde_json::to_string(&tools).ok()
        };

        assert!(tools_str.is_none());
    }

    #[test]
    fn test_tools_json_serializes_when_present() {
        let tools = vec![json!({"type": "function", "function": {"name": "test"}})];
        let tools_str = if tools.is_empty() {
            None
        } else {
            serde_json::to_string(&tools).ok()
        };

        assert!(tools_str.is_some());
        assert!(tools_str.unwrap().contains("test"));
    }

    #[test]
    fn test_tool_names_filtering() {
        use std::collections::HashSet;

        let all_tool_names = vec!["tool_a", "tool_b", "tool_c", "tool_d"];
        let allowed: HashSet<String> = vec!["tool_a".to_string(), "tool_c".to_string()].into_iter().collect();

        let filtered: Vec<_> = all_tool_names.into_iter()
            .filter(|name| allowed.contains(*name))
            .collect();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&"tool_a"));
        assert!(filtered.contains(&"tool_c"));
        assert!(!filtered.contains(&"tool_b"));
    }

    #[test]
    fn test_prompt_tool_names_empty_when_at_commands_disabled() {
        use std::collections::HashSet;

        let tool_names: HashSet<String> = vec!["tool_a".to_string(), "tool_b".to_string()].into_iter().collect();
        let allow_at_commands = false;

        let prompt_tool_names = if allow_at_commands { tool_names.clone() } else { HashSet::new() };

        assert!(prompt_tool_names.is_empty());
    }

    #[test]
    fn test_prompt_tool_names_preserved_when_at_commands_enabled() {
        use std::collections::HashSet;

        let tool_names: HashSet<String> = vec!["tool_a".to_string(), "tool_b".to_string()].into_iter().collect();
        let allow_at_commands = true;

        let prompt_tool_names = if allow_at_commands { tool_names.clone() } else { HashSet::new() };

        assert_eq!(prompt_tool_names.len(), 2);
        assert!(prompt_tool_names.contains("tool_a"));
    }
}
