use tracing::warn;

use crate::call_validation::ChatContent;
use crate::scratchpads::multimodality::MultimodalElement;
use crate::scratchpads::scratchpad_utils::parse_image_b64_from_image_url_openai;

const MAX_IMAGES_PER_MESSAGE: usize = 5;

pub fn validate_content_with_attachments(content: &serde_json::Value, attachments: &[serde_json::Value]) -> Result<ChatContent, String> {
    let mut elements: Vec<MultimodalElement> = Vec::new();
    let mut image_count = 0;

    if let Some(s) = content.as_str() {
        if !s.is_empty() {
            elements.push(MultimodalElement::new("text".to_string(), s.to_string())
                .map_err(|e| format!("Invalid text content: {}", e))?);
        }
    } else if let Some(arr) = content.as_array() {
        if arr.is_empty() {
            return Err("Content array is empty".to_string());
        }
        for (idx, item) in arr.iter().enumerate() {
            let item_type = item.get("type").and_then(|t| t.as_str())
                .ok_or_else(|| format!("Content element {} missing 'type' field", idx))?;
            match item_type {
                "text" => {
                    let text = item.get("text").and_then(|t| t.as_str())
                        .ok_or_else(|| format!("Content element {} missing 'text' field", idx))?;
                    elements.push(MultimodalElement::new("text".to_string(), text.to_string())
                        .map_err(|e| format!("Invalid text content at {}: {}", idx, e))?);
                }
                "image_url" => {
                    image_count += 1;
                    if image_count > MAX_IMAGES_PER_MESSAGE {
                        return Err(format!("Too many images: max {} allowed", MAX_IMAGES_PER_MESSAGE));
                    }
                    let url = item.get("image_url")
                        .and_then(|u| u.get("url"))
                        .and_then(|u| u.as_str())
                        .ok_or_else(|| format!("Content element {} missing image_url.url", idx))?;
                    let (image_type, _, image_content) = parse_image_b64_from_image_url_openai(url)
                        .ok_or_else(|| format!("Invalid image URL format at element {}", idx))?;
                    elements.push(MultimodalElement::new(image_type, image_content)
                        .map_err(|e| format!("Invalid image at {}: {}", idx, e))?);
                }
                other => {
                    return Err(format!("Unknown content type '{}' at element {}", other, idx));
                }
            }
        }
    } else if !content.is_null() {
        return Err(format!("Content must be string or array, got {}", content));
    }

    for (idx, attachment) in attachments.iter().enumerate() {
        let url = attachment.get("image_url")
            .and_then(|u| u.get("url"))
            .and_then(|u| u.as_str())
            .ok_or_else(|| format!("Attachment {} missing image_url.url", idx))?;
        image_count += 1;
        if image_count > MAX_IMAGES_PER_MESSAGE {
            return Err(format!("Too many images: max {} allowed", MAX_IMAGES_PER_MESSAGE));
        }
        let (image_type, _, image_content) = parse_image_b64_from_image_url_openai(url)
            .ok_or_else(|| format!("Invalid attachment image URL at {}", idx))?;
        elements.push(MultimodalElement::new(image_type, image_content)
            .map_err(|e| format!("Invalid attachment image at {}: {}", idx, e))?);
    }

    if elements.is_empty() {
        Ok(ChatContent::SimpleText(String::new()))
    } else if elements.len() == 1 && elements[0].m_type == "text" {
        Ok(ChatContent::SimpleText(elements.remove(0).m_content))
    } else {
        Ok(ChatContent::Multimodal(elements))
    }
}

pub fn parse_content_with_attachments(content: &serde_json::Value, attachments: &[serde_json::Value]) -> ChatContent {
    let base_content = parse_content_from_value(content);

    if attachments.is_empty() {
        return base_content;
    }

    let mut elements: Vec<MultimodalElement> = match base_content {
        ChatContent::SimpleText(s) if !s.is_empty() => {
            vec![MultimodalElement::new("text".to_string(), s).unwrap()]
        }
        ChatContent::Multimodal(v) => v,
        _ => Vec::new(),
    };

    for attachment in attachments {
        if let Some(url) = attachment.get("image_url").and_then(|u| u.get("url")).and_then(|u| u.as_str()) {
            if let Some((image_type, _, image_content)) = parse_image_b64_from_image_url_openai(url) {
                if let Ok(el) = MultimodalElement::new(image_type, image_content) {
                    elements.push(el);
                }
            }
        }
    }

    if elements.is_empty() {
        ChatContent::SimpleText(String::new())
    } else if elements.len() == 1 && elements[0].m_type == "text" {
        ChatContent::SimpleText(elements.remove(0).m_content)
    } else {
        ChatContent::Multimodal(elements)
    }
}

