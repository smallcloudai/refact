use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ContextEnum, DiffChunk};
use crate::integrations::integr_abstract::IntegrationConfirmation;
use crate::privacy::{check_file_privacy, load_privacy_if_needed, FilePrivacyLevel, PrivacySettings};
use crate::tools::file_edit::auxiliary::{await_ast_indexing, convert_edit_to_diffchunks, str_replace, sync_documents_ast};
use crate::tools::tools_description::{MatchConfirmDeny, MatchConfirmDenyResult, Tool};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use crate::files_correction::{canonicalize_normalized_path, preprocess_path_for_normalization};
use tokio::sync::RwLock as ARwLock;
use crate::global_context::GlobalContext;

struct ToolUpdateTextDocArgs {
    path: PathBuf,
    old_str: String,
    replacement: String,
    multiple: bool,
}

pub struct ToolUpdateTextDoc;

fn parse_args(args: &HashMap<String, Value>, privacy_settings: Arc<PrivacySettings>) -> Result<ToolUpdateTextDocArgs, String> {
    let path = match args.get("path") {
        Some(Value::String(s)) => {
            let path = PathBuf::from(preprocess_path_for_normalization(s.trim().to_string()));
            if !path.is_absolute() {
                return Err(format!(
                    "Error: The provided path '{}' is not absolute. Please provide a full path starting from the root directory.",
                    s.trim()
                ));
            }
            let path = canonicalize_normalized_path(path);
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
    let old_str = match args.get("old_str") {
        Some(Value::String(s)) => s.to_string(),
        Some(v) => return Err(format!("Error: The 'old_str' argument must be a string containing the text to replace, but received: {:?}", v)),
        None => return Err("Error: The 'old_str' argument is required. Please provide the text that needs to be replaced.".to_string())
    };
    let replacement = match args.get("replacement") {
        Some(Value::String(s)) => s.to_string(),
        Some(v) => return Err(format!("Error: The 'replacement' argument must be a string containing the new text, but received: {:?}", v)),
        None => return Err("Error: The 'replacement' argument is required. Please provide the new text that will replace the old text.".to_string())
    };
    let multiple = match args.get("multiple") {
        Some(Value::Bool(b)) => b.clone(),
        Some(v) => return Err(format!("Error: The 'multiple' argument must be a boolean (true/false) indicating whether to replace all occurrences, but received: {:?}", v)),
        None => return Err("Error: The 'multiple' argument is required. Please specify true to replace all occurrences or false to replace only the first occurrence.".to_string())
    };

    Ok(ToolUpdateTextDocArgs {
        path,
        old_str,
        replacement,
        multiple
    })
}

pub async fn tool_update_text_doc_exec(
    gcx: Arc<ARwLock<GlobalContext>>,
    args: &HashMap<String, Value>,
    dry: bool
) -> Result<(String, String, Vec<DiffChunk>), String> {
    let privacy_settings = load_privacy_if_needed(gcx.clone()).await;
    let args = parse_args(args, privacy_settings)?;
    await_ast_indexing(gcx.clone()).await?;
    let (before_text, after_text) = str_replace(gcx.clone(), &args.path, &args.old_str, &args.replacement, args.multiple, dry).await?;
    sync_documents_ast(gcx.clone(), &args.path).await?;
    let diff_chunks = convert_edit_to_diffchunks(args.path.clone(), &before_text, &after_text)?;
    Ok((before_text, after_text, diff_chunks))
}

#[async_trait]
impl Tool for ToolUpdateTextDoc {
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
        let (_, _, diff_chunks) = tool_update_text_doc_exec(gcx.clone(), args, false).await?;
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
        
        async fn can_execute_tool_edit(args: &HashMap<String, Value>, privacy_settings: Arc<PrivacySettings>) -> Result<(), String> {
            let _ = parse_args(args, privacy_settings)?;
            Ok(())
        }

        let msgs_len = ccx.lock().await.messages.len();

        // workaround: if messages weren't passed by ToolsPermissionCheckPost, legacy
        if msgs_len != 0 {
            // if we cannot execute apply_edit, there's no need for confirmation
            if let Err(_) = can_execute_tool_edit(args, privacy_settings).await {
                return Ok(MatchConfirmDeny {
                    result: MatchConfirmDenyResult::PASS,
                    command: "update_textdoc".to_string(),
                    rule: "".to_string(),
                });
            }
        }
        Ok(MatchConfirmDeny {
            result: MatchConfirmDenyResult::CONFIRMATION,
            command: "update_textdoc".to_string(),
            rule: "default".to_string(),
        })
    }

    fn command_to_match_against_confirm_deny(
        &self,
        _args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        Ok("update_textdoc".to_string())
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(IntegrationConfirmation {
            ask_user: vec!["update_textdoc*".to_string()],
            deny: vec![],
        })
    }
}
