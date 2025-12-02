#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use crate::call_validation::{ChatContent, ChatMessage, ChatToolCall, ChatToolFunction};
    use crate::scratchpads::passthrough_convert_messages::convert_messages_to_openai_format;
    use crate::scratchpads::multimodality::MultimodalElement;

    fn create_thinking_block(thinking: &str, signature: &str) -> Value {
        json!({
            "type": "thinking",
            "thinking": thinking,
            "signature": signature
        })
    }

    fn create_redacted_thinking_block() -> Value {
        json!({
            "type": "redacted_thinking",
            "data": "redacted_data_here"
        })
    }

    // Test 1: Assistant message with thinking blocks should have them in content array
    #[test]
    fn test_thinking_blocks_in_content_array() {
        let thinking_blocks = vec![
            create_thinking_block("Let me analyze this problem...", "sig123"),
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("Here is my response.".to_string()),
            thinking_blocks: Some(thinking_blocks.clone()),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        // Content should be an array
        let content = value.get("content").expect("content field missing");
        assert!(content.is_array(), "content should be an array when thinking_blocks present");

        let content_array = content.as_array().unwrap();

        // First element should be thinking block
        assert_eq!(content_array[0].get("type").unwrap(), "thinking");
        assert_eq!(content_array[0].get("thinking").unwrap(), "Let me analyze this problem...");
        assert_eq!(content_array[0].get("signature").unwrap(), "sig123");

        // Second element should be text
        assert_eq!(content_array[1].get("type").unwrap(), "text");
        assert_eq!(content_array[1].get("text").unwrap(), "Here is my response.");

        // There should be NO separate thinking_blocks field
        assert!(value.get("thinking_blocks").is_none(), "thinking_blocks should NOT be a separate field");
    }

    // Test 2: Assistant message with thinking blocks but empty text content
    #[test]
    fn test_thinking_blocks_with_empty_text() {
        let thinking_blocks = vec![
            create_thinking_block("Thinking without visible output...", "sig456"),
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("".to_string()),
            thinking_blocks: Some(thinking_blocks.clone()),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        let content = value.get("content").expect("content field missing");
        assert!(content.is_array(), "content should be an array");

        let content_array = content.as_array().unwrap();

        // Should only have thinking block, no empty text element
        assert_eq!(content_array.len(), 1, "should only have thinking block, no empty text");
        assert_eq!(content_array[0].get("type").unwrap(), "thinking");
    }

    // Test 3: Assistant message with thinking blocks and tool_calls
    #[test]
    fn test_thinking_blocks_with_tool_calls() {
        let thinking_blocks = vec![
            create_thinking_block("I need to use a tool here...", "sig789"),
        ];

        let tool_calls = vec![
            ChatToolCall {
                id: "call_123".to_string(),
                function: ChatToolFunction {
                    name: "search".to_string(),
                    arguments: r#"{"query": "test"}"#.to_string(),
                },
                tool_type: "function".to_string(),
            }
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("".to_string()),
            thinking_blocks: Some(thinking_blocks.clone()),
            tool_calls: Some(tool_calls),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        // Content should have thinking blocks
        let content = value.get("content").expect("content field missing");
        assert!(content.is_array());
        let content_array = content.as_array().unwrap();
        assert_eq!(content_array[0].get("type").unwrap(), "thinking");

        // tool_calls should still be present as a separate field (that's correct)
        assert!(value.get("tool_calls").is_some(), "tool_calls should be present");

        // thinking_blocks should NOT be a separate field
        assert!(value.get("thinking_blocks").is_none());
    }

    // Test 4: Multiple thinking blocks
    #[test]
    fn test_multiple_thinking_blocks() {
        let thinking_blocks = vec![
            create_thinking_block("First thought...", "sig1"),
            create_redacted_thinking_block(),
            create_thinking_block("Third thought...", "sig3"),
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("Final answer.".to_string()),
            thinking_blocks: Some(thinking_blocks.clone()),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        let content = value.get("content").unwrap().as_array().unwrap();

        // Should have 4 elements: 3 thinking blocks + 1 text
        assert_eq!(content.len(), 4);
        assert_eq!(content[0].get("type").unwrap(), "thinking");
        assert_eq!(content[1].get("type").unwrap(), "redacted_thinking");
        assert_eq!(content[2].get("type").unwrap(), "thinking");
        assert_eq!(content[3].get("type").unwrap(), "text");
    }

    // Test 5: Message without thinking blocks should work normally
    #[test]
    fn test_no_thinking_blocks_simple_text() {
        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("Just a simple response.".to_string()),
            thinking_blocks: None,
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        // Content should be a simple string, not an array
        let content = value.get("content").expect("content field missing");
        assert!(content.is_string(), "content should be a string when no thinking_blocks");
        assert_eq!(content.as_str().unwrap(), "Just a simple response.");
    }

    // Test 6: Full conversation with thinking in convert_messages_to_openai_format
    #[test]
    fn test_conversation_with_thinking_blocks() {
        let thinking_blocks = vec![
            create_thinking_block("Analyzing the user request...", "sig_analyze"),
        ];

        let messages = vec![
            ChatMessage::new("user".to_string(), "What is 2+2?".to_string()),
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("The answer is 4.".to_string()),
                thinking_blocks: Some(thinking_blocks),
                ..Default::default()
            },
            ChatMessage::new("user".to_string(), "Thanks!".to_string()),
        ];

        let style = Some("openai".to_string());
        let output = convert_messages_to_openai_format(messages, &style, "anthropic/claude-3-5-sonnet");

        assert_eq!(output.len(), 3);

        // First message: user
        assert_eq!(output[0].get("role").unwrap(), "user");
        assert_eq!(output[0].get("content").unwrap(), "What is 2+2?");

        // Second message: assistant with thinking
        assert_eq!(output[1].get("role").unwrap(), "assistant");
        let assistant_content = output[1].get("content").unwrap().as_array().unwrap();
        assert_eq!(assistant_content[0].get("type").unwrap(), "thinking");
        assert_eq!(assistant_content[1].get("type").unwrap(), "text");
        assert_eq!(assistant_content[1].get("text").unwrap(), "The answer is 4.");

        // Third message: user
        assert_eq!(output[2].get("role").unwrap(), "user");
    }

    // Test 7: Tool use loop with thinking - simulates real Anthropic workflow
    // When assistant has thinking_blocks + tool_calls (even with empty visible content),
    // thinking blocks MUST be preserved - Anthropic requires them during multi-turn conversations.
    #[test]
    fn test_tool_use_loop_with_thinking() {
        let thinking_blocks = vec![
            create_thinking_block("I should search for this information...", "sig_tool"),
        ];

        let tool_calls = vec![
            ChatToolCall {
                id: "call_abc".to_string(),
                function: ChatToolFunction {
                    name: "web_search".to_string(),
                    arguments: r#"{"query": "rust programming"}"#.to_string(),
                },
                tool_type: "function".to_string(),
            }
        ];

        // This is the critical case: empty content + thinking_blocks + tool_calls
        // The hack should NOT strip thinking_blocks when tool_calls are present
        let messages = vec![
            ChatMessage::new("user".to_string(), "Search for Rust programming".to_string()),
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("".to_string()), // Empty content!
                thinking_blocks: Some(thinking_blocks),
                tool_calls: Some(tool_calls),
                ..Default::default()
            },
            ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText("Search results: Rust is a systems programming language...".to_string()),
                tool_call_id: "call_abc".to_string(),
                ..Default::default()
            },
        ];

        let style = Some("openai".to_string());
        let output = convert_messages_to_openai_format(messages, &style, "anthropic/claude-3-5-sonnet");

        // Check assistant message has thinking in content array
        let assistant_msg = &output[1];
        assert_eq!(assistant_msg.get("role").unwrap(), "assistant");

        let content = assistant_msg.get("content").unwrap();
        assert!(content.is_array(), "assistant content must be array with thinking blocks");

        let content_array = content.as_array().unwrap();
        assert!(content_array.len() >= 1);
        assert_eq!(content_array[0].get("type").unwrap(), "thinking");

        // Verify tool_calls is present
        assert!(assistant_msg.get("tool_calls").is_some());

        // Verify NO thinking_blocks field
        assert!(assistant_msg.get("thinking_blocks").is_none());
    }

    // Test 8: Verify the hack for interrupted thinking still works
    #[test]
    fn test_interrupted_thinking_hack() {
        // This simulates a case where thinking was interrupted mid-stream
        // The hack in convert_messages_to_openai_format should replace the message
        let thinking_blocks = vec![
            create_thinking_block("Partial thought that was interru", "partial_sig"),
        ];

        let messages = vec![
            ChatMessage::new("user".to_string(), "Hello".to_string()),
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("".to_string()), // Empty content
                thinking_blocks: Some(thinking_blocks),
                tool_calls: None, // No tool calls
                ..Default::default()
            },
        ];

        let style = Some("openai".to_string());
        let output = convert_messages_to_openai_format(messages, &style, "anthropic/claude-3-5-sonnet");

        // The hack should replace the last assistant message
        let assistant_msg = &output[1];
        let content = assistant_msg.get("content").unwrap();

        // After the hack, content should be the replacement text, not thinking blocks
        assert!(content.is_string(), "hack should have replaced with simple text");
        assert!(content.as_str().unwrap().contains("interrupted"));
    }

    // Test 9: Verify JSON structure matches Anthropic API expectations
    #[test]
    fn test_anthropic_api_json_structure() {
        let thinking_blocks = vec![
            json!({
                "type": "thinking",
                "thinking": "Deep analysis here...",
                "signature": "abc123xyz"
            }),
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("Based on my analysis...".to_string()),
            thinking_blocks: Some(thinking_blocks),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        // Serialize to JSON string and parse back to verify structure
        let json_str = serde_json::to_string(&value).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        // Verify the exact structure Anthropic expects
        assert_eq!(parsed.get("role").unwrap(), "assistant");

        let content = parsed.get("content").unwrap().as_array().unwrap();

        // First element must be thinking (Anthropic requirement)
        let first = &content[0];
        assert!(
            first.get("type").unwrap() == "thinking" || first.get("type").unwrap() == "redacted_thinking",
            "First content element must be thinking or redacted_thinking"
        );

        // Last visible element should be text (if present)
        let last = &content[content.len() - 1];
        assert_eq!(last.get("type").unwrap(), "text");
    }

    // Test 10: Edge case - thinking blocks with tool_use and visible text
    #[test]
    fn test_thinking_with_tool_use_and_text() {
        let thinking_blocks = vec![
            create_thinking_block("Let me think and explain...", "sig_explain"),
        ];

        let tool_calls = vec![
            ChatToolCall {
                id: "call_xyz".to_string(),
                function: ChatToolFunction {
                    name: "calculator".to_string(),
                    arguments: r#"{"expression": "2+2"}"#.to_string(),
                },
                tool_type: "function".to_string(),
            }
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("I'll calculate that for you.".to_string()),
            thinking_blocks: Some(thinking_blocks),
            tool_calls: Some(tool_calls),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        let content = value.get("content").unwrap().as_array().unwrap();

        // Structure should be: [thinking, text]
        assert_eq!(content.len(), 2);
        assert_eq!(content[0].get("type").unwrap(), "thinking");
        assert_eq!(content[1].get("type").unwrap(), "text");
        assert_eq!(content[1].get("text").unwrap(), "I'll calculate that for you.");

        // tool_calls separate field
        assert!(value.get("tool_calls").is_some());

        // NO thinking_blocks field
        assert!(value.get("thinking_blocks").is_none());
    }

    // ==================== ADDITIONAL CORNER CASES ====================

    // Test 11: Empty thinking_blocks vector (Some but empty)
    #[test]
    fn test_empty_thinking_blocks_vector() {
        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("Response with empty thinking vec.".to_string()),
            thinking_blocks: Some(vec![]), // Empty vector
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        let content = value.get("content").unwrap();
        // With empty thinking_blocks, content should still be array format
        assert!(content.is_array());
        let arr = content.as_array().unwrap();
        // Should only have the text element
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].get("type").unwrap(), "text");
    }

    // Test 12: Thinking blocks with multimodal content
    #[test]
    fn test_thinking_blocks_with_multimodal_content() {
        let thinking_blocks = vec![
            create_thinking_block("Analyzing the image...", "sig_img"),
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::Multimodal(vec![
                MultimodalElement::new("text".to_string(), "Here is my analysis.".to_string()).unwrap(),
            ]),
            thinking_blocks: Some(thinking_blocks),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        let content = value.get("content").unwrap().as_array().unwrap();
        // First should be thinking, then multimodal elements
        assert_eq!(content[0].get("type").unwrap(), "thinking");
        assert_eq!(content[1].get("type").unwrap(), "text");
    }

    // Test 13: Multiple tool calls with thinking
    #[test]
    fn test_multiple_tool_calls_with_thinking() {
        let thinking_blocks = vec![
            create_thinking_block("I need multiple tools...", "sig_multi"),
        ];

        let tool_calls = vec![
            ChatToolCall {
                id: "call_1".to_string(),
                function: ChatToolFunction {
                    name: "search".to_string(),
                    arguments: r#"{"q": "a"}"#.to_string(),
                },
                tool_type: "function".to_string(),
            },
            ChatToolCall {
                id: "call_2".to_string(),
                function: ChatToolFunction {
                    name: "read_file".to_string(),
                    arguments: r#"{"path": "b"}"#.to_string(),
                },
                tool_type: "function".to_string(),
            },
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("".to_string()),
            thinking_blocks: Some(thinking_blocks),
            tool_calls: Some(tool_calls),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        // Thinking in content
        let content = value.get("content").unwrap().as_array().unwrap();
        assert_eq!(content[0].get("type").unwrap(), "thinking");

        // Multiple tool_calls
        let tc = value.get("tool_calls").unwrap().as_array().unwrap();
        assert_eq!(tc.len(), 2);
    }

    // Test 14: Whitespace-only content with thinking (should be treated as empty)
    #[test]
    fn test_whitespace_only_content_with_thinking() {
        let thinking_blocks = vec![
            create_thinking_block("Thinking...", "sig_ws"),
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("   \n\t  ".to_string()), // Whitespace only
            thinking_blocks: Some(thinking_blocks),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        let content = value.get("content").unwrap().as_array().unwrap();
        // Should have thinking + text (whitespace is not filtered in into_value)
        assert!(content.len() >= 1);
        assert_eq!(content[0].get("type").unwrap(), "thinking");
    }

    // Test 15: Very long thinking content
    #[test]
    fn test_long_thinking_content() {
        let long_thinking = "A".repeat(10000);
        let thinking_blocks = vec![
            create_thinking_block(&long_thinking, "sig_long"),
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("Short response.".to_string()),
            thinking_blocks: Some(thinking_blocks),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        let content = value.get("content").unwrap().as_array().unwrap();
        assert_eq!(content[0].get("type").unwrap(), "thinking");
        assert_eq!(content[0].get("thinking").unwrap().as_str().unwrap().len(), 10000);
    }

    // Test 16: Unicode in thinking blocks
    #[test]
    fn test_unicode_in_thinking_blocks() {
        let thinking_blocks = vec![
            create_thinking_block("思考中... 🤔 Размышляю...", "sig_unicode"),
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("Response with émojis 🎉".to_string()),
            thinking_blocks: Some(thinking_blocks),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        let content = value.get("content").unwrap().as_array().unwrap();
        assert!(content[0].get("thinking").unwrap().as_str().unwrap().contains("思考中"));
        assert!(content[1].get("text").unwrap().as_str().unwrap().contains("🎉"));
    }

    // Test 17: Thinking blocks on non-assistant role (edge case - should still work)
    #[test]
    fn test_thinking_blocks_on_user_role() {
        // This shouldn't happen in practice, but test the behavior
        let thinking_blocks = vec![
            create_thinking_block("User thinking?", "sig_user"),
        ];

        let message = ChatMessage {
            role: "user".to_string(),
            content: ChatContent::SimpleText("User message.".to_string()),
            thinking_blocks: Some(thinking_blocks),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        // Should still serialize with thinking in content array
        let content = value.get("content").unwrap().as_array().unwrap();
        assert_eq!(content[0].get("type").unwrap(), "thinking");
    }

    // Test 18: Conversation with multiple assistant messages having thinking
    #[test]
    fn test_multi_turn_with_thinking_preserved() {
        let messages = vec![
            ChatMessage::new("user".to_string(), "First question".to_string()),
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("First answer.".to_string()),
                thinking_blocks: Some(vec![create_thinking_block("First thought", "sig1")]),
                ..Default::default()
            },
            ChatMessage::new("user".to_string(), "Second question".to_string()),
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("Second answer.".to_string()),
                thinking_blocks: Some(vec![create_thinking_block("Second thought", "sig2")]),
                ..Default::default()
            },
        ];

        let style = Some("openai".to_string());
        let output = convert_messages_to_openai_format(messages, &style, "anthropic/claude-3-5-sonnet");

        // Both assistant messages should have thinking in content array
        let first_asst = &output[1];
        let second_asst = &output[3];

        assert!(first_asst.get("content").unwrap().is_array());
        assert!(second_asst.get("content").unwrap().is_array());

        let first_content = first_asst.get("content").unwrap().as_array().unwrap();
        let second_content = second_asst.get("content").unwrap().as_array().unwrap();

        assert_eq!(first_content[0].get("thinking").unwrap(), "First thought");
        assert_eq!(second_content[0].get("thinking").unwrap(), "Second thought");
    }

    // Test 19: Tool result after thinking+tool_call assistant message
    #[test]
    fn test_full_tool_loop_sequence() {
        let thinking1 = vec![create_thinking_block("Need to search", "sig_s1")];
        let thinking2 = vec![create_thinking_block("Got results, analyzing", "sig_s2")];

        let tool_calls = vec![ChatToolCall {
            id: "call_search".to_string(),
            function: ChatToolFunction {
                name: "search".to_string(),
                arguments: r#"{"q":"test"}"#.to_string(),
            },
            tool_type: "function".to_string(),
        }];

        let messages = vec![
            ChatMessage::new("user".to_string(), "Search for test".to_string()),
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("".to_string()),
                thinking_blocks: Some(thinking1),
                tool_calls: Some(tool_calls),
                ..Default::default()
            },
            ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText("Search results here".to_string()),
                tool_call_id: "call_search".to_string(),
                ..Default::default()
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("Based on results...".to_string()),
                thinking_blocks: Some(thinking2),
                ..Default::default()
            },
        ];

        let style = Some("openai".to_string());
        let output = convert_messages_to_openai_format(messages, &style, "anthropic/claude-3-5-sonnet");

        assert_eq!(output.len(), 4);

        // First assistant: thinking + tool_calls
        let asst1 = &output[1];
        assert!(asst1.get("content").unwrap().is_array());
        assert!(asst1.get("tool_calls").is_some());

        // Tool result
        assert_eq!(output[2].get("role").unwrap(), "tool");

        // Second assistant: thinking + text
        let asst2 = &output[3];
        let content2 = asst2.get("content").unwrap().as_array().unwrap();
        assert_eq!(content2[0].get("type").unwrap(), "thinking");
        assert_eq!(content2[1].get("type").unwrap(), "text");
    }

    // Test 20: Gemini model (different model_id) with thinking blocks
    #[test]
    fn test_thinking_blocks_with_gemini_model() {
        let thinking_blocks = vec![
            create_thinking_block("Gemini thinking...", "sig_gem"),
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("Gemini response.".to_string()),
            thinking_blocks: Some(thinking_blocks),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "google_gemini/gemini-2.0-flash");

        // Should still work the same way
        let content = value.get("content").unwrap().as_array().unwrap();
        assert_eq!(content[0].get("type").unwrap(), "thinking");
        assert_eq!(content[1].get("type").unwrap(), "text");
    }

    // Test 21: Only redacted thinking blocks (no regular thinking)
    #[test]
    fn test_only_redacted_thinking_blocks() {
        let thinking_blocks = vec![
            create_redacted_thinking_block(),
            create_redacted_thinking_block(),
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("Response after redacted thinking.".to_string()),
            thinking_blocks: Some(thinking_blocks),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        let content = value.get("content").unwrap().as_array().unwrap();
        assert_eq!(content.len(), 3);
        assert_eq!(content[0].get("type").unwrap(), "redacted_thinking");
        assert_eq!(content[1].get("type").unwrap(), "redacted_thinking");
        assert_eq!(content[2].get("type").unwrap(), "text");
    }

    // Test 22: Interrupted thinking hack should NOT trigger for non-last assistant
    #[test]
    fn test_hack_only_affects_last_assistant() {
        let thinking_blocks = vec![
            create_thinking_block("Early thinking", "sig_early"),
        ];

        let messages = vec![
            ChatMessage::new("user".to_string(), "Q1".to_string()),
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("".to_string()), // Empty!
                thinking_blocks: Some(thinking_blocks),
                tool_calls: None,
                ..Default::default()
            },
            ChatMessage::new("user".to_string(), "Q2".to_string()),
            ChatMessage::new("assistant".to_string(), "A2".to_string()),
        ];

        let style = Some("openai".to_string());
        let output = convert_messages_to_openai_format(messages, &style, "anthropic/claude-3-5-sonnet");

        // First assistant is NOT the last, so hack should not apply
        // But wait - the hack only looks at the LAST assistant message
        // So the first assistant with empty content + thinking should be preserved
        let first_asst = &output[1];
        let content = first_asst.get("content").unwrap();

        // This should have thinking blocks preserved (not replaced by hack)
        assert!(content.is_array(), "First assistant should preserve thinking");
    }

    // Test 23: Special characters in signature
    #[test]
    fn test_special_chars_in_signature() {
        let thinking_blocks = vec![
            json!({
                "type": "thinking",
                "thinking": "Normal thinking",
                "signature": "sig+/=ABC123xyz=="
            }),
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("Response.".to_string()),
            thinking_blocks: Some(thinking_blocks),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        let content = value.get("content").unwrap().as_array().unwrap();
        assert_eq!(content[0].get("signature").unwrap(), "sig+/=ABC123xyz==");
    }

    // Test 24: Newlines and special formatting in thinking content
    #[test]
    fn test_newlines_in_thinking() {
        let thinking_blocks = vec![
            create_thinking_block("Line 1\nLine 2\n\nLine 4\tTabbed", "sig_nl"),
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("Response.".to_string()),
            thinking_blocks: Some(thinking_blocks),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        let content = value.get("content").unwrap().as_array().unwrap();
        let thinking_text = content[0].get("thinking").unwrap().as_str().unwrap();
        assert!(thinking_text.contains("\n"));
        assert!(thinking_text.contains("\t"));
    }

    // Test 25: Verify no data loss in round-trip serialization
    #[test]
    fn test_serialization_roundtrip_integrity() {
        let thinking_blocks = vec![
            create_thinking_block("Complex thinking with \"quotes\" and \\backslashes\\", "sig_rt"),
        ];

        let message = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("Response with \"quotes\".".to_string()),
            thinking_blocks: Some(thinking_blocks),
            ..Default::default()
        };

        let style = Some("openai".to_string());
        let value = message.into_value(&style, "anthropic/claude-3-5-sonnet");

        // Serialize to string
        let json_str = serde_json::to_string(&value).unwrap();

        // Parse back
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        let content = parsed.get("content").unwrap().as_array().unwrap();
        let thinking = content[0].get("thinking").unwrap().as_str().unwrap();

        assert!(thinking.contains("\"quotes\""));
        assert!(thinking.contains("\\backslashes\\"));
    }
}