fn parse_content_from_value(content: &serde_json::Value) -> ChatContent {
    if let Some(s) = content.as_str() {
        return ChatContent::SimpleText(s.to_string());
    }

    if let Some(arr) = content.as_array() {
        let mut elements = Vec::new();
        for item in arr {
            let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match item_type {
                "text" => {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        if let Ok(el) = MultimodalElement::new("text".to_string(), text.to_string()) {
                            elements.push(el);
                        }
                    }
                }
                "image_url" => {
                    if let Some(url) = item.get("image_url").and_then(|u| u.get("url")).and_then(|u| u.as_str()) {
                        if let Some((image_type, _, image_content)) = parse_image_b64_from_image_url_openai(url) {
                            if let Ok(el) = MultimodalElement::new(image_type, image_content) {
                                elements.push(el);
                            }
                        }
                    }
                }
                _ => {
                    warn!("Unknown content type '{}' in message, preserving as text", item_type);
                    if let Ok(el) = MultimodalElement::new("text".to_string(), item.to_string()) {
                        elements.push(el);
                    }
                }
            }
        }
        if !elements.is_empty() {
            return ChatContent::Multimodal(elements);
        }
    }

    ChatContent::SimpleText(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_content_empty_array_error() {
        let content = json!([]);
        let result = validate_content_with_attachments(&content, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_validate_content_missing_type_error() {
        let content = json!([{"text": "hello"}]);
        let result = validate_content_with_attachments(&content, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("type"));
    }

    #[test]
    fn test_validate_content_text_missing_text_field_error() {
        let content = json!([{"type": "text"}]);
        let result = validate_content_with_attachments(&content, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("text"));
    }

    #[test]
    fn test_validate_content_image_missing_url_error() {
        let content = json!([{"type": "image_url"}]);
        let result = validate_content_with_attachments(&content, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("image_url.url"));
    }

    #[test]
    fn test_validate_content_unknown_type_error() {
        let content = json!([{"type": "video", "data": "xyz"}]);
        let result = validate_content_with_attachments(&content, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown content type"));
    }

    #[test]
    fn test_validate_content_non_string_non_array_error() {
        let content = json!({"key": "value"});
        let result = validate_content_with_attachments(&content, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be string or array"));
    }

    #[test]
    fn test_validate_content_number_error() {
        let content = json!(123);
        let result = validate_content_with_attachments(&content, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_content_simple_string_ok() {
        let content = json!("Hello world");
        let result = validate_content_with_attachments(&content, &[]);
        assert!(result.is_ok());
        match result.unwrap() {
            ChatContent::SimpleText(s) => assert_eq!(s, "Hello world"),
            _ => panic!("Expected SimpleText"),
        }
    }

    #[test]
    fn test_validate_content_text_array_ok() {
        let content = json!([{"type": "text", "text": "Hello"}]);
        let result = validate_content_with_attachments(&content, &[]);
        assert!(result.is_ok());
        match result.unwrap() {
            ChatContent::SimpleText(s) => assert_eq!(s, "Hello"),
            _ => panic!("Expected SimpleText for single text element"),
        }
    }

    #[test]
    fn test_validate_content_null_returns_empty() {
        let content = json!(null);
        let result = validate_content_with_attachments(&content, &[]);
        assert!(result.is_ok());
        match result.unwrap() {
            ChatContent::SimpleText(s) => assert!(s.is_empty()),
            _ => panic!("Expected empty SimpleText"),
        }
    }

    #[test]
    fn test_validate_content_empty_string_returns_empty() {
        let content = json!("");
        let result = validate_content_with_attachments(&content, &[]);
        assert!(result.is_ok());
        match result.unwrap() {
            ChatContent::SimpleText(s) => assert!(s.is_empty()),
            _ => panic!("Expected empty SimpleText"),
        }
    }

    #[test]
    fn test_parse_content_string() {
        let content = json!("Simple text");
        let result = parse_content_with_attachments(&content, &[]);
        match result {
            ChatContent::SimpleText(s) => assert_eq!(s, "Simple text"),
            _ => panic!("Expected SimpleText"),
        }
    }

    #[test]
    fn test_parse_content_null_returns_empty() {
        let content = json!(null);
        let result = parse_content_with_attachments(&content, &[]);
        match result {
            ChatContent::SimpleText(s) => assert!(s.is_empty()),
            _ => panic!("Expected empty SimpleText"),
        }
    }

    #[test]
    fn test_parse_content_unknown_type_preserved_as_text() {
        let content = json!([{"type": "custom", "data": "xyz"}]);
        let result = parse_content_with_attachments(&content, &[]);
        match result {
            ChatContent::Multimodal(elements) => {
                assert_eq!(elements.len(), 1);
                assert_eq!(elements[0].m_type, "text");
                assert!(elements[0].m_content.contains("custom"));
            }
            _ => panic!("Expected Multimodal with preserved unknown type"),
        }
    }

    #[test]
    fn test_parse_content_empty_array_returns_empty() {
        let content = json!([]);
        let result = parse_content_with_attachments(&content, &[]);
        match result {
            ChatContent::SimpleText(s) => assert!(s.is_empty()),
            _ => panic!("Expected empty SimpleText"),
        }
    }

    #[test]
    fn test_parse_content_text_array_single_element() {
        let content = json!([{"type": "text", "text": "Hello"}]);
        let result = parse_content_with_attachments(&content, &[]);
        match result {
            ChatContent::Multimodal(elements) => {
                assert_eq!(elements.len(), 1);
                assert_eq!(elements[0].m_content, "Hello");
            }
            _ => panic!("Expected Multimodal"),
        }
    }

    #[test]
    fn test_parse_content_multiple_text_elements() {
        let content = json!([
            {"type": "text", "text": "Hello"},
            {"type": "text", "text": "World"}
        ]);
        let result = parse_content_with_attachments(&content, &[]);
        match result {
            ChatContent::Multimodal(elements) => {
                assert_eq!(elements.len(), 2);
            }
            _ => panic!("Expected Multimodal"),
        }
    }
}
