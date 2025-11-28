use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ContextEnum, DiffChunk};
use crate::integrations::integr_abstract::IntegrationConfirmation;
use crate::privacy::{check_file_privacy, load_privacy_if_needed, FilePrivacyLevel, PrivacySettings};
use crate::tools::file_edit::auxiliary::{await_ast_indexing, convert_edit_to_diffchunks, str_replace_lines, sync_documents_ast};
use crate::tools::tools_description::{MatchConfirmDeny, MatchConfirmDenyResult, Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use crate::files_correction::{canonicalize_normalized_path, get_project_dirs, preprocess_path_for_normalization};
use tokio::sync::RwLock as ARwLock;
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::global_context::GlobalContext;

struct ToolUpdateTextDocByLinesArgs {
    path: PathBuf,
    content: String,
    ranges: String,
}

pub struct ToolUpdateTextDocByLines {
    pub config_path: String,
}

async fn parse_args(
    gcx: Arc<ARwLock<GlobalContext>>,
    args: &HashMap<String, Value>,
    privacy_settings: Arc<PrivacySettings>
) -> Result<ToolUpdateTextDocByLinesArgs, String> {
    let path = match args.get("path") {
        Some(Value::String(s)) => {
            let raw_path = preprocess_path_for_normalization(s.trim().to_string());
            let candidates_file = file_repair_candidates(gcx.clone(), &raw_path, 3, false).await;
            let path = match return_one_candidate_or_a_good_error(gcx.clone(), &raw_path, &candidates_file, &get_project_dirs(gcx.clone()).await, false).await {
                Ok(f) => canonicalize_normalized_path(PathBuf::from(f)),
                Err(e) => return Err(e),
            };
            if check_file_privacy(privacy_settings, &path, &FilePrivacyLevel::AllowToSendAnywhere).is_err() {
                return Err(format!(
                    "Error: Cannot update the file '{:?}' due to privacy settings.",
                    s.trim()
                ));
            }
            if !path.exists() {
                return Err(format!(
                    "Error: The file '{:?}' does not exist. Please check if the path is correct and the file exists.",
                    path
                ));
            }
            path
        }
        Some(v) => return Err(format!("Error: The 'path' argument must be a string, but received: {:?}", v)),
        None => return Err("Error: The 'path' argument is required but was not provided.".to_string()),
    };

    let content = match args.get("content") {
        Some(Value::String(s)) => s.to_string(),
        Some(v) => return Err(format!("Error: The 'content' argument must be a string containing the new text, but received: {:?}", v)),
        None => return Err("Error: The 'content' argument is required. Please provide the new text that will replace the specified lines.".to_string())
    };

    let ranges = match args.get("ranges") {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(v) => return Err(format!("Error: The 'ranges' argument must be a string, but received: {:?}", v)),
        None => return Err("Error: The 'ranges' argument is required. Format: ':3' (lines 1-3), '40:50' (lines 40-50), '100:' (line 100 to end), or combine with commas like ':3,40:50,100:'.".to_string())
    };

    if ranges.is_empty() {
        return Err("Error: The 'ranges' argument cannot be empty.".to_string());
    }

    Ok(ToolUpdateTextDocByLinesArgs {
        path,
        content,
        ranges,
    })
}

pub async fn tool_update_text_doc_by_lines_exec(
    gcx: Arc<ARwLock<GlobalContext>>,
    args: &HashMap<String, Value>,
    dry: bool
) -> Result<(String, String, Vec<DiffChunk>), String> {
    let privacy_settings = load_privacy_if_needed(gcx.clone()).await;
    let args = parse_args(gcx.clone(), args, privacy_settings).await?;
    await_ast_indexing(gcx.clone()).await?;
    let (before_text, after_text) = str_replace_lines(
        gcx.clone(),
        &args.path,
        &args.content,
        &args.ranges,
        dry
    ).await?;
    sync_documents_ast(gcx.clone(), &args.path).await?;
    let diff_chunks = convert_edit_to_diffchunks(args.path.clone(), &before_text, &after_text)?;
    Ok((before_text, after_text, diff_chunks))
}

#[async_trait]
impl Tool for ToolUpdateTextDocByLines {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let gcx = ccx.lock().await.global_context.clone();
        let (_, _, diff_chunks) = tool_update_text_doc_by_lines_exec(gcx.clone(), args, false).await?;
        let results = vec![ChatMessage {
            role: "diff".to_string(),
            content: ChatContent::SimpleText(json!(diff_chunks).to_string()),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            usage: None,
            ..Default::default()
        }]
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
        let gcx = ccx.lock().await.global_context.clone();
        let privacy_settings = load_privacy_if_needed(gcx.clone()).await;

        async fn can_execute_tool_edit(gcx: Arc<ARwLock<GlobalContext>>, args: &HashMap<String, Value>, privacy_settings: Arc<PrivacySettings>) -> Result<(), String> {
            let _ = parse_args(gcx.clone(), args, privacy_settings).await?;
            Ok(())
        }

        let msgs_len = ccx.lock().await.messages.len();

        if msgs_len != 0 {
            if let Err(_) = can_execute_tool_edit(gcx.clone(), args, privacy_settings).await {
                return Ok(MatchConfirmDeny {
                    result: MatchConfirmDenyResult::PASS,
                    command: "update_textdoc_by_lines".to_string(),
                    rule: "".to_string(),
                });
            }
        }
        Ok(MatchConfirmDeny {
            result: MatchConfirmDenyResult::CONFIRMATION,
            command: "update_textdoc_by_lines".to_string(),
            rule: "default".to_string(),
        })
    }

    async fn command_to_match_against_confirm_deny(
        &self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        _args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        Ok("update_textdoc_by_lines".to_string())
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(IntegrationConfirmation {
            ask_user: vec!["update_textdoc_by_lines*".to_string()],
            deny: vec![],
        })
    }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "update_textdoc_by_lines".to_string(),
            display_name: "Update Text Document By Lines".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: false,
            experimental: false,
            description: "Replaces line ranges in an existing file with new content. Line numbers are 1-based and inclusive. Supports multiple non-overlapping ranges.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "path".to_string(),
                    description: "Absolute path to the file to modify.".to_string(),
                    param_type: "string".to_string(),
                },
                ToolParam {
                    name: "content".to_string(),
                    description: "The new text content. For multiple ranges, separate content for each range with '---RANGE_SEPARATOR---'.".to_string(),
                    param_type: "string".to_string(),
                },
                ToolParam {
                    name: "ranges".to_string(),
                    description: "Line ranges to replace. Format: ':3' (lines 1-3), '40:50' (lines 40-50), '100:' (line 100 to end), '5' (just line 5). Combine multiple ranges with commas: ':3,40:50,100:'. Ranges must not overlap.".to_string(),
                    param_type: "string".to_string(),
                },
            ],
            parameters_required: vec![
                "path".to_string(),
                "content".to_string(),
                "ranges".to_string(),
            ],
        }
    }
}
