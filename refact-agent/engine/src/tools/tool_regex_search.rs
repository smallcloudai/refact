use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use futures::future::join_all;
use itertools::Itertools;
use fancy_regex::Regex;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tracing::info;


use crate::at_commands::at_commands::{vec_context_file_to_context_tools, AtCommandsContext};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, ContextFile};
use crate::postprocessing::pp_command_output::OutputFilter;
use crate::files_correction::shortify_paths;
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::global_context::GlobalContext;
use crate::tools::scope_utils::{resolve_scope, validate_scope_files};
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};

pub struct ToolRegexSearch {
    pub config_path: String,
}

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
        if regex.is_match(line).unwrap_or(false) {
            let line_num = (line_idx + 1) as i64;
            let context_start = line_idx.saturating_sub(2);
            let context_end = (line_idx + 3).min(lines.len());
            let context_content = lines[context_start..context_end].join("\n");
            file_results.push(ContextFile {
                file_name: file_path.clone(),
                file_content: context_content,
                line1: (line_num - 10).max(1) as usize,
                line2: (line_num + 10).min(lines.len() as i64) as usize,
                symbols: vec![],
                gradient_type: 5,
                usefulness: 100.0,
                skip_pp: false,
            });
        }
    }
    
    file_results
}

async fn search_files_with_regex(
    gcx: Arc<ARwLock<GlobalContext>>,
    pattern: &str,
    scope: &String,
) -> Result<Vec<ContextFile>, String> {
    let regex = Regex::new(pattern).map_err(|e| format!("Invalid regex pattern: {}", e))?;
    let files_to_search = resolve_scope(gcx.clone(), scope)
        .await
        .and_then(|files| validate_scope_files(files, scope))?;
    let regex_arc = Arc::new(regex);
    let results_mutex = Arc::new(AMutex::new(Vec::new()));
    let search_futures = files_to_search.into_iter().map(|file_path| {
        let gcx_clone = gcx.clone();
        let regex_clone = regex_arc.clone();
        let results_mutex_clone = results_mutex.clone();
        async move {
            let file_results = search_single_file(gcx_clone, file_path, &regex_clone).await;
            if !file_results.is_empty() {
                let mut results = results_mutex_clone.lock().await;
                results.extend(file_results);
            }
        }
    });
    join_all(search_futures).await;
    let mut results = results_mutex.lock().await.clone();
    results.sort_by(|a, b| a.file_name.cmp(&b.file_name));
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
    
    let mut used_files = HashSet::new();
    let mut estimated_size = content.len();
    let short_paths = shortify_paths(gcx.clone(), &file_paths).await;
    
    for file_path in file_paths.iter() {
        if used_files.contains(file_path) {
            continue;
        }
        let idx = file_paths.iter().position(|p| p == file_path);
        let short_path = idx
            .and_then(|i| short_paths.get(i))
            .unwrap_or(file_path);
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
        info!("Compressing `search_pattern` output: estimated {} bytes (exceeds 4KB limit)", estimated_size);
        content.push_str("\nNote: Output has been compressed. Use more specific pattern or scope for detailed results.");
    }
    content
}

#[async_trait]
impl Tool for ToolRegexSearch {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "search_pattern".to_string(),
            display_name: "Regex Search".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: false,
            experimental: false,
            description: "Search for files and folders whose names or paths match the given regular expression pattern, and also search for text matches inside files using the same pattern. Reports both path matches and text matches in separate sections.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "pattern".to_string(),
                    description: "The pattern is used to search for matching file/folder names/paths, and also for matching text inside files. Use (?i) at the start for case-insensitive search.".to_string(),
                    param_type: "string".to_string(),
                },
                ToolParam {
                    name: "scope".to_string(),
                    description: "'workspace' to search all files in workspace, 'dir/subdir/' to search in files within a directory, 'dir/file.ext' to search in a single file.".to_string(),
                    param_type: "string".to_string(),
                }
            ],
            parameters_required: vec!["pattern".to_string(), "scope".to_string()],
        }
    }
    
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let pattern = match args.get("pattern") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `pattern` is not a string: {:?}", v)),
            None => return Err("Missing argument `pattern` in the `search_pattern()` call.".to_string())
        };
        
        let scope = match args.get("scope") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `scope` is not a string: {:?}", v)),
            None => return Err("Missing argument `scope` in the search_pattern() call.".to_string())
        };
        
        let ccx_lock = ccx.lock().await;
        let gcx = ccx_lock.global_context.clone();
        drop(ccx_lock);

        let files_in_scope = resolve_scope(gcx.clone(), &scope)
            .await
            .and_then(|files| validate_scope_files(files, &scope))?;

        let mut all_content = String::new();
        let mut all_search_results = Vec::new();
        
        // 1. Path matches
        let regex = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(e) => return Err(format!("Invalid regex pattern '{}': {}. Please check your syntax.", pattern, e)),
        };
        let mut path_matches: Vec<String> = files_in_scope
            .iter()
            .filter(|path| regex.is_match(path).unwrap_or(false))
            .cloned()
            .collect();
        path_matches.sort();

        all_content.push_str("Path matches (file/folder names):\n");
        if path_matches.is_empty() {
            all_content.push_str("  No files or folders matched by name.\n");
        } else {
            for path in &path_matches {
                all_content.push_str(&format!("  {}\n", path));
            }
        }

        // --- Add ContextFiles for path matches (files only) ---
        for path in &path_matches {
            match get_file_text_from_memory_or_disk(gcx.clone(), &PathBuf::from(path)).await {
                Ok(text) => {
                    let total_lines = text.lines().count();
                    let cf = ContextFile {
                        file_name: path.clone(),
                        file_content: "".to_string(),
                        line1: 1,
                        line2: total_lines.max(1),
                        symbols: vec![],
                        gradient_type: 4,
                        usefulness: 80.0,
                        skip_pp: false,
                    };
                    all_search_results.push(cf);
                },
                Err(_) => {
                    tracing::warn!("Failed to read file '{}'. Skipping...", path);
                }
            }
        }

        // 2. Text matches
        let search_results = search_files_with_regex(gcx.clone(), &pattern, &scope).await?;
        all_content.push_str("\nText matches inside files:\n");
        if search_results.is_empty() {
            all_content.push_str("  No text matches found in any file.\n");
        } else {
            let mut file_results: HashMap<String, Vec<&ContextFile>> = HashMap::new();
            search_results.iter().for_each(|rec| {
                file_results.entry(rec.file_name.clone()).or_insert(vec![]).push(rec)
            });
            let pattern_content = smart_compress_results(&search_results, &file_results, gcx.clone(), &pattern).await;
            all_content.push_str(&pattern_content);
            all_search_results.extend(search_results);
        }
        
        if all_search_results.is_empty() {
            return Err("All pattern searches produced no results. Try adjusting your pattern or scope.".to_string());
        }

        let mut results = vec_context_file_to_context_tools(all_search_results);
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(all_content),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            output_filter: Some(OutputFilter::no_limits()), // Already compressed internally
            ..Default::default()
        }));

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}
