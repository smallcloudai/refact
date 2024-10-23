use serde::{Deserialize, Deserializer, Serialize};
use std::sync::{Arc, RwLock, RwLockReadGuard};
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use crate::call_validation::{ChatContent, ChatMessage, ChatToolCall, ChatUsage};
use crate::scratchpads::scratchpad_utils::{calculate_image_tokens_openai, count_tokens as count_tokens_simple_text, parse_image_b64_from_image_url_openai};


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct MultimodalElement {
    pub m_type: String, // text or image/*
    pub m_content: String,
}

impl MultimodalElement {
    pub fn is_text(&self) -> bool {
        self.m_type == "text"
    }
    
    pub fn is_image(&self) -> bool {
        self.m_type.starts_with("image/")
    }
    
    pub fn from_openai_image(openai_image: MultimodalElementImageOpenAI) -> Result<Self, String> {
        let (image_type, _, image_content) = parse_image_b64_from_image_url_openai(&openai_image.image_url.url)
            .ok_or(format!("Failed to parse image URL: {}", openai_image.image_url.url))?;
        Ok(MultimodalElement {
            m_type: image_type.to_string(),
            m_content: image_content,
        })
    }
    
    pub fn from_openai_text(openai_text: MultimodalElementTextOpenAI) -> Self {
        MultimodalElement {
            m_type: "text".to_string(),
            m_content: openai_text.text,
        }
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

    pub fn count_tokens(&self, tokenizer: Option<&RwLockReadGuard<Tokenizer>>, style: &Option<String>) -> Result<i32, String> {
        if self.is_text() {
            if let Some(tokenizer) = tokenizer {
                Ok(count_tokens_simple_text(&tokenizer, &self.m_content) as i32)
            } else {
                return Err("count_tokens() received no tokenizer".to_string());
            }
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
                            Ok(MultimodalElement::from_openai_text(text_el.clone()))
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

    pub fn size_estimate(&self, tokenizer: Arc<RwLock<Tokenizer>>, style: &Option<String>) -> usize {
        match self {
            ChatContent::SimpleText(text) => text.len(),
            ChatContent::Multimodal(_elements) => { 
                let tcnt = self.count_tokens(tokenizer, style).unwrap_or(0);
                (tcnt as f32 * 2.618) as usize
            }
        }
    }

    pub fn count_tokens(&self, tokenizer: Arc<RwLock<Tokenizer>>, style: &Option<String>) -> Result<i32, String> {
        let tokenizer_lock = tokenizer.read().unwrap();
        match self {
            ChatContent::SimpleText(text) => Ok(count_tokens_simple_text(&tokenizer_lock, text) as i32),
            ChatContent::Multimodal(elements) => elements.iter()
                .map(|e|e.count_tokens(Some(&tokenizer_lock), style))
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
    
    pub fn into_value(&self, style: &Option<String>) -> Value {
        let mut dict = serde_json::Map::new();
        let chat_content_raw = self.content.into_raw(style);
        
        dict.insert("role".to_string(), Value::String(self.role.clone()));
        dict.insert("content".to_string(), json!(chat_content_raw));
        dict.insert("tool_calls".to_string(), json!(self.tool_calls.clone()));
        dict.insert("tool_call_id".to_string(), Value::String(self.tool_call_id.clone()));
        
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
        
        let content = {
            let content_value = value.get("content")
                .ok_or_else(|| serde::de::Error::missing_field("content")).cloned()?;
            let content_raw: ChatContentRaw = chat_content_raw_from_value(content_value)
                .map_err(|e| serde::de::Error::custom(e))?;
            content_raw.to_internal_format()
                .map_err(|e| serde::de::Error::custom(e))?
        };

        let tool_calls: Option<Vec<ChatToolCall>> = value.get("tool_calls")
            .and_then(|v| v.as_array())
            .map(|v| v.iter().map(|v| serde_json::from_value(v.clone()).map_err(serde::de::Error::custom)).collect::<Result<Vec<_>, _>>())
            .transpose()?;
        let tool_call_id: Option<String> = value.get("tool_call_id")
            .and_then(|s| s.as_str()).map(|s| s.to_string());
        let usage: Option<ChatUsage> = value.get("usage")
            .map(|v| serde_json::from_value(v.clone()).map_err(serde::de::Error::custom))
            .transpose()?;

        Ok(ChatMessage {
            role,
            content,
            tool_calls,
            tool_call_id: tool_call_id.unwrap_or_default(),
            usage,
        })
    }
}
