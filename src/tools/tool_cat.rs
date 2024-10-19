use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use serde_json::Value;

use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, ContextFile};
use crate::files_correction::{correct_to_nearest_dir_path, get_project_dirs};
use crate::files_in_workspace::{get_file_text_from_memory_or_disk, ls_files};


pub struct ToolCat;


#[async_trait]
impl Tool for ToolCat {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let mut corrections = false;
        let paths = match args.get("paths") {
            Some(Value::String(s)) => {
                let paths = s.split(",").map(|x|x.trim().to_string()).collect::<Vec<_>>();
                paths
            },
            Some(v) => return Err(format!("argument `paths` is not a string: {:?}", v)),
            None => return Err("Missing argument `paths`".to_string())
        };
        let symbols = match args.get("symbols") {
            Some(Value::String(s)) => {
                if s == "*" {
                    vec![]
                } else {
                    s.split(",")
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect::<Vec<_>>()
                }
            },
            Some(v) => return Err(format!("argument `symbols` is not a string: {:?}", v)),
            None => vec![],
        };
        let skeleton = match args.get("skeleton") {
            Some(Value::Bool(s)) => *s,
            Some(Value::String(s)) => {
                if s == "true" {
                    true
                } else if s == "false" {
                    false
                } else {
                    return Err(format!("argument `skeleton` is not a bool: {:?}", s));
                }
            }
            Some(v) => return Err(format!("argument `skeleton` is not a bool: {:?}", v)),
            None => false,  // the default
        };
        ccx.lock().await.pp_skeleton = skeleton;

        let (filenames_present, symbols_not_found, not_found_messages, context_files) = paths_and_symbols_to_cat(ccx.clone(), paths, symbols).await;

        let mut content = "".to_string();
        if !filenames_present.is_empty() {
            content.push_str(&format!("Paths found:\n{}\n\n", filenames_present.join("\n")));
            if !symbols_not_found.is_empty() {
                content.push_str(&format!("Symbols not found in the {} files:\n{}\n\n", filenames_present.len(), symbols_not_found.join("\n")));
                corrections = true;
            }
        }
        if !not_found_messages.is_empty() {
            content.push_str(&format!("Path problems:\n\n{}\n\n", not_found_messages.join("\n\n")));
            corrections = true;
        }

        let mut results = context_files.into_iter().map(|i|ContextEnum::ContextFile(i)).collect::<Vec<ContextEnum>>();

        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(content),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok((corrections, results))
    }
}

pub async fn paths_and_symbols_to_cat(
    ccx: Arc<AMutex<AtCommandsContext>>,
    paths: Vec<String>,
    arg_symbols: Vec<String>,
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<ContextFile>)
{
    let (gcx, top_n) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.global_context.clone(), ccx_locked.top_n)
    };
    let ast_service_opt = gcx.read().await.ast_service.clone();

    let mut not_found_messages = vec![];
    let mut corrected_paths = vec![];

    for p in paths {
        // both not fuzzy
        let candidates_file = file_repair_candidates(gcx.clone(), &p, top_n, false).await;
        let candidates_dir = correct_to_nearest_dir_path(gcx.clone(), &p, false, top_n).await;

        if !candidates_file.is_empty() || candidates_dir.is_empty() {
            let file_path = match return_one_candidate_or_a_good_error(gcx.clone(), &p, &candidates_file, &get_project_dirs(gcx.clone()).await, false).await {
                Ok(f) => f,
                Err(e) => { not_found_messages.push(e); continue;}
            };
            corrected_paths.push(file_path);
        } else {
            let candidate = match return_one_candidate_or_a_good_error(gcx.clone(), &p, &candidates_dir, &get_project_dirs(gcx.clone()).await, true).await {
                Ok(f) => f,
                Err(e) => { not_found_messages.push(e); continue;}
            };
            let files_in_dir = ls_files(&PathBuf::from(candidate), false).unwrap_or(vec![]);
            corrected_paths.extend(files_in_dir.into_iter().map(|x|x.to_string_lossy().to_string()));
        }
    }

    let unique_paths = corrected_paths.into_iter().collect::<HashSet<_>>().into_iter().collect::<Vec<_>>();

    let mut context_files = vec![];
    let mut symbols_found = HashSet::<String>::new();

    if let Some(ast_service) = ast_service_opt {
        let ast_index = ast_service.lock().await.ast_index.clone();
        for p in unique_paths.iter() {
            let doc_syms = crate::ast::ast_db::doc_defs(ast_index.clone(), &p).await;
            // s.name() means the last part of the path
            // symbols.contains means exact match in comma-separated list
            let mut syms_def_in_this_file = vec![];
            for looking_for in arg_symbols.iter() {
                let colon_colon_looking_for = format!("::{}", looking_for.trim());
                for x in doc_syms.iter() {
                    if x.path().ends_with(colon_colon_looking_for.as_str()) {
                        syms_def_in_this_file.push(x.clone());
                    }
                }
                symbols_found.insert(looking_for.clone());
            }

            for sym in syms_def_in_this_file {
                let cf = ContextFile {
                    file_name: p.clone(),
                    file_content: "".to_string(),
                    line1: sym.full_line1(),
                    line2: sym.full_line2(),
                    symbols: vec![sym.path()],
                    gradient_type: -1,
                    usefulness: 100.0,
                };
                context_files.push(cf);
            }
        }
    }

    let mut symbols_not_found = vec![];
    for looking_for in arg_symbols.iter() {
        if !symbols_found.contains(looking_for) {
            symbols_not_found.push(looking_for.clone());
        }
    }

    let filenames_got_symbols_for = context_files.iter().map(|x|x.file_name.clone()).collect::<Vec<_>>();
    for p in unique_paths.iter().filter(|x|!filenames_got_symbols_for.contains(x)) {
        // don't have symbols for these, so we need to mention them as files, without a symbol, analog of @file
        match get_file_text_from_memory_or_disk(gcx.clone(), &PathBuf::from(p)).await {
            Ok(text) => {
                let cf = ContextFile {
                    file_name: p.clone(),
                    file_content: "".to_string(),
                    line1: 1,
                    line2: text.lines().count(),
                    symbols: vec![],
                    gradient_type: -1,
                    usefulness: 0.0,
                };
                context_files.push(cf);
            },
            Err(e) => {
                not_found_messages.push(format!("{}: {}", p, e));
            }
        }
    }
    let filenames_present = context_files.iter().map(|x|x.file_name.clone()).collect::<HashSet<_>>().into_iter().collect::<Vec<_>>();
    (filenames_present, symbols_not_found, not_found_messages, context_files)
}
