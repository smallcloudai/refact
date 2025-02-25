use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use futures::future::join_all;
use itertools::Itertools;
use regex::Regex;
use serde_json::{Value, json};
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;


use crate::at_commands::at_commands::{vec_context_file_to_context_tools, AtCommandsContext};
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, ContextFile};
use crate::files_correction::{correct_to_nearest_dir_path, get_project_dirs, shortify_paths};
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::global_context::GlobalContext;
use crate::tools::tools_description::Tool;

pub struct ToolRegexSearch;

async fn search_single_file(
    gcx: Arc<ARwLock<GlobalContext>>,
    file_path: String,
    regex: &Regex,
) -> Vec<ContextFile> {
    let file_content = match get_file_text_from_memory_or_disk(gcx.clone(), &PathBuf::from(&file_path)).await {
        Ok(content) => content.to_string(),
        Err(_) => return Vec::new(),
    };

    let lines: Vec<&str> = file_content.lines().collect();
    let mut file_results = Vec::new();
    
    for (line_idx, line) in lines.iter().enumerate() {
        if regex.is_match(line) {
            let line_num = line_idx + 1;
            let context_start = line_idx.saturating_sub(2);
            let context_end = (line_idx + 3).min(lines.len());
            
            let context_content = lines[context_start..context_end].join("\n");
            
            file_results.push(ContextFile {
                file_name: file_path.clone(),
                file_content: context_content,
                line1: line_num as usize,
                line2: line_num as usize,
                symbols: vec![],
                gradient_type: -1,
                usefulness: 100.0,
            });
        }
    }
    
    file_results
}

struct SearchProgress {
    total_files: usize,
    processed_files: AtomicUsize,
    total_matches: AtomicUsize,
}

async fn search_files_with_regex(
    gcx: Arc<ARwLock<GlobalContext>>,
    pattern: &str,
    scope: &String,
    subchat_tx: Option<Arc<AMutex<tokio::sync::mpsc::UnboundedSender<Value>>>>,
) -> Result<Vec<ContextFile>, String> {
    let regex = Regex::new(pattern).map_err(|e| format!("Invalid regex pattern: {}", e))?;

    let files_to_search = if scope == "workspace" {
        let workspace_files = gcx.read().await.documents_state.workspace_files.lock().unwrap().clone();
        workspace_files.into_iter().map(|f| f.to_string_lossy().to_string()).collect::<Vec<_>>()
    } else {
        let scope_is_dir = scope.ends_with('/') || scope.ends_with('\\');

        if scope_is_dir {
            let dir_path = return_one_candidate_or_a_good_error(
                gcx.clone(),
                scope,
                &correct_to_nearest_dir_path(gcx.clone(), scope, false, 10).await,
                &get_project_dirs(gcx.clone()).await,
                true,
            ).await?;
            
            let workspace_files = gcx.read().await.documents_state.workspace_files.lock().unwrap().clone();
            workspace_files.into_iter()
                .filter(|f| f.starts_with(&dir_path))
                .map(|f| f.to_string_lossy().to_string())
                .collect::<Vec<_>>()
        } else {
            vec![return_one_candidate_or_a_good_error(
                gcx.clone(),
                scope,
                &file_repair_candidates(gcx.clone(), scope, 10, false).await,
                &get_project_dirs(gcx.clone()).await,
                false,
            ).await?]
        }
    };

    if files_to_search.is_empty() {
        return Err(format!("No files found in scope: {}", scope));
    }

    // Send initial progress update
    if let Some(tx) = &subchat_tx {
        let _ = tx.lock().await.send(json!({
            "progress": format!(
                "Starting regex search for pattern '{}' in {} files...",
                pattern, files_to_search.len()
            )
        }));
    }

    let progress = Arc::new(SearchProgress {
        total_files: files_to_search.len(),
        processed_files: AtomicUsize::new(0),
        total_matches: AtomicUsize::new(0),
    });

    let regex_arc = Arc::new(regex);
    let results_mutex = Arc::new(AMutex::new(Vec::new()));
    let search_futures = files_to_search.into_iter().map(|file_path| {
        let gcx_clone = gcx.clone();
        let regex_clone = regex_arc.clone();
        let progress_clone = progress.clone();
        let results_mutex_clone = results_mutex.clone();
        let subchat_tx_clone = subchat_tx.clone();
        
        async move {
            let file_results = search_single_file(gcx_clone, file_path, &regex_clone).await;
            let processed = progress_clone.processed_files.fetch_add(1, Ordering::Relaxed) + 1;
            let matches_found = progress_clone.total_matches.fetch_add(file_results.len(), Ordering::Relaxed) + file_results.len();
            
            if let Some(tx) = &subchat_tx_clone {
                if processed % 10 == 0 || !file_results.is_empty() {
                    let _ = tx.lock().await.send(json!({
                        "progress": format!(
                            "Processed {}/{} files. Found {} matches so far...",
                            processed, progress_clone.total_files, matches_found
                        )
                    }));
                }
            }
            
            if !file_results.is_empty() {
                let mut results = results_mutex_clone.lock().await;
                results.extend(file_results);
            }
        }
    });
    
    join_all(search_futures).await;
    let mut results = results_mutex.lock().await.clone();
    
    // Sort results by file name
    results.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    
    // Send final progress update
    if let Some(tx) = &subchat_tx {
        let _ = tx.lock().await.send(json!({
            "progress": format!(
                "Search complete. Found {} matches in {} files.",
                results.len(), progress.processed_files.load(Ordering::Relaxed)
            )
        }));
    }
    
    Ok(results)
}

