use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;
use tokio::fs;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use serde_json::json;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::return_one_candidate_or_a_good_error;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, DiffChunk};
use crate::files_correction::{canonical_path, correct_to_nearest_dir_path, correct_to_nearest_filename, get_project_dirs, preprocess_path_for_normalization};
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::privacy::{check_file_privacy, load_privacy_if_needed, FilePrivacyLevel};
use crate::tools::tools_description::{MatchConfirmDeny, MatchConfirmDenyResult, Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::integrations::integr_abstract::IntegrationConfirmation;

pub struct ToolRm {
    pub config_path: String,
}

impl ToolRm {
    fn preformat_path(path: &String) -> String {
        path.trim_end_matches(&['/', '\\'][..]).to_string()
    }

    fn parse_recursive(args: &HashMap<String, Value>) -> Result<(bool, Option<u32>, bool), String> {
        let recursive = match args.get("recursive") {
            Some(Value::Bool(b)) => *b,
            Some(Value::String(s)) => {
                let s = s.trim().to_lowercase();
                s == "true"
            },
            None => false,
            Some(other) => return Err(format!("Expected boolean for 'recursive', got {:?}", other)),
        };
        let max_depth = match args.get("max_depth") {
            Some(Value::Number(n)) => n.as_u64().map(|v| v as u32),
            _ => None,
        };
        let dry_run = match args.get("dry_run") {
            Some(Value::Bool(b)) => *b,
            Some(Value::String(s)) => s.trim().eq_ignore_ascii_case("true"),
            _ => false,
        };
        Ok((recursive, max_depth, dry_run))
    }
}

#[async_trait]
impl Tool for ToolRm {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn command_to_match_against_confirm_deny(
        &self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        let path = match args.get("path") {
            Some(Value::String(s)) if !s.trim().is_empty() => s.trim().to_string(),
            _ => return Ok("".to_string()),
        };
        let (recursive, _, dry_run) = Self::parse_recursive(args).unwrap_or((false, None, false));
        Ok(format!("rm {} {} {}",
            if recursive { "-r" } else { "" },
            if dry_run { "--dry-run" } else { "" },
            path))
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(IntegrationConfirmation {
            ask_user: vec!["*".to_string()],
            deny: vec![],
        })
    }

    async fn match_against_confirm_deny(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>,
    ) -> Result<MatchConfirmDeny, String> {
        let command_to_match = self.command_to_match_against_confirm_deny(ccx.clone(), &args).await.map_err(|e| {
            format!("Error getting tool command to match: {}", e)
        })?;
        Ok(MatchConfirmDeny {
            result: MatchConfirmDenyResult::CONFIRMATION,
            command: command_to_match,
            rule: "default".to_string(),
        })
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        // Get "path" argument.
        let path_str = match args.get("path") {
            Some(Value::String(s)) if !s.trim().is_empty() => Self::preformat_path(&s.trim().to_string()),
            _ => return Err("Missing required argument `path`".to_string()),
        };
        let path_str = preprocess_path_for_normalization(path_str);

        // Reject if wildcards are present, '?' is allowed if preceeded by '\' or '/' only, like \\?\C:\Some\Path
        if path_str.contains('*') || path_str.contains('[') ||
            path_str.chars().enumerate().any(|(i, c)| {
                c == '?' && !path_str[..i].chars().all(|ch| ch == '/' || ch == '\\')
            }) {
            return Err("Wildcards and shell patterns are not supported".to_string());
        }

        let (recursive, _max_depth, dry_run) = Self::parse_recursive(args)?;
        let gcx = ccx.lock().await.global_context.clone();
        let project_dirs = get_project_dirs(gcx.clone()).await;

        // Use file correction to get a candidate path.
        let file_candidates = correct_to_nearest_filename(gcx.clone(), &path_str, false, ccx.lock().await.top_n).await;
        let dir_candidates = correct_to_nearest_dir_path(gcx.clone(), &path_str, false, ccx.lock().await.top_n).await;
        let corrected_path = if !file_candidates.is_empty() {
            return_one_candidate_or_a_good_error(
                gcx.clone(),
                &path_str,
                &file_candidates,
                &project_dirs,
                false
            ).await?
        } else if !dir_candidates.is_empty() {
            return_one_candidate_or_a_good_error(
                gcx.clone(),
                &path_str,
                &dir_candidates,
                &project_dirs,
                true
            ).await?
        } else {
            return Err(format!("Path '{}' not found", path_str));
        };

        let true_path = canonical_path(&corrected_path);

        let privacy_settings = load_privacy_if_needed(gcx.clone()).await;
        if let Err(e) = check_file_privacy(
            privacy_settings.clone(),
            &true_path,
            &FilePrivacyLevel::AllowToSendAnywhere
        ) {
            return Err(format!("Cannot rm '{}': {}", path_str, e));
        }

        // Check that the true_path is within project directories.
        let is_within_project = project_dirs.iter().any(|p| true_path.starts_with(p));
        if !is_within_project && !gcx.read().await.cmdline.inside_container {
            return Err(format!("Cannot execute rm(): '{}' is not within the project directories.", path_str));
        }

        // Check if path exists.
        if !true_path.exists() {
            return Err(format!("Path '{}' does not exist", corrected_path));
        }

        // Check if we have write permission to the parent directory.
        if let Some(parent) = true_path.parent() {
            let parent_metadata = fs::metadata(parent).await.map_err(|e| {
                format!("Failed to check parent directory permissions: {}", e)
            })?;
            if parent_metadata.permissions().readonly() {
                return Err(format!("No write permission to parent directory of '{}'", corrected_path));
            }
        }

        let mut file_content = String::new();
        let mut file_size = None;
        let is_dir = true_path.is_dir();
        if !is_dir {
            file_content = match get_file_text_from_memory_or_disk(gcx.clone(), &true_path).await {
                Ok(content) => content,
                Err(e) => {
                    tracing::warn!("Failed to get file content: {}", e);
                    String::new()
                },
            };
            if let Ok(meta) = fs::metadata(&true_path).await {
                file_size = Some(meta.len());
            }
        }
        let mut messages: Vec<ContextEnum> = Vec::new();
        let corrections = path_str != corrected_path;
        if is_dir {
            if !recursive {
                return Err(format!("Cannot remove directory '{}' without recursive=true", corrected_path));
            }
            if dry_run {
                messages.push(ContextEnum::ChatMessage(ChatMessage {
                    role: "tool".to_string(),
                    content: ChatContent::SimpleText(format!("[Dry run] Would remove directory '{}'", corrected_path)),
                    tool_calls: None,
                    tool_call_id: tool_call_id.clone(),
                    ..Default::default()
                }));
                return Ok((corrections, messages));
            }
            fs::remove_dir_all(&true_path).await.map_err(|e| {
                format!("Failed to remove directory '{}': {}", corrected_path, e)
            })?;
            messages.push(ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText(format!("Removed directory '{}'", corrected_path)),
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                ..Default::default()
            }));
        } else {
            if dry_run {
                messages.push(ContextEnum::ChatMessage(ChatMessage {
                    role: "tool".to_string(),
                    content: ChatContent::SimpleText(format!("[Dry run] Would remove file '{}'", corrected_path)),
                    tool_calls: None,
                    tool_call_id: tool_call_id.clone(),
                    ..Default::default()
                }));
                return Ok((corrections, messages));
            }
            fs::remove_file(&true_path).await.map_err(|e| {
                format!("Failed to remove file '{}': {}", corrected_path, e)
            })?;
            if !file_content.is_empty() {
                let diff_chunk = DiffChunk {
                    file_name: corrected_path.clone(),
                    file_action: "remove".to_string(),
                    line1: 1,
                    line2: file_content.lines().count(),
                    lines_remove: file_content.clone(),
                    lines_add: "".to_string(),
                    file_name_rename: None,
                    is_file: true,
                    application_details: format!("File `{}` removed", corrected_path),
                };
                messages.push(ContextEnum::ChatMessage(ChatMessage {
                    role: "diff".to_string(),
                    content: ChatContent::SimpleText(json!([diff_chunk]).to_string()),
                    tool_calls: None,
                    tool_call_id: tool_call_id.clone(),
                    ..Default::default()
                }));
            } else {
                let mut message = format!("Removed file '{}'", corrected_path);
                if let Some(file_size) = file_size {
                    message = format!("{} ({})", message, crate::nicer_logs::human_readable_bytes(file_size));
                }
                messages.push(ContextEnum::ChatMessage(ChatMessage {
                    role: "tool".to_string(),
                    content: ChatContent::SimpleText(message),
                    tool_calls: None,
                    tool_call_id: tool_call_id.clone(),
                    ..Default::default()
                }));
            }
        }

        Ok((corrections, messages))
    }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "rm".to_string(),
            display_name: "rm".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: false,
            experimental: false,
            description: "Deletes a file or directory. Use recursive=true for directories. Set dry_run=true to preview without deletion.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "Absolute or relative path of the file or directory to delete.".to_string(),
                },
                ToolParam {
                    name: "recursive".to_string(),
                    param_type: "boolean".to_string(),
                    description: "If true and target is a directory, delete recursively. Defaults to false.".to_string(),
                },
                ToolParam {
                    name: "dry_run".to_string(),
                    param_type: "boolean".to_string(),
                    description: "If true, only report what would be done without deleting.".to_string(),
                },
                ToolParam {
                    name: "max_depth".to_string(),
                    param_type: "number".to_string(),
                    description: "(Optional) Maximum depth (currently unused).".to_string(),
                }
            ],
            parameters_required: vec!["path".to_string()],
        }
    }
}
