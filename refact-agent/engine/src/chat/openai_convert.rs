use itertools::Itertools;
use serde_json::Value;
use tracing::{error, warn};
use crate::call_validation::{ChatContent, ChatMessage, ContextFile, DiffChunk};

// Note: This function always produces OpenAI-compatible format.
// When going through litellm proxy, litellm handles the conversion to Anthropic native format.
// Tool results use role="tool" with tool_call_id (OpenAI format), not tool_result blocks.
// Thinking blocks are preserved in assistant messages' content arrays for Anthropic models.
pub fn convert_messages_to_openai_format(mut messages: Vec<ChatMessage>, style: &Option<String>, model_id: &str) -> Vec<Value> {
    if let Some(last_asst_idx) = messages.iter().rposition(|m| m.role == "assistant") {
        let has_only_thinking = messages[last_asst_idx]
            .content
            .content_text_only()
            .trim()
            .is_empty()
            && messages[last_asst_idx]
                .thinking_blocks
                .as_ref()
                .map_or(false, |v| !v.is_empty())
            && messages[last_asst_idx]
                .tool_calls
                .as_ref()
                .map_or(true, |v| v.is_empty());
        if has_only_thinking {
            let m = &mut messages[last_asst_idx];
            m.content = ChatContent::SimpleText(
                "Previous reasoning was interrupted; continuing from here.".to_string(),
            );
            m.thinking_blocks = None;
        }
    }

    let mut results = vec![];
    let mut delay_images = vec![];

    let flush_delayed_images = |results: &mut Vec<Value>, delay_images: &mut Vec<Value>| {
        results.extend(delay_images.clone());
        delay_images.clear();
    };

    for msg in messages {
        if msg.role == "tool" {
            // Always use OpenAI format for tool results.
            // Litellm will convert to Anthropic native format if needed.
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
                    results.push(msg_cloned.into_value(&style, model_id));
                    if !images.is_empty() {
                        let msg_img = ChatMessage {
                            role: "user".to_string(),
                            content: ChatContent::Multimodal(images.into_iter().cloned().collect()),
                            ..Default::default()
                        };
                        delay_images.push(msg_img.into_value(&style, model_id));
                    }
                },
                ChatContent::SimpleText(_) => {
                    results.push(msg.into_value(&style, model_id));
                },
                ChatContent::ContextFiles(_) => {
                    // Context files as tool results - pass through
                    results.push(msg.into_value(&style, model_id));
                }
            }

        } else if msg.role == "assistant" || msg.role == "system" {
            flush_delayed_images(&mut results, &mut delay_images);
            results.push(msg.into_value(&style, model_id));

        } else if msg.role == "user" {
            flush_delayed_images(&mut results, &mut delay_images);
            results.push(msg.into_value(&style, model_id));

        } else if msg.role == "diff" {
            // Always use OpenAI format for diff results (as tool role).
            // Litellm will convert to Anthropic native format if needed.
            let extra_message = match serde_json::from_str::<Vec<DiffChunk>>(&msg.content.content_text_only()) {
                Ok(chunks) => {
                    if chunks.is_empty() {
                        "Nothing has changed.".to_string()
                    } else {
                        chunks.iter()
                            .filter(|x| !x.application_details.is_empty())
                            .map(|x| x.application_details.clone())
                            .join("\n")
                    }
                },
                Err(_) => "".to_string()
            };
            let content_text = format!("The operation has succeeded.\n{extra_message}");
            let tool_msg = ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText(content_text),
                tool_calls: None,
                tool_call_id: msg.tool_call_id.clone(),
                ..Default::default()
            };
            results.push(tool_msg.into_value(&style, model_id));

        } else if msg.role == "plain_text" || msg.role == "cd_instruction" {
            flush_delayed_images(&mut results, &mut delay_images);
            results.push(ChatMessage::new(
                "user".to_string(),
                msg.content.content_text_only(),
            ).into_value(&style, model_id));

        } else if msg.role == "context_file" {
            flush_delayed_images(&mut results, &mut delay_images);
            // Handle both new structured format and legacy JSON string format
            let context_files: Vec<ContextFile> = match &msg.content {
                ChatContent::ContextFiles(files) => files.clone(),
                ChatContent::SimpleText(text) => {
                    // Legacy: try to parse as JSON
                    match serde_json::from_str::<Vec<ContextFile>>(text) {
                        Ok(files) => files,
                        Err(e) => {
                            error!("error parsing context file JSON: {}", e);
                            continue;
                        }
                    }
                },
                ChatContent::Multimodal(_) => {
                    error!("unexpected multimodal content for context_file role");
                    continue;
                }
            };
            for context_file in context_files {
                results.push(ChatMessage::new(
                    "user".to_string(),
                    format!("{}:{}-{}\n```\n{}```",
                            context_file.file_name,
                            context_file.line1,
                            context_file.line2,
                            context_file.file_content),
                ).into_value(&style, model_id));
            }
        } else {
            warn!("unknown role: {}", msg.role);
        }
    }
    flush_delayed_images(&mut results, &mut delay_images);

    results
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_validation::{ChatContent, ChatMessage};
    use serde_json::json;
    use crate::scratchpads::multimodality::MultimodalElement;

    const TEST_PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

    fn style() -> Option<String> {
        Some("openai".to_string())
    }

    #[test]
    fn test_convert_messages_to_openai_format() {
        let messages = vec![
            ChatMessage::new("user".to_string(), "user".to_string()),
            ChatMessage::new("assistant".to_string(), "assistant".to_string()),
            ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::Multimodal(vec![
                    MultimodalElement::new("text".to_string(), "text".to_string()).unwrap(),
                    MultimodalElement::new("image/png".to_string(), TEST_PNG_1X1.to_string()).unwrap(),
                ]),
                ..Default::default()
            },
            ChatMessage::new("plain_text".to_string(), "plain_text".to_string()),
            ChatMessage::new("user".to_string(), "user".to_string()),
            ChatMessage::new("assistant".to_string(), "assistant".to_string()),
            ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::Multimodal(vec![
                    MultimodalElement::new("text".to_string(), "text".to_string()).unwrap(),
                    MultimodalElement::new("image/png".to_string(), TEST_PNG_1X1.to_string()).unwrap(),
                ]),
                ..Default::default()
            },
            ChatMessage::new("plain_text".to_string(), "plain_text".to_string()),
        ];

        let expected_output = vec![
            json!({"role": "user", "content": "user"}),
            json!({"role": "assistant", "content": "assistant"}),
            json!({"role": "tool", "content": "text"}),
            json!({"role": "user", "content": "plain_text"}),
            json!({"role": "user", "content": "IMAGE_HERE"}),
            json!({"role": "user", "content": "user"}),
            json!({"role": "assistant", "content": "assistant"}),
            json!({"role": "tool", "content": "text"}),
            json!({"role": "user", "content": "plain_text"}),
            json!({"role": "user", "content": "IMAGE_HERE"}),
        ];

        let roles_out_expected: Vec<_> = expected_output.iter()
            .map(|x| x.get("role").unwrap().as_str().unwrap().to_string())
            .collect();

        let output = convert_messages_to_openai_format(messages, &style(), "Refact/gpt-4o");
        let roles_out: Vec<_> = output.iter()
            .map(|x| x.get("role").unwrap().as_str().unwrap().to_string())
            .collect();

        assert_eq!(roles_out, roles_out_expected);
    }

    #[test]
    fn test_thinking_only_assistant_replaced() {
        let messages = vec![
            ChatMessage::new("user".to_string(), "hello".to_string()),
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("".to_string()),
                thinking_blocks: Some(vec![json!({"type": "thinking", "thinking": "deep thought"})]),
                ..Default::default()
            },
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        assert_eq!(output.len(), 2);
        let content = output[1].get("content").unwrap().as_str().unwrap();
        assert!(content.contains("Previous reasoning was interrupted"));
    }

    #[test]
    fn test_thinking_only_with_tool_calls_not_replaced() {
        let messages = vec![
            ChatMessage::new("user".to_string(), "hello".to_string()),
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("".to_string()),
                thinking_blocks: Some(vec![json!({"type": "thinking"})]),
                tool_calls: Some(vec![crate::call_validation::ChatToolCall {
                    id: "tc1".into(),
                    function: crate::call_validation::ChatToolFunction {
                        name: "test".into(),
                        arguments: "{}".into(),
                    },
                    tool_type: "function".into(),
                    index: None,
                }]),
                ..Default::default()
            },
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        let content = output[1].get("content");
        assert!(content.is_none() || content.unwrap().as_str().map(|s| s.is_empty()).unwrap_or(true)
            || content.unwrap().is_array());
    }

    #[test]
    fn test_thinking_with_content_not_replaced() {
        let messages = vec![
            ChatMessage::new("user".to_string(), "hello".to_string()),
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("actual content".to_string()),
                thinking_blocks: Some(vec![json!({"type": "thinking"})]),
                ..Default::default()
            },
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        let content = output[1].get("content").unwrap();
        assert!(!content.as_str().unwrap_or("").contains("Previous reasoning"));
    }

    #[test]
    fn test_diff_role_converts_to_tool() {
        let diff_content = serde_json::to_string(&vec![DiffChunk {
            file_name: "test.rs".into(),
            file_action: "edit".into(),
            line1: 1,
            line2: 10,
            lines_remove: "old".into(),
            lines_add: "new".into(),
            file_name_rename: None,
            is_file: true,
            application_details: "Applied successfully".into(),
        }]).unwrap();

        let messages = vec![
            ChatMessage {
                role: "diff".to_string(),
                content: ChatContent::SimpleText(diff_content),
                tool_call_id: "tc1".into(),
                ..Default::default()
            },
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].get("role").unwrap(), "tool");
        let content = output[0].get("content").unwrap().as_str().unwrap();
        assert!(content.contains("operation has succeeded"));
        assert!(content.contains("Applied successfully"));
    }

    #[test]
    fn test_diff_role_empty_chunks() {
        let messages = vec![
            ChatMessage {
                role: "diff".to_string(),
                content: ChatContent::SimpleText("[]".into()),
                tool_call_id: "tc1".into(),
                ..Default::default()
            },
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        let content = output[0].get("content").unwrap().as_str().unwrap();
        assert!(content.contains("Nothing has changed"));
    }

    #[test]
    fn test_diff_role_invalid_json() {
        let messages = vec![
            ChatMessage {
                role: "diff".to_string(),
                content: ChatContent::SimpleText("not json".into()),
                tool_call_id: "tc1".into(),
                ..Default::default()
            },
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].get("role").unwrap(), "tool");
    }

    fn make_context_file(name: &str, content: &str) -> ContextFile {
        ContextFile {
            file_name: name.into(),
            file_content: content.into(),
            line1: 1,
            line2: 1,
            symbols: vec![],
            gradient_type: -1,
            usefulness: 0.0,
            skip_pp: false,
        }
    }

    #[test]
    fn test_context_file_structured() {
        let files = vec![
            make_context_file("main.rs", "fn main() {}"),
            make_context_file("lib.rs", "pub mod x;"),
        ];
        let messages = vec![
            ChatMessage {
                role: "context_file".to_string(),
                content: ChatContent::ContextFiles(files),
                ..Default::default()
            },
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        assert_eq!(output.len(), 2);
        assert_eq!(output[0].get("role").unwrap(), "user");
        assert_eq!(output[1].get("role").unwrap(), "user");
        let content0 = output[0].get("content").unwrap().as_str().unwrap();
        assert!(content0.contains("main.rs"));
        assert!(content0.contains("fn main()"));
    }

    #[test]
    fn test_context_file_legacy_json() {
        let files = vec![make_context_file("test.py", "print('hi')")];
        let json_str = serde_json::to_string(&files).unwrap();
        let messages = vec![
            ChatMessage {
                role: "context_file".to_string(),
                content: ChatContent::SimpleText(json_str),
                ..Default::default()
            },
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        assert_eq!(output.len(), 1);
        assert!(output[0].get("content").unwrap().as_str().unwrap().contains("test.py"));
    }

    #[test]
    fn test_context_file_invalid_json_skipped() {
        let messages = vec![
            ChatMessage {
                role: "context_file".to_string(),
                content: ChatContent::SimpleText("not valid json".into()),
                ..Default::default()
            },
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        assert!(output.is_empty());
    }

    #[test]
    fn test_plain_text_converts_to_user() {
        let messages = vec![
            ChatMessage::new("plain_text".to_string(), "some instruction".to_string()),
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].get("role").unwrap(), "user");
        assert_eq!(output[0].get("content").unwrap(), "some instruction");
    }

    #[test]
    fn test_cd_instruction_converts_to_user() {
        let messages = vec![
            ChatMessage::new("cd_instruction".to_string(), "cd /path".to_string()),
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].get("role").unwrap(), "user");
    }

    #[test]
    fn test_system_message_preserved() {
        let messages = vec![
            ChatMessage::new("system".to_string(), "you are helpful".to_string()),
            ChatMessage::new("user".to_string(), "hi".to_string()),
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        assert_eq!(output.len(), 2);
        assert_eq!(output[0].get("role").unwrap(), "system");
        assert_eq!(output[0].get("content").unwrap(), "you are helpful");
    }

    #[test]
    fn test_tool_simple_text() {
        let messages = vec![
            ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText("tool result".into()),
                tool_call_id: "tc1".into(),
                ..Default::default()
            },
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].get("role").unwrap(), "tool");
        assert_eq!(output[0].get("content").unwrap(), "tool result");
    }

    #[test]
    fn test_tool_multimodal_no_text() {
        let messages = vec![
            ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::Multimodal(vec![
                    MultimodalElement::new("image/png".to_string(), TEST_PNG_1X1.to_string()).unwrap(),
                ]),
                tool_call_id: "tc1".into(),
                ..Default::default()
            },
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        assert_eq!(output.len(), 2);
        assert_eq!(output[0].get("role").unwrap(), "tool");
        assert!(output[0].get("content").unwrap().as_str().unwrap().contains("attached images"));
        assert_eq!(output[1].get("role").unwrap(), "user");
    }

    #[test]
    fn test_delayed_images_flushed_on_user() {
        let messages = vec![
            ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::Multimodal(vec![
                    MultimodalElement::new("image/png".to_string(), TEST_PNG_1X1.to_string()).unwrap(),
                ]),
                ..Default::default()
            },
            ChatMessage::new("user".to_string(), "what's in the image?".to_string()),
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        assert_eq!(output[0].get("role").unwrap(), "tool");
        assert_eq!(output[1].get("role").unwrap(), "user");
        assert_eq!(output[2].get("role").unwrap(), "user");
    }

    #[test]
    fn test_delayed_images_flushed_at_end() {
        let messages = vec![
            ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::Multimodal(vec![
                    MultimodalElement::new("image/png".to_string(), TEST_PNG_1X1.to_string()).unwrap(),
                ]),
                ..Default::default()
            },
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        assert_eq!(output.len(), 2);
        assert_eq!(output[1].get("role").unwrap(), "user");
    }

    #[test]
    fn test_empty_messages() {
        let messages: Vec<ChatMessage> = vec![];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        assert!(output.is_empty());
    }

    #[test]
    fn test_only_thinking_replacement_targets_last_assistant() {
        let messages = vec![
            ChatMessage::new("user".to_string(), "first".to_string()),
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("".to_string()),
                thinking_blocks: Some(vec![json!({"type": "thinking"})]),
                ..Default::default()
            },
            ChatMessage::new("user".to_string(), "second".to_string()),
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::SimpleText("real content".to_string()),
                ..Default::default()
            },
        ];
        let output = convert_messages_to_openai_format(messages, &style(), "test-model");
        let first_asst = output[1].get("content").unwrap().as_str().unwrap_or("");
        assert!(!first_asst.contains("Previous reasoning"));
    }
}
