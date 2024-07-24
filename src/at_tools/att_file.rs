use std::collections::HashMap;
use std::path::PathBuf;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::{at_file_repair_candidates, get_project_paths, text_on_clip};
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum, ContextFile};
use crate::files_correction::correct_to_nearest_dir_path;
use crate::files_in_workspace::get_file_text_from_memory_or_disk;


pub struct AttFile;

pub async fn real_file_path_candidate(
    ccx: &mut AtCommandsContext,
    file_path: &String,
    candidates: &Vec<String>,
    project_paths: &Vec<PathBuf>,
    dirs: bool,
) -> Result<String, String>{
    let mut f_path = PathBuf::from(file_path);

    if candidates.is_empty() {
        let similar_paths_str = if dirs {
            correct_to_nearest_dir_path(ccx.global_context.clone(), file_path, true, 10).await.join("\n")
        } else {
            at_file_repair_candidates(file_path, ccx, true).await.iter().take(10).cloned().collect::<Vec<_>>().join("\n")
        };
        if f_path.is_absolute() {
            if !project_paths.iter().any(|x|x.starts_with(&f_path)) {
                return Err(format!("Path {:?} is outside of project directories:\n\n{:?}\n\nThere are paths with similar names:\n{}", f_path, project_paths, similar_paths_str));
            }
        }
        if f_path.is_relative() {
            let projpath_options = project_paths.iter().map(|x|x.join(&f_path)).filter(|x|x.is_file()).collect::<Vec<_>>();
            if projpath_options.len() > 1 {
                let projpath_options_str = projpath_options.iter().map(|x|x.to_string_lossy().to_string()).collect::<Vec<_>>().join("\n");
                return Err(format!("The path {:?} is ambiguous.\n\nAdding project path, it might be:\n{:?}\n\nAlso, there are similar filepaths:\n{}", f_path, projpath_options_str, similar_paths_str));
            }
            if projpath_options.is_empty() {
                if similar_paths_str.is_empty() {
                    return Err(format!("The path {:?} does not exist. There are no similar names either.", f_path));
                } else {
                    return Err(format!("The path {:?} does not exist.\n\nThere are paths with similar names however:\n{}", f_path, similar_paths_str));
                }
            } else {
                f_path = projpath_options[0].clone();
                return Ok(f_path.to_string_lossy().to_string());
            }
        }
    }

    if candidates.len() > 1 {
        return Err(format!("The path {:?} is ambiguous.\n\nIt could be interpreted as:\n{}", file_path, candidates.join("\n")));
    }
    Ok(candidates[0].clone())
}

#[async_trait]
impl Tool for AttFile {
    async fn tool_execute(&mut self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let p = match args.get("path") {
            Some(Value::String(s)) => s,
            Some(v) => { return Err(format!("argument `path` is not a string: {:?}", v)) },
            None => { return Err("argument `path` is missing".to_string()) }
        };

        let candidates = at_file_repair_candidates(p, ccx, false).await;
        let candidate = real_file_path_candidate(ccx, p, &candidates, &get_project_paths(ccx).await, false).await?;
        let file_text_mb = get_file_text_from_memory_or_disk(ccx.global_context.clone(), &PathBuf::from(candidate.clone())).await;

        let mut results = vec![];
        let content_on_clip = match file_text_mb {
            Ok(file_content) => {
                let res = ContextFile {
                    file_name: candidate,
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
            content: content_on_clip,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok(results)
    }
}
