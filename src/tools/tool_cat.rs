use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use serde_json::Value;

use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;

use crate::at_commands::at_commands::{AtCommandsContext, vec_context_file_to_context_tools};
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ContextEnum, ContextFile};
use crate::files_correction::{correct_to_nearest_dir_path, get_project_dirs, get_files_in_dir};
use crate::files_in_workspace::get_file_text_from_memory_or_disk;


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
        let symbols_str = match args.get("symbols") {
            Some(Value::String(s)) => {
                if s == "*" {
                    vec![]
                } else {
                    s.split(",").map(|x|x.trim().to_string()).collect::<Vec<_>>()
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
        let (gcx, top_n) = {
            let ccx_lock = ccx.lock().await;
            (ccx_lock.global_context.clone(), ccx_lock.top_n)
        };

        let mut files_not_found_errs = vec![];
        let mut corrected_paths = vec![];

        for p in paths {
            let candidates_file = file_repair_candidates(gcx.clone(), &p, top_n, false).await;
            let candidates_dir = correct_to_nearest_dir_path(gcx.clone(), &p, false, top_n).await;

            if PathBuf::from(&p).extension().is_some() || candidates_dir.is_empty() {
                let file_path = match return_one_candidate_or_a_good_error(gcx.clone(), &p, &candidates_file, &get_project_dirs(gcx.clone()).await, false).await {
                    Ok(f) => f,
                    Err(e) => { files_not_found_errs.push(e); continue;}
                };
                corrected_paths.push(file_path);
            } else {
                let candidate = match return_one_candidate_or_a_good_error(gcx.clone(), &p, &candidates_dir, &get_project_dirs(gcx.clone()).await, true).await {
                    Ok(f) => f,
                    Err(e) => { files_not_found_errs.push(e); continue;}
                };
                let files_in_dir = get_files_in_dir(gcx.clone(), &PathBuf::from(candidate)).await;
                corrected_paths.extend(files_in_dir.into_iter().map(|x|x.to_string_lossy().to_string()));
            }
        }

        // drop duplicates
        let corrected_paths = corrected_paths.into_iter().collect::<HashSet<_>>().into_iter().collect::<Vec<_>>();

        let mut context_files_in = vec![];
        let mut symbols_found = vec![];

        if !symbols_str.is_empty() {
            let gcx = ccx.lock().await.global_context.clone();
            let ast_service_opt = gcx.read().await.ast_service.clone();
            if let Some(ast_service) = ast_service_opt {
                let ast_index = ast_service.lock().await.ast_index.clone();
                for p in corrected_paths.iter() {
                    // XXX verify if it still works
                    let doc_syms = crate::ast::ast_db::doc_symbols(ast_index.clone(), &p).await;
                    let syms_intersection = doc_syms.into_iter().filter(|s|symbols_str.contains(&s.name())).collect::<Vec<_>>();
                    for sym in syms_intersection {
                        symbols_found.push(sym.path().clone());
                        let cf = ContextFile {
                            file_name: p.clone(),
                            file_content: "".to_string(),
                            line1: sym.full_range.start_point.row + 1,
                            line2: sym.full_range.end_point.row + 1,
                            symbols: vec![sym.path()],
                            gradient_type: -1,
                            usefulness: 100.0,
                            is_body_important: false,
                        };
                        context_files_in.push(cf);
                    }
                }
            } else {
                return Err("AST service is not available".to_string());
            }
        }

        let filenames_present = context_files_in.iter().map(|x|x.file_name.clone()).collect::<Vec<_>>();
        for p in corrected_paths.iter().filter(|x|!filenames_present.contains(x)) {
            let text = get_file_text_from_memory_or_disk(gcx.clone(), &PathBuf::from(p)).await?.to_string();
            let cf = ContextFile {
                file_name: p.clone(),
                file_content: "".to_string(),
                line1: 0,
                line2: text.lines().count(),
                symbols: vec![],
                gradient_type: -1,
                usefulness: 0.,
                is_body_important: false,
            };
            context_files_in.push(cf);
        }
        let filenames_present = context_files_in.iter().map(|x|x.file_name.clone()).collect::<HashSet<_>>().into_iter().collect::<Vec<_>>();

        let mut content = "".to_string();
        if !filenames_present.is_empty() {
            content.push_str(&format!("Paths found:\n{}\n\n", filenames_present.join("\n")));

            let symbols_not_found = symbols_str.iter().filter(|x|!symbols_found.contains(x)).cloned().collect::<Vec<_>>();
            if !symbols_not_found.is_empty() {
                content.push_str(&format!("Symbols not found in the {} files:\n{}\n\n", filenames_present.len(), symbols_not_found.join("\n")));
                corrections = true;
            }
        }
        if !files_not_found_errs.is_empty() {
            content.push_str(&format!("Path problems:\n\n{}\n\n", files_not_found_errs.join("\n\n")));
            corrections = true;
        }

        let mut results = vec_context_file_to_context_tools(context_files_in);
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        ccx.lock().await.pp_skeleton = skeleton;

        Ok((corrections, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
