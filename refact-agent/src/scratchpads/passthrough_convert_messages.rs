use serde_json::Value;
use tracing::{error, warn};
use crate::call_validation::{ChatContent, ChatMessage, ContextFile};


pub fn convert_messages_to_openai_format(messages: Vec<ChatMessage>, style: &Option<String>) -> Vec<Value> {
    let mut results = vec![];
    let mut delay_images = vec![];

    let flush_delayed_images = |results: &mut Vec<Value>, delay_images: &mut Vec<Value>| {
        results.extend(delay_images.clone());
        delay_images.clear();
    };

    for msg in messages {
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
                    results.push(msg_cloned.into_value(&style));
                    if !images.is_empty() {
                        let msg_img = ChatMessage {
                            role: "user".to_string(),
                            content: ChatContent::Multimodal(images.into_iter().cloned().collect()),
                            ..Default::default()
                        };
                        delay_images.push(msg_img.into_value(&style));
                    }
                },
                ChatContent::SimpleText(_) => {
                    results.push(msg.into_value(&style));
                }
            }

        } else if msg.role == "assistant" || msg.role == "system" {
            flush_delayed_images(&mut results, &mut delay_images);
            results.push(msg.into_value(&style));

        } else if msg.role == "user" {
            flush_delayed_images(&mut results, &mut delay_images);
            results.push(msg.into_value(&style));

        } else if msg.role == "diff" {
            let tool_msg = ChatMessage {
                role: "tool".to_string(),
                content: msg.content.clone(),
                tool_calls: None,
                tool_call_id: msg.tool_call_id.clone(),
                ..Default::default()
            };
            results.push(tool_msg.into_value(&style));

        } else if msg.role == "plain_text" || msg.role == "cd_instruction" {
            flush_delayed_images(&mut results, &mut delay_images);
            results.push(ChatMessage::new(
                "user".to_string(),
                msg.content.content_text_only(),
            ).into_value(&style));

        } else if msg.role == "context_file" {
            flush_delayed_images(&mut results, &mut delay_images);
            match serde_json::from_str::<Vec<ContextFile>>(&msg.content.content_text_only()) {
                Ok(vector_of_context_files) => {
                    for context_file in vector_of_context_files {
                        results.push(ChatMessage::new(
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
    flush_delayed_images(&mut results, &mut delay_images);

    results
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_validation::{ChatContent, ChatMessage};
    use serde_json::json;
    use crate::scratchpads::multimodality::MultimodalElement;

    // cargo test -- --nocapture test_convert_messages_to_openai_format
    #[test]
    fn test_convert_messages_to_openai_format() {
        let messages = vec![
            // conv1
            ChatMessage::new("user".to_string(), "user".to_string()),
            ChatMessage::new("assistant".to_string(), "assistant".to_string()),
            ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::Multimodal(vec![
                    MultimodalElement::new("text".to_string(), "text".to_string()).unwrap(),
                    MultimodalElement::new("image/png".to_string(), "image/png".to_string()).unwrap(),
                ]),
                ..Default::default()
            },
            ChatMessage::new("plain_text".to_string(), "plain_text".to_string()),

            //conv2
            ChatMessage::new("user".to_string(), "user".to_string()),
            ChatMessage::new("assistant".to_string(), "assistant".to_string()),
            ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::Multimodal(vec![
                    MultimodalElement::new("text".to_string(), "text".to_string()).unwrap(),
                    MultimodalElement::new("image/png".to_string(), "image/png".to_string()).unwrap(),
                ]),
                ..Default::default()
            },
            ChatMessage::new("plain_text".to_string(), "plain_text".to_string()),
        ];

        // checking only roles from output, other fields are simplified
        let expected_output = vec![
            // conv1
            json!({
                "role": "user",
                "content": "user",
            }),
            json!({
                "role": "assistant",
                "content": "assistant"
            }),
            json!({
                "role": "tool",
                "content": "text"
            }),
            json!({
                "role": "user",
                "content": "plain_text"
            }),
            json!({
                "role": "user",
                "content": "IMAGE_HERE"
            }),

            // conv2
            json!({
                "role": "user",
                "content": "user"
            }),
            json!({
                "role": "assistant",
                "content": "assistant"
            }),
            json!({
                "role": "tool",
                "content": "text"
            }),
            json!({
                "role": "user",
                "content": "plain_text"
            }),
            json!({
                "role": "user",
                "content": "IMAGE_HERE"
            }),
        ];

        let roles_out_expected = expected_output.iter().map(|x| x.get("role").unwrap().as_str().unwrap().to_string()).collect::<Vec<_>>();

        let style = Some("openai".to_string());
        let output = convert_messages_to_openai_format(messages, &style);

        // println!("OUTPUT: {:#?}", output);
        let roles_out = output.iter().map(|x| x.get("role").unwrap().as_str().unwrap().to_string()).collect::<Vec<_>>();

        assert_eq!(roles_out, roles_out_expected);
    }
}
