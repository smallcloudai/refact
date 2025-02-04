use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{
    ChatContent, ChatMessage, ChatUsage, ContextEnum,
};
use crate::integrations::integr_abstract::IntegrationConfirmation;
use crate::tools::tools_description::{MatchConfirmDeny, MatchConfirmDenyResult, Tool};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tracing::warn;

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
}

fn write_file(path: &PathBuf, file_text: &String) -> Result<(), String> {
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
    fs::write(&path, file_text).map_err(|e| {
        let err = format!("Failed to write file: {:?}\nERROR: {}", path, e);
        warn!("{err}");
        err
    })
}

fn str_replace(path: &PathBuf, old_str: &String, new_str: &String) -> Result<(), String> {
    let file_content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {:?}\nERROR: {}", path, e))?;
    let occurrences = file_content.matches(old_str).count();
    if occurrences == 0 {
        return Err(format!(
            "No replacement was performed, old_str `{}` did not appear verbatim in {:?}.",
            old_str, path
        ));
    } else if occurrences > 1 {
        let lines: Vec<usize> = file_content
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains(old_str))
            .map(|(idx, _)| idx + 1)
            .collect();
        return Err(format!(
            "No replacement was performed. Multiple occurrences of old_str `{}` in lines {:?}. Please ensure it is unique",
            old_str, lines
        ));
    }

    let new_file_content = file_content.replace(old_str, new_str);
    write_file(path, &new_file_content)
}

fn process_command(command: &ToolTextEditCommand) -> Result<(), String> {
    match command.command.as_str() {
        "create" => {
            let file_text = command
                .file_text
                .clone()
                .expect("file_text is checked before");
            write_file(&command.path, &file_text)
        }
        "str_replace" => {
            let old_str = command.old_str.clone().expect("old_str is checked before");
            let new_str = command.new_str.clone().expect("new_str is checked before");
            str_replace(&command.path, &old_str, &new_str)
        }
        _ => Err("unknown command".to_string()),
    }
}

fn parse_args_to_command(args: &HashMap<String, Value>) -> Result<ToolTextEditCommand, String> {
    let command = match args.get("command") {
        Some(Value::String(s)) => {
            let command = s.trim().to_string();
            if command != "create" && command != "str_replace" {
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
            if !path.exists() {
                return Err(format!("argument 'path' doesn't exist: {:?}", path));
            }
            path
        }
        Some(v) => return Err(format!("argument 'path' should be a string: {:?}", v)),
        None => return Err("argument 'path' is required".to_string()),
    };
    let file_text_mb = match args.get("file_text") {
        Some(Value::String(s)) => {
            if command != "create" {
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
    Ok(ToolTextEditCommand {
        command,
        path,
        file_text: file_text_mb,
        new_str: new_str_mb,
        old_str: old_str_mb,
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
        process_command(&command)?;

        Ok((false, vec![
            ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText(
                    "Command execution successful. Use `cat()` to review changes and make sure they are as expected. Edit the file again if necessary.".to_string()
                ),
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                ..Default::default()
            })
        ]))
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
            result: MatchConfirmDenyResult::CONFIRMATION,
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
