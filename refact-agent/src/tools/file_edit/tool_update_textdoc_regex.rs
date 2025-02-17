use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ContextEnum, DiffChunk};
use crate::integrations::integr_abstract::IntegrationConfirmation;
use crate::tools::file_edit::auxiliary::{await_ast_indexing, convert_edit_to_diffchunks, str_replace_regex, sync_documents_ast};
use crate::tools::tools_description::{MatchConfirmDeny, MatchConfirmDenyResult, Tool};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use regex::Regex;
use tokio::sync::Mutex as AMutex;
use crate::files_correction::canonical_path;
use tokio::sync::RwLock as ARwLock;
use crate::global_context::GlobalContext;

struct ToolUpdateTextDocRegexArgs {
    path: PathBuf,
    pattern: Regex,
    replacement: String,
    multiple: bool,
}

pub struct ToolUpdateTextDocRegex;

fn parse_args(args: &HashMap<String, Value>) -> Result<ToolUpdateTextDocRegexArgs, String> {
    let path = match args.get("path") {
        Some(Value::String(s)) => {
            let path = canonical_path(&s.trim().to_string());
            if !path.is_absolute() {
                return Err(format!(
                    "argument 'path' should be an absolute path: {:?}",
                    path
                ));
            }
            if !path.exists() {
                return Err(format!("argument 'path' doesn't exists: {:?}", path));
            }
            path
        }
        Some(v) => return Err(format!("argument 'path' should be a string: {:?}", v)),
        None => return Err("argument 'path' is required".to_string()),
    };
    let pattern = match args.get("pattern") {
        Some(Value::String(s)) => {
            match Regex::new(s) {
                Ok(r) => r,
                Err(err) => {
                    return Err(format!("argument 'pattern' should be a correct regex: {:?}", err));
                }
            }
        },
        Some(v) => return Err(format!("argument 'pattern' should be a string: {:?}", v)),
        None => return Err("argument 'pattern' is required:".to_string())
    };
    let replacement = match args.get("replacement") {
        Some(Value::String(s)) => s.to_string(),
        Some(v) => return Err(format!("argument 'replacement' should be a string: {:?}", v)),
        None => return Err("argument 'replacement' is required".to_string())
    };
    let multiple = match args.get("multiple") {
        Some(Value::Bool(b)) => b.clone(),
        Some(v) => return Err(format!("argument 'multiple' should be a boolean: {:?}", v)),
        None => return Err("argument 'multiple' is required".to_string())
    };

    Ok(ToolUpdateTextDocRegexArgs {
        path,
        pattern,
        replacement,
        multiple
    })
}

pub async fn tool_update_text_doc_regex_exec(
    gcx: Arc<ARwLock<GlobalContext>>,
    args: &HashMap<String, Value>,
    dry: bool
) -> Result<(String, String, Vec<DiffChunk>), String> {
    let args = parse_args(args)?;
    await_ast_indexing(gcx.clone()).await?;
    let (before_text, after_text) = str_replace_regex(&args.path, &args.pattern, &args.replacement, args.multiple, dry)?;
    sync_documents_ast(gcx.clone(), &args.path).await?;
    let diff_chunks = convert_edit_to_diffchunks(args.path.clone(), &before_text, &after_text)?;
    Ok((before_text, after_text, diff_chunks))
}

#[async_trait]
impl Tool for ToolUpdateTextDocRegex {
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
        let (_, _, diff_chunks) = tool_update_text_doc_regex_exec(gcx.clone(), args, false).await?;
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
        async fn can_execute_tool_edit(args: &HashMap<String, Value>) -> Result<(), String> {
            let _ = parse_args(args)?;
            Ok(())
        }

        let msgs_len = ccx.lock().await.messages.len();

        // workaround: if messages weren't passed by ToolsPermissionCheckPost, legacy
        if msgs_len != 0 {
            // if we cannot execute apply_edit, there's no need for confirmation
            if let Err(_) = can_execute_tool_edit(args).await {
                return Ok(MatchConfirmDeny {
                    result: MatchConfirmDenyResult::PASS,
                    command: "update_textdoc_regex".to_string(),
                    rule: "".to_string(),
                });
            }
        }
        Ok(MatchConfirmDeny {
            result: MatchConfirmDenyResult::CONFIRMATION,
            command: "update_textdoc_regex".to_string(),
            rule: "default".to_string(),
        })
    }

    fn command_to_match_against_confirm_deny(
        &self,
        _args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        Ok("update_textdoc_regex".to_string())
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(IntegrationConfirmation {
            ask_user: vec!["update_textdoc_regex*".to_string()],
            deny: vec![],
        })
    }
}
