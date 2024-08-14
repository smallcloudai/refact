use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use serde_json::Value;

use tokio::sync::{Mutex as AMutex};
use async_trait::async_trait;
use tracing::info;
use crate::ast::ast_index::RequestSymbolType;
use crate::at_commands::at_commands::{AtCommandsContext, vec_context_file_to_context_tools};
use crate::at_commands::at_file::{at_file_repair_candidates, get_project_paths};
use crate::at_tools::att_file::real_file_path_candidate;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum, ContextFile};
use crate::files_correction::{correct_to_nearest_dir_path, get_files_in_dir};
use crate::files_in_workspace::{Document, get_file_text_from_memory_or_disk};


pub struct AttCat;

#[async_trait]
impl Tool for AttCat {
    async fn tool_execute(
        &mut self, 
        ccx: Arc<AMutex<AtCommandsContext>>, 
        tool_call_id: &String, 
        args: &HashMap<String, Value>
    ) -> Result<Vec<ContextEnum>, String> {
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
                let symbols = s.split(",").map(|x|x.trim().to_string()).collect::<Vec<_>>();
                symbols
            },
            Some(v) => return Err(format!("argument `symbols` is not a string: {:?}", v)),
            None => vec![],
        };
        let skeleton = match args.get("skeleton") {
            Some(Value::Bool(s)) => *s,
            Some(v) => return Err(format!("argument `skeleton` is not a bool: {:?}", v)),
            None => false,
        };
        let global_context = ccx.lock().await.global_context.clone();
        
        let mut files_not_found_errs = vec![];
        let mut corrected_paths = vec![];
        
        for p in paths {
            if PathBuf::from(&p).extension().is_some() {
                let candidates = at_file_repair_candidates(ccx.clone(), &p, false).await;
                let file_path = match real_file_path_candidate(ccx.clone(), &p, &candidates, &get_project_paths(ccx.clone()).await, false).await {
                    Ok(f) => f,
                    Err(e) => { files_not_found_errs.push(e); continue;}
                };
                corrected_paths.push(file_path);
            } else {
                let candidates = correct_to_nearest_dir_path(global_context.clone(), &p, false, 10).await;
                let candidate = match real_file_path_candidate(ccx.clone(), &p, &candidates, &get_project_paths(ccx.clone()).await, true).await {
                    Ok(f) => f,
                    Err(e) => { files_not_found_errs.push(e); continue;}
                };
                let files_in_dir = get_files_in_dir(global_context.clone(), &PathBuf::from(candidate)).await;
                corrected_paths.extend(files_in_dir.into_iter().map(|x|x.to_string_lossy().to_string()));
            }
        }
        
        // drop duplicates
        let corrected_paths = corrected_paths.into_iter().collect::<HashSet<_>>().into_iter().collect::<Vec<_>>();
        
        let mut context_files_in = vec![];
        let mut symbols_found = vec![];
        
        if !symbols_str.is_empty() {
            let ast_arc = global_context.read().await.ast_module.clone().unwrap();
            let ast_lock = ast_arc.read().await;
            for p in corrected_paths.iter() {
                let mut doc = Document::new(&PathBuf::from(p));
                let text = get_file_text_from_memory_or_disk(global_context.clone(), &PathBuf::from(p)).await?.to_string();
                doc.update_text(&text);
                
                let doc_syms = ast_lock.get_file_symbols(RequestSymbolType::All, &doc).await?.symbols;
                let syms_intersection = doc_syms.into_iter().filter(|s|symbols_str.contains(&s.name)).collect::<Vec<_>>();
                for sym in syms_intersection {
                    symbols_found.push(sym.name.clone());
                    let cf = ContextFile {
                        file_name: p.clone(),
                        file_content: "".to_string(),
                        line1: sym.full_range.start_point.row + 1,
                        line2: sym.full_range.end_point.row + 1,
                        symbols: vec![sym.guid.clone()],
                        gradient_type: -1,
                        usefulness: 100.,
                        is_body_important: false,
                    };
                    context_files_in.push(cf);
                }
            }
        }

        let filenames_present = context_files_in.iter().map(|x|x.file_name.clone()).collect::<Vec<_>>();
        for p in corrected_paths.iter().filter(|x|!filenames_present.contains(x)) {
            let text = get_file_text_from_memory_or_disk(global_context.clone(), &PathBuf::from(p)).await?.to_string();
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
            content.push_str(&format!("Files found:\n{}\n\n", filenames_present.join("\n")));
            
            let symbols_not_found = symbols_str.iter().filter(|x|!symbols_found.contains(x)).cloned().collect::<Vec<_>>();
            if !symbols_not_found.is_empty() {
                content.push_str(&format!("Symbols not found in the {} files:\n{}\n\n", filenames_present.len(), symbols_not_found.join("\n")));
            }
        }
        if !files_not_found_errs.is_empty() {
            content.push_str(&format!("Files not found:\n{}\n\n", files_not_found_errs.join("\n\n")));
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

        Ok(results)
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
