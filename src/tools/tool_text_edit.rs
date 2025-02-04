use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ChatUsage, ContextEnum, DiffChunk};
use crate::integrations::integr_abstract::IntegrationConfirmation;
use crate::tools::tools_description::{MatchConfirmDeny, MatchConfirmDenyResult, Tool};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tracing::warn;
use crate::tools::tool_apply_edit_aux::diff_structs::chunks_from_diffs;

fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n")
}

fn restore_line_endings(content: &str, original_had_crlf: bool) -> String {
    if original_had_crlf {
        content.replace("\n", "\r\n")
    } else {
        content.to_string()
    }
}

pub struct ToolTextEdit {
    pub usage: Option<ChatUsage>,
}

impl ToolTextEdit {
    pub fn new() -> Self {
        ToolTextEdit { usage: None }
    }
}

struct ToolTextEditCommand {
    command: String,
    path: PathBuf,
    file_text: Option<String>,
    new_str: Option<String>,
    old_str: Option<String>,
    replace_multiple: bool,
}

fn write_file(path: &PathBuf, file_text: &String) -> Result<(String, String), String> {
    if !path.exists() {
        let parent = path.parent().ok_or(format!(
            "Failed to Add: {:?}. Path is invalid.\nReason: path must have had a parent directory",
            path
        ))?;
        if !parent.exists() {
            fs::create_dir_all(&parent).map_err(|e| {
                let err = format!("Failed to Add: {:?}; Its parent dir {:?} did not exist and attempt to create it failed.\nERROR: {}", path, parent, e);
                warn!("{err}");
                err
            })?;
        }
    }
    let before_text = if path.exists() {
        fs::read_to_string(&path).map_err(|x| x.to_string())?
    } else {
        "".to_string()
    };
    fs::write(&path, file_text).map_err(|e| {
        let err = format!("Failed to write file: {:?}\nERROR: {}", path, e);
        warn!("{err}");
        err
    })?;
    Ok((before_text, file_text.to_string()))
}

fn str_replace(path: &PathBuf, old_str: &String, new_str: &String, replace_multiple: bool) -> Result<(String, String), String> {
    let file_content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {:?}\nERROR: {}", path, e))?;
    
    let has_crlf = file_content.contains("\r\n");
    
    let normalized_content = normalize_line_endings(&file_content);
    let normalized_old_str = normalize_line_endings(old_str);
    
    let occurrences = normalized_content.matches(&normalized_old_str).count();
    if occurrences == 0 {
        return Err(format!(
            "No replacement was performed, old_str `{}` did not appear verbatim in {:?}.",
            old_str, path
        ));
    }
    if !replace_multiple && occurrences > 1 {
        let lines: Vec<usize> = normalized_content
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains(&normalized_old_str))
            .map(|(idx, _)| idx + 1)
            .collect();
        return Err(format!(
            "No replacement was performed. Multiple occurrences of old_str `{}` in lines {:?}. Please ensure it is unique or set `replace_multiple` to true.",
            old_str, lines
        ));
    }

    let normalized_new_str = normalize_line_endings(new_str);
    let new_content = normalized_content.replace(&normalized_old_str, &normalized_new_str);
    
    let new_file_content = restore_line_endings(&new_content, has_crlf);
    write_file(path, &new_file_content)?;
    Ok((file_content, new_file_content))
}

fn process_command(command: &ToolTextEditCommand) -> Result<(String, String), String> {
    match command.command.as_str() {
        "create" | "file_replace" => {
            let file_text = command
                .file_text
                .clone()
                .expect("file_text is checked before");
            write_file(&command.path, &file_text)
        }
        "str_replace" => {
            let old_str = command.old_str.clone().expect("old_str is checked before");
            let new_str = command.new_str.clone().expect("new_str is checked before");
            str_replace(&command.path, &old_str, &new_str, command.replace_multiple)
        }
        _ => Err("unknown command".to_string()),
    }
}

fn convert_edit_to_diffchunks(path: PathBuf, before: &String, after: &String) -> Result<Vec<DiffChunk>, String> {
    let diffs = diff::lines(&before, &after);
    chunks_from_diffs(path.clone(), diffs)
}

