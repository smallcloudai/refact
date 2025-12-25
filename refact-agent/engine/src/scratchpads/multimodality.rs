use serde::{Deserialize, Deserializer, Serialize};
use std::sync::Arc;
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use crate::call_validation::{ChatContent, ChatMessage, ChatToolCall};
use crate::scratchpads::scratchpad_utils::{calculate_image_tokens_openai, image_reader_from_b64string, parse_image_b64_from_image_url_openai};
use crate::tokens::count_text_tokens;


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct MultimodalElement {
    pub m_type: String, // "text", "image/png" etc
    pub m_content: String,
}

impl MultimodalElement {
    pub fn new(m_type: String, m_content: String) -> Result<Self, String> {
        if !(m_type == "text") && !m_type.starts_with("image/") {
            return Err(format!("MultimodalElement::new() received invalid type: {}", m_type));
        }
        if m_type.starts_with("image/") {
            image_reader_from_b64string(&m_content)
                .map_err(|e| format!("MultimodalElement::new() failed to parse image: {}", e))?;
        }
        Ok(MultimodalElement { m_type, m_content })
    }

    pub fn is_text(&self) -> bool {
        self.m_type == "text"
    }

    pub fn is_image(&self) -> bool {
        self.m_type.starts_with("image/")
    }

    pub fn from_openai_image(openai_image: MultimodalElementImageOpenAI) -> Result<Self, String> {
        let (image_type, _, image_content) = parse_image_b64_from_image_url_openai(&openai_image.image_url.url)
            .ok_or(format!("Failed to parse image URL: {}", openai_image.image_url.url))?;
        MultimodalElement::new(image_type, image_content)
    }

    pub fn from_openai_text(openai_text: MultimodalElementTextOpenAI) -> Result<Self, String> {
        MultimodalElement::new("text".to_string(), openai_text.text)
    }

    pub fn to_orig(&self, style: &Option<String>) -> ChatMultimodalElement {
        let style = style.clone().unwrap_or("openai".to_string());
        match style.as_str() {
            "openai" => {
                if self.is_text() {
                    self.to_openai_text()
                } else if self.is_image() {
                    self.to_openai_image()
                } else {
                    unreachable!()
                }
            },
            _ => unreachable!()
        }
    }

    fn to_openai_image(&self) -> ChatMultimodalElement {
        let image_url = format!("data:{};base64,{}", self.m_type, self.m_content);
        ChatMultimodalElement::MultimodalElementImageOpenAI(MultimodalElementImageOpenAI {
            content_type: "image_url".to_string(),
            image_url: MultimodalElementImageOpenAIImageURL {
                url: image_url.clone(),
                detail: "high".to_string(),
            }
        })
    }

    fn to_openai_text(&self) -> ChatMultimodalElement {
        ChatMultimodalElement::MultimodalElementTextOpenAI(MultimodalElementTextOpenAI {
            content_type: "text".to_string(),
            text: self.m_content.clone(),
        })
    }

