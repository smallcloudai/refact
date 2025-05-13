use serde::{Deserialize, Deserializer, Serialize};
use std::sync::Arc;
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use crate::call_validation::{ChatContent, ChatMessage, ChatToolCall};
use crate::scratchpads::scratchpad_utils::{calculate_image_tokens_openai, image_reader_from_b64string, parse_image_b64_from_image_url_openai};
use crate::tokens::count_text_tokens;

pub const MULTIMODALITY_IMAGE_EXTENSIONS: [&'static str; 5] = ["png", "jpeg", "jpg", "gif", "webp"];

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
            let _ = image_reader_from_b64string(&m_content)
                .map_err(|e| format!("MultimodalElement::new() failed to parse m_content: {}", e));
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
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            ChatContentRaw::SimpleText(text) => text.is_empty(),
            ChatContentRaw::Multimodal(elements) => elements.is_empty(),
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
        }
    }

    pub fn size_estimate(&self, tokenizer: Option<Arc<Tokenizer>>, style: &Option<String>) -> usize {
        match self {
            ChatContent::SimpleText(text) => text.len(),
            ChatContent::Multimodal(_elements) => {
                let tcnt = self.count_tokens(tokenizer, style).unwrap_or(0);
                (tcnt as f32 * 2.618) as usize
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
                    let extensions = MULTIMODALITY_IMAGE_EXTENSIONS.join("|");
                    return Err(format!("Invalid image URL in MultimodalElementImageOpenAI: must pass regexp `data:image/({extensions});base64,([A-Za-z0-9+/=]+)`"));
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
        _ => Err("deserialize_chat_content() can't parse content".to_string()),
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

        Value::Object(dict)
    }
}

impl<'de> Deserialize<'de> for ChatMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let value: Value = Deserialize::deserialize(deserializer)?;
        let role = value.get("role")
            .and_then(|s| s.as_str())
            .ok_or_else(|| serde::de::Error::missing_field("role"))?
            .to_string();

        let content = match value.get("content") {
            Some(content_value) => {
                let content_raw: ChatContentRaw = chat_content_raw_from_value(content_value.clone())
                    .map_err(|e| serde::de::Error::custom(e))?;
                content_raw.to_internal_format()
                    .map_err(|e| serde::de::Error::custom(e))?
            },
            None => ChatContent::SimpleText(String::new()),
        };
        let finish_reason = value.get("finish_reason").and_then(|x| x.as_str().map(|x| x.to_string()));

        let tool_calls: Option<Vec<ChatToolCall>> = value.get("tool_calls")
            .and_then(|v| v.as_array())
            .map(|v| v.iter().map(|v| serde_json::from_value(v.clone()).map_err(serde::de::Error::custom)).collect::<Result<Vec<_>, _>>())
            .transpose()?;
        let tool_call_id: Option<String> = value.get("tool_call_id")
            .and_then(|s| s.as_str()).map(|s| s.to_string());

        let thinking_blocks: Option<Vec<Value>> = value.get("thinking_blocks")
            .and_then(|v| v.as_array())
            .map(|v| v.iter().map(|v| serde_json::from_value(v.clone()).map_err(serde::de::Error::custom)).collect::<Result<Vec<_>, _>>())
            .transpose()?;

        Ok(ChatMessage {
            role,
            content,
            finish_reason,
            tool_calls,
            tool_call_id: tool_call_id.unwrap_or_default(),
            thinking_blocks,
            ..Default::default()
        })
    }
}

/// If API supports sending fields with empty strings
fn model_supports_empty_strings(model_id: &str) -> bool {
    !model_id.starts_with("google_gemini/")
}