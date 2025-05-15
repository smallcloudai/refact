use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;
use tokio::fs;
use std::io;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use serde_json::json;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::return_one_candidate_or_a_good_error;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, DiffChunk};
use crate::files_correction::{canonical_path, correct_to_nearest_dir_path, correct_to_nearest_filename, get_project_dirs, preprocess_path_for_normalization};
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::tools::tools_description::{MatchConfirmDeny, MatchConfirmDenyResult, Tool, ToolDesc, ToolParam};
use crate::integrations::integr_abstract::IntegrationConfirmation;
use crate::privacy::{FilePrivacyLevel, load_privacy_if_needed, check_file_privacy};

pub struct ToolMv;

impl ToolMv {
    fn preformat_path(path: &String) -> String {
        path.trim_end_matches(&['/', '\\'][..]).to_string()
    }

    // Parse the overwrite flag.
    fn parse_overwrite(args: &HashMap<String, Value>) -> Result<bool, String> {
        match args.get("overwrite") {
            Some(Value::Bool(b)) => Ok(*b),
            Some(Value::String(s)) => {
                let lower = s.to_lowercase();
                Ok(lower == "true")
            },
            None => Ok(false),
            Some(other) => Err(format!("Expected boolean for 'overwrite', got {:?}", other)),
        }
    }
}

#[async_trait]
impl Tool for ToolMv {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let src_str = match args.get("source") {
            Some(Value::String(s)) if !s.trim().is_empty() => Self::preformat_path(&s.trim().to_string()),
            _ => return Err("Missing required argument `source`".to_string()),
        };
        let dst_str = match args.get("destination") {
            Some(Value::String(s)) if !s.trim().is_empty() => Self::preformat_path(&s.trim().to_string()),
            _ => return Err("Missing required argument `destination`".to_string()),
        };
        let src_str = preprocess_path_for_normalization(src_str);
        let dst_str = preprocess_path_for_normalization(dst_str);
        let overwrite = Self::parse_overwrite(args)?;

        let gcx = ccx.lock().await.global_context.clone();
        let project_dirs = get_project_dirs(gcx.clone()).await;

        let src_file_candidates = correct_to_nearest_filename(gcx.clone(), &src_str, false, ccx.lock().await.top_n).await;
        let src_dir_candidates = correct_to_nearest_dir_path(gcx.clone(), &src_str, false, ccx.lock().await.top_n).await;
        let (src_corrected_path, src_is_dir) = if !src_file_candidates.is_empty() {
            (return_one_candidate_or_a_good_error(
                gcx.clone(),
                &src_str,
                &src_file_candidates,
                &project_dirs,
                false
            ).await?, false)
        } else if !src_dir_candidates.is_empty() {
            (return_one_candidate_or_a_good_error(
                gcx.clone(),
                &src_str,
                &src_dir_candidates,
                &project_dirs,
                true
            ).await?, true)
        } else {
            return Err(format!("Source path '{}' not found", src_str));
        };

        let dst_parent = if let Some(p) = std::path::Path::new(&dst_str).parent() {
            if cfg!(target_os = "windows") {
                p.to_string_lossy().replace("/", "\\")
            } else {
                p.to_string_lossy().to_string()
            }
        } else { dst_str.clone() };

        let dst_dir_candidates = correct_to_nearest_dir_path(gcx.clone(), &dst_parent, false, ccx.lock().await.top_n).await;
        let dst_parent_path = if !dst_dir_candidates.is_empty() {
            return_one_candidate_or_a_good_error(
                gcx.clone(),
                &dst_parent,
                &dst_dir_candidates,
                &project_dirs,
                true
            ).await?
        } else {
            return Err(format!("Destination parent directory '{}' not found", dst_parent));
        };

        let dst_name = std::path::Path::new(&dst_str)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(dst_str.clone());
        let dst_corrected_path = format!("{}/{}", dst_parent_path.trim_end_matches('/'), dst_name);

        let src_true_path = canonical_path(&src_corrected_path);
        let dst_true_path = canonical_path(&dst_corrected_path);

        let privacy_settings = load_privacy_if_needed(gcx.clone()).await;
        if let Err(e) = check_file_privacy(
            privacy_settings.clone(),
            &src_true_path,
            &FilePrivacyLevel::AllowToSendAnywhere
        ) {
            return Err(format!("Cannot move '{}': {}", src_str, e));
        }
        if let Err(e) = check_file_privacy(
            privacy_settings.clone(),
            &dst_true_path,
            &FilePrivacyLevel::AllowToSendAnywhere
        ) {
            return Err(format!("Cannot move to '{}': {}", src_str, e));
        }

