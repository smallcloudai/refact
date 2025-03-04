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
use tracing::info;


use crate::at_commands::at_commands::{vec_context_file_to_context_tools, AtCommandsContext};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, ContextFile};
use crate::files_correction::shortify_paths;
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::global_context::GlobalContext;
use crate::tools::scope_utils::{resolve_scope, validate_scope_files};
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

    let files_to_search = resolve_scope(gcx.clone(), scope)
        .await
        .and_then(|files| validate_scope_files(files, scope))?;
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
    
    results.sort_by(|a, b| a.file_name.cmp(&b.file_name));
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

fn path_depth(path: &str) -> usize {
    path.chars().filter(|&c| c == '/' || c == '\\').count()
}

async fn smart_compress_results(
    search_results: &Vec<ContextFile>,
    file_results: &HashMap<String, Vec<&ContextFile>>,
    gcx: Arc<ARwLock<GlobalContext>>,
    pattern: &str,
) -> String {
    const MAX_OUTPUT_SIZE: usize = 4 * 1024;
    const MAX_MATCHES_PER_FILE: usize = 25;
    
    let total_matches = search_results.len();
    let total_files = file_results.len();
    
    let mut content = format!("Regex search results for pattern '{}':\n\n", pattern);
    content.push_str(&format!("Found {} matches across {} files\n\n", total_matches, total_files));
    
    let mut file_paths: Vec<String> = file_results.keys().cloned().collect();
    
    file_paths.sort_by(|a, b| {
        let a_depth = path_depth(a);
        let b_depth = path_depth(b);
        if a_depth == b_depth {
            a.cmp(b)
        } else {
            a_depth.cmp(&b_depth)
        }
    });
    
    let mut used_files: HashSet<String> = HashSet::new();
    let mut estimated_size = content.len();
    
    for file_path in file_paths.iter() {
        if used_files.contains(file_path) {
            continue;
        }
        
        let short_path = shortify_paths(gcx.clone(), &vec![file_path.clone()]).await.get(0).unwrap().clone();
        let file_matches = file_results.get(file_path).unwrap();
        
        let file_header = format!("{}: ({} matches)\n", short_path, file_matches.len());
        estimated_size += file_header.len();
        content.push_str(&file_header);
        
        let matches_to_show = std::cmp::min(file_matches.len(), MAX_MATCHES_PER_FILE);
        
        for file_match in file_matches.iter().take(matches_to_show).sorted_by_key(|m| m.line1) {
            let match_line = format!("    line {}\n", file_match.line1);
            estimated_size += match_line.len();
            content.push_str(&match_line);
        }
        
        if file_matches.len() > MAX_MATCHES_PER_FILE {
            let summary = format!("    ... and {} more matches in this file\n", file_matches.len() - MAX_MATCHES_PER_FILE);
            estimated_size += summary.len();
            content.push_str(&summary);
        }
        
        content.push('\n');
        estimated_size += 1;
        used_files.insert(file_path.clone());
        
        if estimated_size > MAX_OUTPUT_SIZE * 3 / 4 {
            break;
        }
    }
    
    if file_paths.len() > used_files.len() {
        let remaining_files = file_paths.len() - used_files.len();
        content.push_str(&format!("... and {} more files with matches (not shown due to size limit)\n", remaining_files));
    }
    
    if estimated_size > MAX_OUTPUT_SIZE {
        info!("Compressing regex_search output: estimated {} bytes (exceeds 4KB limit)", estimated_size);
        content.push_str("\nNote: Output has been compressed. Use more specific patterns or scopes for detailed results.");
    }
    
    content
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
        


        let ccx_lock = ccx.lock().await;
        let gcx = ccx_lock.global_context.clone();
        let subchat_tx = ccx_lock.subchat_tx.clone();
        drop(ccx_lock);
        let _ = subchat_tx.lock().await.send(json!({
            "progress": format!("Starting regex search for pattern '{}' in scope '{}'...", pattern, scope)
        }));
        
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

        let _ = subchat_tx.lock().await.send(json!({
            "progress": format!("Search complete. Found {} matches.", search_results.len())
        }));

        let mut file_results: HashMap<String, Vec<&ContextFile>> = HashMap::new();
        search_results.iter().for_each(|rec| {
            file_results.entry(rec.file_name.clone()).or_insert(vec![]).push(rec)
        });
        
        let content = smart_compress_results(&search_results, &file_results, gcx.clone(), &pattern).await;

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