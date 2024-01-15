use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use axum::http::StatusCode;
use ropey::Rope;
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
    pub stop: Option<Vec<String>>,
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
                stop: None,
            },
            model: "".to_string(),
            scratchpad: "".to_string(),
            stream: false,
            no_cache: false,
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
                stop: None,
            },
            model: "".to_string(),
            scratchpad: "".to_string(),
            stream: false,
            no_cache: false,
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
                stop: None,
            },
            model: "".to_string(),
            scratchpad: "".to_string(),
            stream: false,
            no_cache: false,
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
                stop: None,
            },
            model: "".to_string(),
            scratchpad: "".to_string(),
            stream: false,
            no_cache: false,
        };
        assert!(validate_post(post).is_err());
    }
    
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContextFile {
    pub file_name: String,
    pub file_content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChatPost {
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub parameters: SamplingParameters,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub scratchpad: String,
    pub stream: Option<bool>,
}