fn parse_args_to_command(args: &HashMap<String, Value>) -> Result<ToolTextEditCommand, String> {
    let command = match args.get("command") {
        Some(Value::String(s)) => {
            let command = s.trim().to_string();
            if command != "create" && command != "str_replace" && command != "file_replace" {
                return Err(format!(
                    "argument 'command' should be either 'create' or 'str_replace': {:?}",
                    command
                ));
            }
            command
        }
        Some(v) => return Err(format!("argument 'command' should be a string: {:?}", v)),
        None => return Err("argument 'command' is required".to_string()),
    };
    let path = match args.get("path") {
        Some(Value::String(s)) => {
            let path = PathBuf::from(s.trim().to_string());
            if !path.is_absolute() {
                return Err(format!(
                    "argument 'path' should be an absolute path: {:?}",
                    path
                ));
            }
            if command != "create" && !path.exists() {
                return Err(format!("argument 'path' doesn't exist: {:?}", path));
            }
            path
        }
        Some(v) => return Err(format!("argument 'path' should be a string: {:?}", v)),
        None => return Err("argument 'path' is required".to_string()),
    };
    let file_text_mb = match args.get("file_text") {
        Some(Value::String(s)) => {
            if command != "create" && command != "file_replace" {
                return Err(format!(
                    "argument 'file_text' should be used only with a `create` command: {:?}",
                    path
                ));
            }
            Some(s.to_string())
        }
        Some(v) => return Err(format!("argument 'file_text' should be a string: {:?}", v)),
        None => {
            if command == "create" {
                return Err(format!(
                    "argument 'file_text' is required for the `create` command: {:?}",
                    path
                ));
            }
            None
        }
    };
    let new_str_mb = match args.get("new_str") {
        Some(Value::String(s)) => {
            if command != "str_replace" {
                return Err(format!(
                    "argument 'new_str' should be used only with a `str_replace` command: {:?}",
                    path
                ));
            }
            Some(s.to_string())
        }
        Some(v) => return Err(format!("argument 'new_str' should be a string: {:?}", v)),
        None => {
            if command == "str_replace" {
                return Err(format!(
                    "argument 'new_str' is required for the `str_replace` command: {:?}",
                    path
                ));
            }
            None
        }
    };
    let old_str_mb = match args.get("old_str") {
        Some(Value::String(s)) => {
            if command != "str_replace" {
                return Err(format!(
                    "argument 'old_str' should be used only with a `str_replace` command: {:?}",
                    path
                ));
            }
            Some(s.to_string())
        }
        Some(v) => return Err(format!("argument 'old_str' should be a string: {:?}", v)),
        None => {
            if command == "str_replace" {
                return Err(format!(
                    "argument 'old_str' is required for the `str_replace` command: {:?}",
                    path
                ));
            }
            None
        }
    };
    let replace_multiple = match args.get("replace_multiple") {
        Some(Value::Bool(b)) => b.clone(),
        Some(v) => return Err(format!("argument 'replace_multiple' should be a boolean: {:?}", v)),
        None => false,
    };
    
    Ok(ToolTextEditCommand {
        command,
        path,
        file_text: file_text_mb,
        new_str: new_str_mb,
        old_str: old_str_mb,
        replace_multiple
    })
}

async fn can_execute_tool_edit(args: &HashMap<String, Value>) -> Result<(), String> {
    let _ = parse_args_to_command(args)?;
    Ok(())
}

#[async_trait]
impl Tool for ToolTextEdit {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn tool_execute(
        &mut self,
        _: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let command = parse_args_to_command(args)?;
        let (before_text, after_text) = process_command(&command)?;
        let diff_chunks = convert_edit_to_diffchunks(command.path.clone(), &before_text, &after_text)?;
        let results = vec![
            ChatMessage {
                role: "diff".to_string(),
                content: ChatContent::SimpleText(json!(diff_chunks).to_string()),
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                usage: None,
                ..Default::default()
            }
        ]
            .into_iter()
            .map(|x| ContextEnum::ChatMessage(x))
            .collect::<Vec<_>>();
        Ok((false, results))
    }

    async fn match_against_confirm_deny(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>,
    ) -> Result<MatchConfirmDeny, String> {
        let msgs_len = ccx.lock().await.messages.len();

        // workaround: if messages weren't passed by ToolsPermissionCheckPost, legacy
        if msgs_len != 0 {
            // if we cannot execute apply_edit, there's no need for confirmation
            if let Err(_) = can_execute_tool_edit(args).await {
                return Ok(MatchConfirmDeny {
                    result: MatchConfirmDenyResult::PASS,
                    command: "text_edit".to_string(),
                    rule: "".to_string(),
                });
            }
        }
        Ok(MatchConfirmDeny {
            result: MatchConfirmDenyResult::PASS,
            command: "text_edit".to_string(),
            rule: "default".to_string(),
        })
    }

    fn command_to_match_against_confirm_deny(
        &self,
        _args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        Ok("text_edit".to_string())
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(IntegrationConfirmation {
            ask_user: vec!["text_edit*".to_string()],
            deny: vec![],
        })
    }

    fn usage(&mut self) -> &mut Option<ChatUsage> {
        &mut self.usage
    }
}
