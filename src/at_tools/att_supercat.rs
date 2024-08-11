use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use serde_json::Value;

use tokio::sync::{Mutex as AMutex};
use async_trait::async_trait;
use crate::ast::ast_index::RequestSymbolType;
use crate::at_commands::at_commands::{AtCommandsContext, vec_context_file_to_context_tools};
use crate::at_commands::at_file::{at_file_repair_candidates, file_repair_candidates, get_project_paths};
use crate::at_tools::att_file::real_file_path_candidate;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum, ContextFile};
use crate::files_correction::{correct_to_nearest_dir_path, get_all_files_in_dir_recursively};
use crate::files_in_workspace::{Document, get_file_text_from_memory_or_disk};


pub struct AttSuperCat;

#[async_trait]
impl Tool for AttSuperCat {
    async fn tool_execute(&mut self, ccx: Arc<AMutex<AtCommandsContext>>, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
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
        let detail_level = match args.get("detail_level") {
            Some(Value::String(s)) => s.to_string(),
            Some(v) => return Err(format!("argument `detail_level` is not a string: {:?}", v)),
            None => "fulltext".to_string()
        };
        
        let usefulness = match detail_level.as_str() {
            "fulltext" => 100.,
            "skeleton" => 0.,
            _ => return Err(format!("argument `detail_level` must be one of: fulltext, skeleton: {:?}", detail_level)),
        };
        
        let global_context = ccx.lock().await.global_context.clone();
        
        let mut corrected_paths = vec![];
        for p in paths {
            if PathBuf::from(&p).extension().is_some() {
                let candidates = at_file_repair_candidates(ccx.clone(), &p, false).await;
                let file_path = real_file_path_candidate(ccx.clone(), &p, &candidates, &get_project_paths(ccx.clone()).await, false).await?;
                corrected_paths.push(file_path);
            } else {
                let candidates = correct_to_nearest_dir_path(global_context.clone(), &p, false, 10).await;
                let candidate = real_file_path_candidate(ccx.clone(), &p, &candidates, &get_project_paths(ccx.clone()).await, true).await?;
                let files_in_dir = get_all_files_in_dir_recursively(global_context.clone(), &PathBuf::from(candidate)).await;
                corrected_paths.extend(files_in_dir.into_iter().map(|x|x.to_string_lossy().to_string()));
            }
        }
        
        // drop duplicates
        let corrected_paths = corrected_paths.into_iter().collect::<HashSet<_>>().into_iter().collect::<Vec<_>>();
        
        let mut context_files_in = vec![];
        
        if !symbols_str.is_empty() {
            let ast_arc = global_context.read().await.ast_module.clone().unwrap();
            let ast_lock = ast_arc.read().await;
            for p in corrected_paths.iter() {
                let mut doc = Document::new(&PathBuf::from(p));
                let text = get_file_text_from_memory_or_disk(global_context.clone(), &PathBuf::from(p)).await?.to_string();
                doc.update_text(&text);
                let doc_syms = ast_lock.get_file_symbols(RequestSymbolType::All, &doc).await?
                    .symbols.iter().map(|s|(s.name.clone(), s.guid.clone())).collect::<Vec<_>>();
                
                let sym_set = doc_syms.iter()
                    .filter(|(s_name, _)| symbols_str.contains(s_name))
                    .map(|(_, s_uuid)|s_uuid.clone())
                    .collect::<Vec<_>>();
                
                let text = doc.text.map(|t|t.to_string()).unwrap_or("".to_string());
                let cf = ContextFile {
                    file_name: p.clone(),
                    file_content: text.clone(),
                    line1: 0,
                    line2: text.lines().count(),
                    symbol: sym_set,
                    gradient_type: -1,
                    usefulness,
                    is_body_important: false,
                };
                context_files_in.push(cf);
            }
        }
        
        let filenames_present = context_files_in.iter().map(|x|x.file_name.clone()).collect::<Vec<_>>();
        for p in corrected_paths.iter().filter(|x|!filenames_present.contains(x)) {
            let text = get_file_text_from_memory_or_disk(global_context.clone(), &PathBuf::from(p)).await?.to_string();
            let cf = ContextFile {
                file_name: p.clone(),
                file_content: text.clone(),
                line1: 0,
                line2: text.lines().count(),
                symbol: vec![],
                gradient_type: -1,
                usefulness,
                is_body_important: false,
            };
            context_files_in.push(cf);
        }

        let mut results = vec_context_file_to_context_tools(context_files_in);
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: "Attached supercat results below".to_string(),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));
        
        Ok(results)
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