#[async_trait]
impl Tool for ToolRegexSearch {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let pattern = match args.get("pattern") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `pattern` is not a string: {:?}", v)),
            None => return Err("Missing argument `pattern` in the regex_search() call.".to_string())
        };
        
        let scope = match args.get("scope") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `scope` is not a string: {:?}", v)),
            None => return Err("Missing argument `scope` in the regex_search() call.".to_string())
        };
        


        // Get context and subchat_tx for progress updates
        let ccx_lock = ccx.lock().await;
        let gcx = ccx_lock.global_context.clone();
        let subchat_tx = ccx_lock.subchat_tx.clone();
        drop(ccx_lock);
        
        // Send initial progress message
        let _ = subchat_tx.lock().await.send(json!({
            "progress": format!("Starting regex search for pattern '{}' in scope '{}'...", pattern, scope)
        }));
        
        // Validate the pattern
        if let Err(e) = Regex::new(&pattern) {
            return Err(format!("Invalid regex pattern: {}. Please check your syntax.", e));
        }
        
        let search_results = search_files_with_regex(
            gcx.clone(), 
            &pattern, 
            &scope, 
            Some(subchat_tx.clone()),
        ).await?;
        
        if search_results.is_empty() {
            return Err("Regex search produced no results. Try adjusting your pattern or scope.".to_string());
        }

        // Send completion message
        let _ = subchat_tx.lock().await.send(json!({
            "progress": format!("Search complete. Found {} matches.", search_results.len())
        }));

        let mut content = format!("Regex search results for pattern '{}':\n\n", pattern);
        
        let mut file_results: HashMap<String, Vec<&ContextFile>> = HashMap::new();
        search_results.iter().for_each(|rec| {
            file_results.entry(rec.file_name.clone()).or_insert(vec![]).push(rec)
        });
        
        let mut used_files: HashSet<String> = HashSet::new();
        let total_matches = search_results.len();
        let total_files = file_results.len();
        
        content.push_str(&format!("Found {} matches across {} files\n\n", total_matches, total_files));
        
        for rec in search_results.iter() {
            if !used_files.contains(&rec.file_name) {
                let short_path = shortify_paths(gcx.clone(), &vec![rec.file_name.clone()]).await.get(0).unwrap().clone();
                let file_matches = file_results.get(&rec.file_name).unwrap();
                content.push_str(&format!("{}: ({} matches)\n", short_path, file_matches.len()));
                
                for file_match in file_matches.iter().sorted_by_key(|m| m.line1) {
                    content.push_str(&format!("    line {}\n", file_match.line1));
                }
                
                used_files.insert(rec.file_name.clone());
            }
        }

        let mut results = vec_context_file_to_context_tools(search_results);
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(content),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));
        
        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}