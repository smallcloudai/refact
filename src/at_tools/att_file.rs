use std::collections::HashMap;
use std::path::PathBuf;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::{at_file_repair_candidates, get_project_paths, text_on_clip};
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum, ContextFile};
use crate::files_in_workspace::get_file_text_from_memory_or_disk;


pub struct AttFile;

async fn get_file_text(ccx: &mut AtCommandsContext, file_path: &String, candidates: &Vec<String>, project_paths: &Vec<PathBuf>) -> Result<(String, String), String> {
    let f_path = PathBuf::from(file_path);
    let mut corrected = PathBuf::new();

    if candidates.is_empty() {
        if f_path.is_absolute() {
            if !project_paths.iter().any(|x|x.starts_with(&f_path)) {
                return Err(format!("The absolute path {:?} points outside of any workspace directories", f_path));
            }
        }
        let similar_files_str = at_file_repair_candidates(&file_path, ccx, true).await.iter().take(10).cloned().collect::<Vec<_>>().join("\n");
        if f_path.is_relative() {
            let projpath_options = project_paths.iter().map(|x|x.join(&f_path)).filter(|x|x.is_file()).collect::<Vec<_>>();
            if projpath_options.len() > 1 {
                let projpath_options_str = projpath_options.iter().map(|x|x.to_string_lossy().to_string()).collect::<Vec<_>>().join("\n");
                return Err(format!("The path {:?} is ambiguous.\n\nAdding project path, it might be:\n{:?}\n\nAlso, there are similar file names:\n{}", f_path, projpath_options_str, similar_files_str));
            }
            if projpath_options.is_empty() {
                if similar_files_str.is_empty() {
                    return Err(format!("The path {:?} does not exist. There are no similar names either.", f_path));
                } else {
                    return Err(format!("The path {:?} does not exist.\n\nThere are files with similar names however:\n{}", f_path, similar_files_str));
                }
            } else {
                corrected = projpath_options[0].clone();
            }
        }
    } else {
        corrected = PathBuf::from(candidates[0].clone());
    }

    if candidates.len() > 1 {
        return Err(format!("The path {:?} is ambiguous.\n\nIt could be interpreted as:\n{}", file_path, candidates.join("\n")));
    }

    get_file_text_from_memory_or_disk(ccx.global_context.clone(), &corrected).await.map(|x|(corrected.to_string_lossy().to_string(), x))
}

#[async_trait]
impl Tool for AttFile {
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let p = match args.get("path") {
            Some(Value::String(s)) => s,
            Some(v) => { return Err(format!("argument `path` is not a string: {:?}", v)) },
            None => { return Err("argument `path` is missing".to_string()) }
        };

        let mut results = vec![];
        let candidates = at_file_repair_candidates(p, ccx, false).await;
        let content = match get_file_text(ccx, p, &candidates, &get_project_paths(ccx).await).await {
            Ok((file_name, file_content)) => {
                let res = ContextFile {
                    file_name,
                    file_content: file_content.clone(),
                    line1: 0,
                    line2: file_content.lines().count(),
                    symbol: Uuid::default(),
                    gradient_type: 0,
                    usefulness: 100.0,
                    is_body_important: false
                };
                let content = text_on_clip(&res, true);
                results.push(ContextEnum::ContextFile(res));
                content
            }
            Err(e) => e
        };

        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
        }));

        Ok(results)
    }
}