        let src_within_project = project_dirs.iter().any(|p| src_true_path.starts_with(p));
        let dst_within_project = project_dirs.iter().any(|p| dst_true_path.starts_with(p));
        if !src_within_project && !gcx.read().await.cmdline.inside_container {
            return Err(format!("Cannot move '{}': source is not within project directories", src_str));
        }
        if !dst_within_project && !gcx.read().await.cmdline.inside_container {
            return Err(format!("Cannot move to '{}': destination is not within project directories", dst_str));
        }

        let src_metadata = fs::symlink_metadata(&src_true_path).await
            .map_err(|e| format!("Failed to access source '{}': {}", src_str, e))?;

        let mut src_file_content = String::new();
        if !src_is_dir {
            src_file_content = get_file_text_from_memory_or_disk(gcx.clone(), &src_true_path).await?;
        }
        let mut dst_file_content = String::new();
        if let Ok(dst_metadata) = fs::metadata(&dst_true_path).await {
            if !overwrite {
                return Err(format!("Destination '{}' exists. Use overwrite=true to replace it", dst_str));
            }
            if dst_metadata.is_dir() {
                fs::remove_dir_all(&dst_true_path).await
                    .map_err(|e| format!("Failed to remove existing directory '{}': {}", dst_str, e))?;
            } else {
                if !dst_metadata.is_dir() {
                    dst_file_content = fs::read_to_string(&dst_true_path).await.unwrap_or_else(|_| "".to_string());
                }
                fs::remove_file(&dst_true_path).await
                    .map_err(|e| format!("Failed to remove existing file '{}': {}", dst_str, e))?;
            }
        }

