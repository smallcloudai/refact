use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ContextEnum};
use crate::integrations::integr_abstract::IntegrationConfirmation;
use crate::tools::file::auxiliary::{await_ast_indexing, convert_edit_to_diffchunks, str_replace, sync_documents_ast};
use crate::tools::tools_description::{MatchConfirmDeny, MatchConfirmDenyResult, Tool};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use crate::files_correction::to_pathbuf_normalize;

struct ToolUpdateTextDocArgs {
    path: PathBuf,
    old_str: String,
    replacement: String,
    multiple: bool,
}

pub struct ToolUpdateTextDoc;

fn parse_args(args: &HashMap<String, Value>) -> Result<ToolUpdateTextDocArgs, String> {
    let path = match args.get("path") {
        Some(Value::String(s)) => {
            let path = to_pathbuf_normalize(&s.trim().to_string());
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
    let old_str = match args.get("old_str") {
        Some(Value::String(s)) => s.to_string(),
        Some(v) => return Err(format!("argument 'old_str' should be a string: {:?}", v)),
        None => return Err("argument 'old_str' is required".to_string())
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

    Ok(ToolUpdateTextDocArgs {
        path,
        old_str,
        replacement,
        multiple
    })
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
        let args = parse_args(args)?;
        await_ast_indexing(gcx.clone()).await?;
        let (before_text, after_text) = str_replace(&args.path, &args.old_str, &args.replacement, args.multiple)?;
        sync_documents_ast(gcx.clone(), &args.path).await?;
        let diff_chunks = convert_edit_to_diffchunks(args.path.clone(), &before_text, &after_text)?;
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
