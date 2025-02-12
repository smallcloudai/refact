use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ContextEnum};
use crate::integrations::integr_abstract::IntegrationConfirmation;
use crate::tools::file::auxiliary::{
    await_ast_indexing, convert_edit_to_diffchunks, sync_documents_ast, write_file,
};
use crate::tools::tools_description::{MatchConfirmDeny, MatchConfirmDenyResult, Tool};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use crate::files_correction::to_pathbuf_normalize;

struct ToolReplaceTextDocArgs {
    path: PathBuf,
    content: String,
}

pub struct ToolReplaceTextDoc;

fn parse_args(args: &HashMap<String, Value>) -> Result<ToolReplaceTextDocArgs, String> {
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
    let content = match args.get("content") {
        Some(Value::String(s)) => s,
        Some(v) => return Err(format!("argument 'content' should be a string: {:?}", v)),
        None => {
            return Err(format!(
                "argument 'content' is required for the `create` command: {:?}",
                path
            ))
        }
    };

    Ok(ToolReplaceTextDocArgs {
        path,
        content: content.clone(),
    })
}

#[async_trait]
impl Tool for ToolReplaceTextDoc {
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
        let (before_text, after_text) = write_file(&args.path, &args.content)?;
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
                    command: "replace_textdoc".to_string(),
                    rule: "".to_string(),
                });
            }
        }
        Ok(MatchConfirmDeny {
            result: MatchConfirmDenyResult::CONFIRMATION,
            command: "replace_textdoc".to_string(),
            rule: "default".to_string(),
        })
    }

    fn command_to_match_against_confirm_deny(
        &self,
        _args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        Ok("replace_textdoc".to_string())
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(IntegrationConfirmation {
            ask_user: vec!["replace_textdoc*".to_string()],
            deny: vec![],
        })
    }
}