        if let Some(parent) = dst_true_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await
                    .map_err(|e| format!("Failed to create parent directory for '{}': {}", dst_str, e))?;
            }
            let parent_metadata = fs::metadata(parent).await
                .map_err(|e| format!("Failed to check parent directory permissions: {}", e))?;
            if parent_metadata.permissions().readonly() {
                return Err(format!("No write permission to parent directory of '{}'", dst_str));
            }
        }

        match fs::rename(&src_true_path, &dst_true_path).await {
            Ok(_) => {
                let corrections = src_str != src_corrected_path || dst_str != dst_corrected_path;
                let mut messages = vec![];
                if !src_is_dir && !src_file_content.is_empty() {
                    let diff_chunk = DiffChunk {
                        file_name: src_corrected_path.clone(),
                        file_action: "rename".to_string(),
                        line1: 1,
                        line2: src_file_content.lines().count(),
                        lines_remove: src_file_content.clone(),
                        lines_add: "".to_string(),
                        file_name_rename: Some(dst_corrected_path.clone()),
                        is_file: true,
                        application_details: format!("File {} from '{}' to '{}'",
                            if src_true_path.parent() == dst_true_path.parent() { "renamed" } else { "moved" },
                            src_corrected_path, dst_corrected_path),
                    };
                    if !dst_file_content.is_empty() {
                        let dst_diff_chunk = DiffChunk {
                            file_name: dst_corrected_path.clone(),
                            file_action: "edit".to_string(), // Use "edit" instead of "overwrite"
                            line1: 1,
                            line2: dst_file_content.lines().count(),
                            lines_remove: dst_file_content.clone(),
                            lines_add: src_file_content.clone(),
                            file_name_rename: None,
                            is_file: true,
                            application_details: format!("`{}` replaced with `{}`", dst_corrected_path, src_corrected_path),
                        };
                        messages.push(ContextEnum::ChatMessage(ChatMessage {
                            role: "diff".to_string(),
                            content: ChatContent::SimpleText(json!([diff_chunk, dst_diff_chunk]).to_string()),
                            tool_calls: None,
                            tool_call_id: tool_call_id.clone(),
                            ..Default::default()
                        }));
                    } else {
                        messages.push(ContextEnum::ChatMessage(ChatMessage {
                            role: "diff".to_string(),
                            content: ChatContent::SimpleText(json!([diff_chunk]).to_string()),
                            tool_calls: None,
                            tool_call_id: tool_call_id.clone(),
                            ..Default::default()
                        }));
                    }
                }
                Ok((corrections, messages))
            },
            Err(e) => {
                if e.kind() == io::ErrorKind::Other && e.to_string().contains("cross-device") {
                    if src_metadata.is_dir() {
                        Err("Cross-device move of directories is not supported in this simplified tool".to_string())
                    } else {
                        fs::copy(&src_true_path, &dst_true_path).await
                            .map_err(|e| format!("Failed to copy '{}' to '{}': {}", src_str, dst_str, e))?;
                        fs::remove_file(&src_true_path).await
                            .map_err(|e| format!("Failed to remove source file '{}' after copy: {}", src_str, e))?;

                        let mut messages = vec![];

                        if !src_file_content.is_empty() {
                            let diff_chunk = DiffChunk {
                                file_name: src_corrected_path.clone(),
                                file_action: "rename".to_string(),
                                line1: 1,
                                line2: src_file_content.lines().count(),
                                lines_remove: src_file_content.clone(),
                                lines_add: "".to_string(),
                                file_name_rename: Some(dst_corrected_path.clone()),
                                is_file: true,
                                application_details: format!("File renamed from '{}' to '{}'",
                                    src_corrected_path, dst_corrected_path),
                            };
                            if !dst_file_content.is_empty() {
                                let dst_diff_chunk = DiffChunk {
                                    file_name: dst_corrected_path.clone(),
                                    file_action: "edit".to_string(),
                                    line1: 1,
                                    line2: dst_file_content.lines().count(),
                                    lines_remove: dst_file_content.clone(),
                                    lines_add: src_file_content.clone(),
                                    file_name_rename: None,
                                    is_file: true,
                                    application_details: format!("`{}` replaced with `{}`", dst_corrected_path, src_corrected_path),
                                };
                                messages.push(ContextEnum::ChatMessage(ChatMessage {
                                    role: "diff".to_string(),
                                    content: ChatContent::SimpleText(json!([diff_chunk, dst_diff_chunk]).to_string()),
                                    tool_calls: None,
                                    tool_call_id: tool_call_id.clone(),
                                    ..Default::default()
                                }));
                            } else {
                                messages.push(ContextEnum::ChatMessage(ChatMessage {
                                    role: "diff".to_string(),
                                    content: ChatContent::SimpleText(json!([diff_chunk]).to_string()),
                                    tool_calls: None,
                                    tool_call_id: tool_call_id.clone(),
                                    ..Default::default()
                                }));
                            }
                        }
                        Ok((false, messages))
                    }
                } else {
                    Err(format!("Failed to move '{}' to '{}': {}", src_str, dst_str, e))
                }
            }
        }
    }

    fn command_to_match_against_confirm_deny(
        &self,
        args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        let src = match args.get("source") {
            Some(Value::String(s)) if !s.trim().is_empty() => s.trim().to_string(),
            _ => return Ok("".to_string()),
        };
        let dst = match args.get("destination") {
            Some(Value::String(s)) if !s.trim().is_empty() => s.trim().to_string(),
            _ => return Ok("".to_string()),
        };
        let overwrite = Self::parse_overwrite(args).unwrap_or(false);
        Ok(format!("mv {} {} {}", if overwrite { "--force" } else { "" }, src, dst))
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(IntegrationConfirmation {
            ask_user: vec!["*".to_string()],
            deny: vec![],
        })
    }

    async fn match_against_confirm_deny(
        &self,
        _: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>,
    ) -> Result<MatchConfirmDeny, String> {
        let command_to_match = self.command_to_match_against_confirm_deny(&args).map_err(|e| {
            format!("Error getting tool command to match: {}", e)
        })?;
        Ok(MatchConfirmDeny {
            result: MatchConfirmDenyResult::CONFIRMATION,
            command: command_to_match,
            rule: "default".to_string(),
        })
    }

    fn tool_name(&self) -> String {
        "mv".to_string()
    }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "mv".to_string(),
            agentic: false,
            experimental: false,
            description: "Moves or renames files and directories. If a simple rename fails due to a cross-device error and the source is a file, it falls back to copying and deleting. Use overwrite=true to replace an existing target.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "source".to_string(),
                    param_type: "string".to_string(),
                    description: "Path of the file or directory to move.".to_string(),
                },
                ToolParam {
                    name: "destination".to_string(),
                    param_type: "string".to_string(),
                    description: "Target path where the file or directory should be placed.".to_string(),
                },
                ToolParam {
                    name: "overwrite".to_string(),
                    param_type: "boolean".to_string(),
                    description: "If true and target exists, replace it. Defaults to false.".to_string(),
                }
            ],
            parameters_required: vec!["source".to_string(), "destination".to_string()],
        }
    }
}