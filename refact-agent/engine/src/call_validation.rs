use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use axum::http::StatusCode;
use ropey::Rope;

use crate::custom_error::ScratchError;
use crate::git::checkpoints::Checkpoint;
use crate::scratchpads::multimodality::MultimodalElement;

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

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ReasoningEffort {
    Low,
    #[default]
    Medium,
    High,
}

impl ReasoningEffort {
    pub fn to_string(&self) -> String { format!("{:?}", self).to_lowercase() }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SamplingParameters {
    #[serde(default)]
    pub max_new_tokens: usize, // TODO: rename it to `max_completion_tokens` everywhere, including chat-js
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,  // NOTE: deprecated
    #[serde(default)]
    pub stop: Vec<String>,
    pub n: Option<usize>,
    #[serde(default)]
    pub boost_reasoning: bool,
    // NOTE: use the following arguments for direct API calls
    #[serde(default)]
    pub reasoning_effort: Option<ReasoningEffort>,  // OpenAI style reasoning
    #[serde(default)]
    pub thinking: Option<serde_json::Value>,  // Anthropic style reasoning
    #[serde(default)]
    pub enable_thinking: Option<bool>,  // Qwen style reasoning
}

#[derive(Debug, Deserialize, Clone)]
pub struct CodeCompletionPost {
    pub inputs: CodeCompletionInputs,
    #[serde(default)]
    pub parameters: SamplingParameters,
    #[serde(default)]
    pub model: String,
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

pub fn code_completion_post_validate(
    code_completion_post: &CodeCompletionPost,
) -> axum::response::Result<(), ScratchError> {
    let pos = &code_completion_post.inputs.cursor;
    let Some(source) = code_completion_post
        .inputs
        .sources
        .get(&code_completion_post.inputs.cursor.file)
    else {
        return Err(ScratchError::new(
            StatusCode::BAD_REQUEST,
            "Invalid post: cursor in a file that is not a source".to_string(),
        ));
    };
    let text = Rope::from_str(source);
    let line_number = pos.line as usize;
    if line_number >= text.len_lines() {
        return Err(ScratchError::new(
            StatusCode::BAD_REQUEST,
            "Invalid post: line number exceeds lines in file".to_string(),
        ));
    }
    let line = text.line(line_number);
    let col = pos.character as usize;
    if col > line.len_chars() {
        return Err(ScratchError::new(
            StatusCode::BAD_REQUEST,
            "Invalid post: char number exceeds chars in line".to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContextFile {
    pub file_name: String,
    pub file_content: String,
    pub line1: usize, // starts from 1, zero means non-valid
    pub line2: usize, // starts from 1
    #[serde(default, skip_serializing)]
    pub symbols: Vec<String>,
    #[serde(default = "default_gradient_type_value", skip_serializing)]
    pub gradient_type: i32,
    #[serde(default, skip_serializing)]
    pub usefulness: f32, // higher is better
}

fn default_gradient_type_value() -> i32 { -1 }

#[derive(Debug, Clone)]
pub enum ContextEnum {
    ContextFile(ContextFile),
    ChatMessage(ChatMessage),
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum ChatContent {
    SimpleText(String),
    Multimodal(Vec<MultimodalElement>),
}

impl Default for ChatContent {
    fn default() -> Self {
        ChatContent::SimpleText(String::new())
    }
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct ChatMessage {
    pub role: String,
    pub content: ChatContent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatToolCall>>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tool_call_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_failed: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checkpoints: Vec<Checkpoint>,
    #[serde(default, skip_serializing_if="Option::is_none")]
    pub thinking_blocks: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ModelType {
    Chat,
    Completion,
    Embedding,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatModelType {
    Light,
    Default,
    Thinking
}

impl Default for ChatModelType {
    fn default() -> Self {
        ChatModelType::Default
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubchatParameters {
    pub subchat_n_ctx: usize,
    #[serde(default)]
    pub subchat_tokens_for_rag: usize,
    #[serde(default)]
    pub subchat_temperature: Option<f32>,
    #[serde(default)]
    pub subchat_max_new_tokens: usize,
    #[serde(default)]
    pub subchat_reasoning_effort: Option<ReasoningEffort>,
}


fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Hash, Debug, Eq, PartialEq, Default, Ord, PartialOrd)]
pub struct DiffChunk {
    pub file_name: String,
    pub file_action: String, // edit, rename, add, remove
    pub line1: usize,
    pub line2: usize,
    pub lines_remove: String,
    pub lines_add: String,
    #[serde(default)]
    pub file_name_rename: Option<String>,
    #[serde(default = "default_true", skip_serializing)]
    pub is_file: bool,
    pub application_details: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PostprocessSettings {
    pub useful_background: f32,          // first, fill usefulness of all lines with this
    pub useful_symbol_default: f32,      // when a symbol present, set usefulness higher
    // search results fill usefulness as it passed from outside
    pub downgrade_parent_coef: f32,      // goto parent from search results and mark it useful, with this coef
    pub downgrade_body_coef: f32,        // multiply body usefulness by this, so it's less useful than the declaration
    pub comments_propagate_up_coef: f32, // mark comments above a symbol as useful, with this coef
    pub close_small_gaps: bool,
    pub take_floor: f32,                 // take/dont value
    pub max_files_n: usize,              // don't produce more than n files in output
}

impl Default for PostprocessSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl PostprocessSettings {
    pub fn new() -> Self {
        PostprocessSettings {
            downgrade_body_coef: 0.8,
            downgrade_parent_coef: 0.6,
            useful_background: 5.0,
            useful_symbol_default: 10.0,
            close_small_gaps: true,
            comments_propagate_up_coef: 0.99,
            take_floor: 0.0,
            max_files_n: 0,
        }
    }
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
                sources: HashMap::from_iter([(
                    "hello.py".to_string(),
                    "def hello_world():".to_string(),
                )]),
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
                ..Default::default()
            },
            model: "".to_string(),
            stream: false,
            no_cache: false,
            use_ast: true,
            use_vecdb: true,
            rag_tokens_n: 0,
        };
        assert!(code_completion_post_validate(&post).is_ok());
    }

    #[test]
    fn test_valid_post2() {
        let post = CodeCompletionPost {
            inputs: CodeCompletionInputs {
                sources: HashMap::from_iter([(
                    "hello.py".to_string(),
                    "你好世界Ωßåß🤖".to_string(),
                )]),
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
                ..Default::default()
            },
            model: "".to_string(),
            stream: false,
            no_cache: false,
            use_ast: true,
            use_vecdb: true,
            rag_tokens_n: 0,
        };
        assert!(code_completion_post_validate(&post).is_ok());
    }

    #[test]
    fn test_invalid_post_incorrect_line() {
        let post = CodeCompletionPost {
            inputs: CodeCompletionInputs {
                sources: HashMap::from_iter([(
                    "hello.py".to_string(),
                    "def hello_world():".to_string(),
                )]),
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
                ..Default::default()
            },
            model: "".to_string(),
            stream: false,
            no_cache: false,
            use_ast: true,
            use_vecdb: true,
            rag_tokens_n: 0,
        };
        assert!(code_completion_post_validate(&post).is_err());
    }

    #[test]
    fn test_invalid_post_incorrect_col() {
        let post = CodeCompletionPost {
            inputs: CodeCompletionInputs {
                sources: HashMap::from_iter([(
                    "hello.py".to_string(),
                    "def hello_world():".to_string(),
                )]),
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
                ..Default::default()
            },
            model: "".to_string(),
            stream: false,
            no_cache: false,
            use_ast: true,
            use_vecdb: true,
            rag_tokens_n: 0,
        };
        assert!(code_completion_post_validate(&post).is_err());
    }
}
