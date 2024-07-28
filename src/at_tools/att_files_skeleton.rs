use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;
use tokio::sync::RwLock as ARwLock;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::tools::Tool;
use crate::cached_tokenizers;
use crate::call_validation::{ChatMessage, ContextEnum, ContextFile};
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::global_context::GlobalContext;
use crate::scratchpads::chat_utils_rag::postprocess_at_results2;


pub const MAX_TOKENS: usize = 24000;


pub async fn context_msg_from_file_name(
    global_context: Arc<ARwLock<GlobalContext>>,
    file_name: String,
) -> ContextFile {
    let path = crate::files_correction::canonical_path(&file_name.clone());
    let file_content_mb = get_file_text_from_memory_or_disk(global_context.clone(), &path).await;
    let file_content = file_content_mb.unwrap_or_else(|e| e);
    ContextFile {
        file_name: file_name.clone(),
        file_content: file_content.clone(),
        line1: 0,
        line2: file_content.lines().count(),
        symbol: Uuid::default(),
        gradient_type: 0,
        usefulness: 1.0,
        is_body_important: false
    }
}


pub fn format_context_files_to_message_content(
    context_files: Vec<ContextFile>,
) -> String {
    let mut content: String = String::new();
    for x in context_files.iter() {
        content.push_str(format!("{}:\n\n{}\n\n", x.file_name.as_str(), x.file_content.as_str()).as_str());
    }
    content
}


pub struct AttFilesSkeleton;


#[async_trait]
impl Tool for AttFilesSkeleton {
    async fn tool_execute(&mut self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        // global context copy, tokenizer etc.
        let gx = ccx.global_context.clone();

        let caps = crate::global_context::try_load_caps_quickly_if_not_present(
            gx.clone(), 0)
            .await
            .map_err(|e| {
                format!("No caps: {:?}", e);
                "Network error communicating with the model (1)".to_string()
            })?;

        let tokenizer = cached_tokenizers::cached_tokenizer(
            caps.clone(), gx.clone(), "gpt-4o".to_string(),
        ).await?;

        // parse args
        let file_names = match args.get("paths") {
            Some(Value::String(s)) => Some(s.split(",").map(|x| x.to_string()).collect::<Vec<String>>()),
            Some(v) => { return Err(format!("argument `paths` is not a string: {:?}", v)) }
            None => None
        };

        // collect context
        let ast_mb = gx.read().await.ast_module.clone();
        let _ast = ast_mb.ok_or_else(|| "AST support is turned off".to_string())?;

        let mut context_files: Vec<ContextFile> = Vec::new();
        if let Some(file_names) = file_names.clone() {
            for file_name in file_names.iter() {
                context_files.push(context_msg_from_file_name(gx.clone(), file_name.clone()).await);
            }
        }

        let context_files_postprocessed: Vec<ContextFile> = postprocess_at_results2(
            ccx.global_context.clone(),
            &context_files,
            tokenizer.clone(),
            MAX_TOKENS,
            false,
            context_files.len(),
        ).await;

        Ok(vec![
            ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: format_context_files_to_message_content(context_files_postprocessed).clone(),
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                ..Default::default()
            })
        ])
    }
}