    pub fn count_tokens(&self, tokenizer: Option<Arc<Tokenizer>>, style: &Option<String>) -> Result<i32, String> {
        if self.is_text() {
            Ok(count_text_tokens(tokenizer, &self.m_content)? as i32)
        } else if self.is_image() {
            let style = style.clone().unwrap_or("openai".to_string());
            match style.as_str() {
                "openai" => {
                    calculate_image_tokens_openai(&self.m_content, "high")
                },
                _ => unreachable!(),
            }
        } else {
            unreachable!()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct MultimodalElementTextOpenAI {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct MultimodalElementImageOpenAI {
    #[serde(rename = "type")]
    pub content_type: String,
    pub image_url: MultimodalElementImageOpenAIImageURL,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct MultimodalElementImageOpenAIImageURL {
    pub url: String,
    #[serde(default = "default_detail")]
    pub detail: String,
}

fn default_detail() -> String {
    "high".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)] // tries to deserialize each enum variant in order
pub enum ChatMultimodalElement {
    MultimodalElementTextOpenAI(MultimodalElementTextOpenAI),
    MultimodalElementImageOpenAI(MultimodalElementImageOpenAI),
    MultimodalElement(MultimodalElement),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatContentRaw {
    SimpleText(String),
    Multimodal(Vec<ChatMultimodalElement>),
    ContextFiles(Vec<crate::call_validation::ContextFile>),
}

impl ChatContentRaw {
    pub fn to_internal_format(&self) -> Result<ChatContent, String> {
        match self {
            ChatContentRaw::SimpleText(text) => Ok(ChatContent::SimpleText(text.clone())),
            ChatContentRaw::Multimodal(elements) => {
                let internal_elements: Result<Vec<MultimodalElement>, String> = elements.iter()
                    .map(|el| match el {
                        ChatMultimodalElement::MultimodalElementTextOpenAI(text_el) => {
                            MultimodalElement::from_openai_text(text_el.clone())
                        },
                        ChatMultimodalElement::MultimodalElementImageOpenAI(image_el) => {
                            MultimodalElement::from_openai_image(image_el.clone())
                        },
                        ChatMultimodalElement::MultimodalElement(el) => Ok(el.clone()),
                    })
                    .collect();
                internal_elements.map(ChatContent::Multimodal)
            },
            ChatContentRaw::ContextFiles(files) => Ok(ChatContent::ContextFiles(files.clone())),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            ChatContentRaw::SimpleText(text) => text.is_empty(),
            ChatContentRaw::Multimodal(elements) => elements.is_empty(),
            ChatContentRaw::ContextFiles(files) => files.is_empty(),
        }
    }
}

impl ChatContent {
    pub fn content_text_only(&self) -> String {
        match self {
            ChatContent::SimpleText(text) => text.clone(),
            ChatContent::Multimodal(elements) => elements.iter()
                .filter(|el|el.m_type == "text")
                .map(|el|el.m_content.clone())
                .collect::<Vec<_>>()
                .join("\n\n"),
            ChatContent::ContextFiles(files) => files.iter()
                .map(|f| format!("{}:{}-{}\n{}", f.file_name, f.line1, f.line2, f.file_content))
                .collect::<Vec<_>>()
                .join("\n\n"),
        }
    }

    pub fn size_estimate(&self, tokenizer: Option<Arc<Tokenizer>>, style: &Option<String>) -> usize {
        match self {
            ChatContent::SimpleText(text) => text.len(),
            ChatContent::Multimodal(_elements) => {
                let tcnt = self.count_tokens(tokenizer, style).unwrap_or(0);
                (tcnt as f32 * 2.618) as usize
            },
            ChatContent::ContextFiles(files) => {
                files.iter().map(|f| f.file_content.len() + f.file_name.len()).sum()
            },
        }
    }

    pub fn count_tokens(&self, tokenizer: Option<Arc<Tokenizer>>, style: &Option<String>) -> Result<i32, String> {
        match self {
            ChatContent::SimpleText(text) => Ok(count_text_tokens(tokenizer, text)? as i32),
            ChatContent::Multimodal(elements) => elements.iter()
                .map(|e|e.count_tokens(tokenizer.clone(), style))
                .collect::<Result<Vec<_>, _>>()
                .map(|counts| counts.iter().sum()),
            ChatContent::ContextFiles(files) => {
                let total: i32 = files.iter()
                    .map(|f| count_text_tokens(tokenizer.clone(), &f.file_content).unwrap_or(0) as i32)
                    .sum();
                Ok(total)
            },
        }
    }

    pub fn into_raw(&self, style: &Option<String>) -> ChatContentRaw {
        match self {
            ChatContent::SimpleText(text) => ChatContentRaw::SimpleText(text.clone()),
            ChatContent::Multimodal(elements) => {
                let orig_elements = elements.iter()
                    .map(|el| el.to_orig(style))
                    .collect::<Vec<_>>();
                ChatContentRaw::Multimodal(orig_elements)
            },
            ChatContent::ContextFiles(files) => {
                // Serialize context files as JSON array
                ChatContentRaw::ContextFiles(files.clone())
            }
        }
    }
}

pub fn chat_content_raw_from_value(value: Value) -> Result<ChatContentRaw, String> {
    fn validate_multimodal_element(element: &ChatMultimodalElement) -> Result<(), String> {
        match element {
            ChatMultimodalElement::MultimodalElementTextOpenAI(el) => {
                if el.content_type != "text" {
                    return Err("Invalid multimodal element: type must be `text`".to_string());
                }
            },
            ChatMultimodalElement::MultimodalElementImageOpenAI(el) => {
                if el.content_type != "image_url" {
                    return Err("Invalid multimodal element: type must be `image_url`".to_string());
                }
                if parse_image_b64_from_image_url_openai(&el.image_url.url).is_none() {
                    return Err("Invalid image URL in MultimodalElementImageOpenAI: must pass regexp `data:image/(png|jpeg|jpg|webp|gif);base64,([A-Za-z0-9+/=]+)`".to_string());
                }
            }
            ChatMultimodalElement::MultimodalElement(_el) => {}
        };
        Ok(())
    }

    match value {
        Value::Null => Ok(ChatContentRaw::SimpleText(String::new())),
        Value::String(s) => Ok(ChatContentRaw::SimpleText(s)),
        Value::Array(array) => {
            // First, try to parse as context files (check if first element has file_name)
            if let Some(first) = array.first() {
                if first.get("file_name").is_some() {
                    // Looks like context files
                    let files: Result<Vec<crate::call_validation::ContextFile>, _> =
                        array.iter().map(|item| serde_json::from_value(item.clone())).collect();
                    if let Ok(context_files) = files {
                        return Ok(ChatContentRaw::ContextFiles(context_files));
                    }
                }
            }

            // Otherwise, try to parse as multimodal elements
            let mut elements = vec![];
            for (idx, item) in array.into_iter().enumerate() {
                let element: ChatMultimodalElement = serde_json::from_value(item)
                    .map_err(|e| format!("Error deserializing element at index {}: {}", idx, e))?;
                validate_multimodal_element(&element)
                    .map_err(|e| format!("Validation error for element at index {}: {}", idx, e))?;
                elements.push(element);
            }

            Ok(ChatContentRaw::Multimodal(elements))
        },
        Value::Object(obj) => {
            // Old tool message format: { "tool_call_id": "...", "content": "...", "tool_failed": bool }
            // Try to extract and recursively parse the inner content field
            if let Some(content_val) = obj.get("content") {
                // Recursively parse the inner content
                match chat_content_raw_from_value(content_val.clone()) {
                    Ok(inner) => return Ok(inner),
                    Err(_) => {
                        // If recursive parsing fails, try to get as string
                        if let Some(s) = content_val.as_str() {
                            return Ok(ChatContentRaw::SimpleText(s.to_string()));
                        }
                    }
                }
            }
            // If it's an object but not the old tool format, convert to JSON string
            Ok(ChatContentRaw::SimpleText(serde_json::to_string(&Value::Object(obj)).unwrap_or_default()))
        },
        other => {
            let type_name = match &other {
                Value::Bool(_) => "bool",
                Value::Number(_) => "number",
                _ => "unknown"
            };
            let value_str = serde_json::to_string(&other).unwrap_or_else(|_| "failed to serialize".to_string());
            tracing::error!("deserialize_chat_content() can't parse content type: {}, value: {}", type_name, value_str);
            Err(format!("deserialize_chat_content() can't parse content"))
        }
    }
}

impl ChatMessage {
    pub fn new(role: String, content: String) -> Self {
        ChatMessage {
            role,
            content: ChatContent::SimpleText(content),
            ..Default::default()
        }
    }

    pub fn into_value(&self, style: &Option<String>, model_id: &str) -> Value {
        let mut dict = serde_json::Map::new();
        let chat_content_raw = self.content.into_raw(style);
        dict.insert("role".to_string(), Value::String(self.role.clone()));
        if model_supports_empty_strings(model_id) || !chat_content_raw.is_empty() {
            dict.insert("content".to_string(), json!(chat_content_raw));
        }
        if !model_supports_empty_strings(model_id) && chat_content_raw.is_empty()
            && self.tool_calls.is_none() && self.thinking_blocks.is_none() {
            dict.insert("content".to_string(), "_".into());
        }
        if let Some(tool_calls) = self.tool_calls.clone() {
            dict.insert("tool_calls".to_string(), json!(tool_calls));
        }
        if !self.tool_call_id.is_empty() {
            dict.insert("tool_call_id".to_string(), Value::String(self.tool_call_id.clone()));
        }
        if let Some(thinking_blocks) = self.thinking_blocks.clone() {
            dict.insert("thinking_blocks".to_string(), json!(thinking_blocks));
        }
        if let Some(reasoning_content) = self.reasoning_content.clone() {
            dict.insert("reasoning_content".to_string(), json!(reasoning_content));
        }

        Value::Object(dict)
    }
}

impl<'de> Deserialize<'de> for ChatMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let value: Value = Deserialize::deserialize(deserializer)?;
        let obj = value.as_object().ok_or_else(|| serde::de::Error::custom("expected object"))?;

        let role = obj.get("role")
            .and_then(|s| s.as_str())
            .ok_or_else(|| serde::de::Error::missing_field("role"))?
            .to_string();

        let content = match obj.get("content") {
            Some(content_value) => {
                let content_raw: ChatContentRaw = chat_content_raw_from_value(content_value.clone())
                    .map_err(|e| serde::de::Error::custom(e))?;
                content_raw.to_internal_format()
                    .map_err(|e| serde::de::Error::custom(e))?
            },
            None => ChatContent::SimpleText(String::new()),
        };

        let message_id = obj.get("message_id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
        let finish_reason = obj.get("finish_reason").and_then(|x| x.as_str().map(|x| x.to_string()));
        let reasoning_content = obj.get("reasoning_content").and_then(|x| x.as_str().map(|x| x.to_string()));
        let tool_call_id = obj.get("tool_call_id").and_then(|s| s.as_str()).unwrap_or_default().to_string();
        let tool_failed = obj.get("tool_failed").and_then(|x| x.as_bool());

        let tool_calls: Option<Vec<ChatToolCall>> = obj.get("tool_calls")
            .and_then(|v| v.as_array())
            .map(|v| v.iter().map(|v| serde_json::from_value(v.clone()).map_err(serde::de::Error::custom)).collect::<Result<Vec<_>, _>>())
            .transpose()?;

        let thinking_blocks: Option<Vec<Value>> = obj.get("thinking_blocks")
            .and_then(|v| v.as_array())
            .map(|v| v.iter().cloned().collect());

        let citations: Vec<Value> = obj.get("citations")
            .and_then(|v| v.as_array())
            .map(|v| v.iter().cloned().collect())
            .unwrap_or_default();

        let usage: Option<crate::call_validation::ChatUsage> = obj.get("usage")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let checkpoints: Vec<crate::git::checkpoints::Checkpoint> = obj.get("checkpoints")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        const KNOWN_FIELDS: &[&str] = &[
            "role", "content", "message_id", "finish_reason", "reasoning_content",
            "tool_calls", "tool_call_id", "tool_failed", "usage", "checkpoints",
            "thinking_blocks", "citations"
        ];
        let extra: serde_json::Map<String, Value> = obj.iter()
            .filter(|(k, _)| !KNOWN_FIELDS.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(ChatMessage {
            message_id,
            role,
            content,
            finish_reason,
            reasoning_content,
            tool_calls,
            tool_call_id,
            tool_failed,
            usage,
            checkpoints,
            thinking_blocks,
            citations,
            extra,
            output_filter: None,
        })
    }
}

/// If API supports sending fields with empty strings
fn model_supports_empty_strings(model_id: &str) -> bool {
    !model_id.starts_with("google_gemini/")
}