use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use axum::http::StatusCode;
use ropey::Rope;
use uuid::Uuid;
use crate::custom_error::ScratchError;


#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CursorPosition {
    pub file: String,
    pub line: i32,
    pub character: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CodeCompletionInputs {
    pub sources: HashMap<String, String>,
    pub cursor: CursorPosition,
    pub multiline: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SamplingParameters {
    #[serde(default)]
    pub max_new_tokens: usize,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    #[serde(default)]
    pub stop: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CodeCompletionPost {
    pub inputs: CodeCompletionInputs,
    #[serde(default)]
    pub parameters: SamplingParameters,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub scratchpad: String,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub no_cache: bool,
    #[serde(default)]
    pub use_ast: bool,
    #[allow(dead_code)]
    #[serde(default)]
    pub use_vecdb: bool,
    #[serde(default)]
    pub rag_tokens_n: usize,
}

pub(crate) fn validate_post(code_completion_post: CodeCompletionPost) -> axum::response::Result<(), ScratchError> {
    let pos = code_completion_post.inputs.cursor.clone();
    let Some(source) = code_completion_post.inputs.sources.get(&code_completion_post.inputs.cursor.file) else {
        return Err(ScratchError::new(StatusCode::BAD_REQUEST, "invalid post".to_string()))
    };
    let text = Rope::from_str(&*source);
    let line_number = pos.line as usize;
    if line_number >= text.len_lines() {
        return Err(ScratchError::new(StatusCode::BAD_REQUEST, "invalid post".to_string()))
    }
    let line = text.line(line_number);
    let col = pos.character as usize;
    if col > line.len_chars() {
        return Err(ScratchError::new(StatusCode::BAD_REQUEST, "invalid post".to_string()))
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::call_validation::{CodeCompletionInputs, CursorPosition, SamplingParameters};
    use super::*;

    #[test]
    fn test_valid_post1() {
        let post = CodeCompletionPost {
            inputs: CodeCompletionInputs {
                sources: HashMap::from_iter([("hello.py".to_string(), "def hello_world():".to_string())]),
                cursor: CursorPosition {
                    file: "hello.py".to_string(),
                    line: 0,
                    character: 18,
                },
                multiline: true,
            },
            parameters: SamplingParameters {
                max_new_tokens: 20,
                temperature: Some(0.1),
                top_p: None,
                stop: vec![],
            },
            model: "".to_string(),
            scratchpad: "".to_string(),
            stream: false,
            no_cache: false,
            use_ast: true,
            use_vecdb: true,
            rag_tokens_n: 0,
        };
        assert!(validate_post(post).is_ok());
    }

    #[test]
    fn test_valid_post2() {
        let post = CodeCompletionPost {
            inputs: CodeCompletionInputs {
                sources: HashMap::from_iter([("hello.py".to_string(), "你好世界Ωßåß🤖".to_string())]),
                cursor: CursorPosition {
                    file: "hello.py".to_string(),
                    line: 0,
                    character: 10,
                },
                multiline: true,
            },
            parameters: SamplingParameters {
                max_new_tokens: 20,
                temperature: Some(0.1),
                top_p: None,
                stop: vec![],
            },
            model: "".to_string(),
            scratchpad: "".to_string(),
            stream: false,
            no_cache: false,
            use_ast: true,
            use_vecdb: true,
            rag_tokens_n: 0,
        };
        assert!(validate_post(post).is_ok());
    }

    #[test]
    fn test_invalid_post_incorrect_line() {
        let post = CodeCompletionPost {
            inputs: CodeCompletionInputs {
                sources: HashMap::from_iter([("hello.py".to_string(), "def hello_world():".to_string())]),
                cursor: CursorPosition {
                    file: "hello.py".to_string(),
                    line: 2,
                    character: 18,
                },
                multiline: true,
            },
            parameters: SamplingParameters {
                max_new_tokens: 20,
                temperature: Some(0.1),
                top_p: None,
                stop: vec![],
            },
            model: "".to_string(),
            scratchpad: "".to_string(),
            stream: false,
            no_cache: false,
            use_ast: true,
            use_vecdb: true,
            rag_tokens_n: 0,
        };
        assert!(validate_post(post).is_err());
    }

    #[test]
    fn test_invalid_post_incorrect_col() {
        let post = CodeCompletionPost {
            inputs: CodeCompletionInputs {
                sources: HashMap::from_iter([("hello.py".to_string(), "def hello_world():".to_string())]),
                cursor: CursorPosition {
                    file: "hello.py".to_string(),
                    line: 0,
                    character: 80,
                },
                multiline: true,
            },
            parameters: SamplingParameters {
                max_new_tokens: 20,
                temperature: Some(0.1),
                top_p: None,
                stop: vec![],
            },
            model: "".to_string(),
            scratchpad: "".to_string(),
            stream: false,
            no_cache: false,
            use_ast: true,
            use_vecdb: true,
            rag_tokens_n: 0,
        };
        assert!(validate_post(post).is_err());
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContextFile {
    pub file_name: String,
    pub file_content: String,
    pub line1: usize,   // starts from 1, zero means non-valid
    pub line2: usize,   // starts from 1
    #[serde(default, skip_serializing)]
    pub symbol: Uuid,
    #[serde(default = "default_gradient_type_value", skip_serializing)]
    pub gradient_type: i32,
    #[serde(default)]
    pub usefulness: f32,  // higher is better
    #[serde(default, skip_serializing)]
    pub is_body_important: bool
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContextMemory {
    pub memo_id: String,
    pub memo_text: String,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ContextEnum {
    ContextFile(ContextFile),
    ChatMessage(ChatMessage),
}

fn default_gradient_type_value() -> i32 {
    -1
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatToolFunction {
    pub arguments: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatToolCall {
    pub id: String,
    pub function: ChatToolFunction,
    #[serde(rename = "type")]
    pub tool_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    #[serde(default, deserialize_with="deserialize_content")]
    pub content: String,
    #[serde(default)]
    pub tool_calls: Option<Vec<ChatToolCall>>,
    #[serde(default)]
    pub tool_call_id: String,
}

// this converts null to empty string
fn deserialize_content<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(|opt| opt.unwrap_or_default())
}

impl ChatMessage {
    pub fn new(role: String, content: String) -> Self {
        ChatMessage { role, content, tool_calls: None, tool_call_id: "".to_string() }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatPost {
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub parameters: SamplingParameters,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub scratchpad: String,
    pub stream: Option<bool>,
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: usize,
    #[serde(default)]
    pub tools: Option<Vec<serde_json::Value>>,
    // pub tool_choice: Option<String>,
    #[serde(default)]
    pub only_deterministic_messages: bool,  // means don't sample from the model
    #[serde(default)]
    pub chat_id: String,
}
