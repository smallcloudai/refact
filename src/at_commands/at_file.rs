use async_trait::async_trait;
use regex::Regex;
use serde_json::json;
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex};
use tracing::info;
use std::collections::HashSet;
use std::collections::HashMap;
use std::sync::Arc;
use strsim::normalized_damerau_levenshtein;
use url::Url;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::utils::split_file_into_chunks_from_line_inside;
use crate::files_in_jsonl::files_in_jsonl;
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::call_validation::{ChatMessage, ContextFile};
use crate::global_context::GlobalContext;


pub struct AtFile {
    pub name: String,
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtFile {
    pub fn new() -> Self {
        AtFile {
            name: "@file".to_string(),
            params: vec![
                Arc::new(AMutex::new(AtParamFilePath::new()))
            ],
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum RangeKind {
    GradToCursorTwosided,
    GradToCursorPrefix,
    GradToCursorSuffix,
    Range,
}

#[derive(Debug, PartialEq)]
pub struct ColonLinesRange {
    pub kind: RangeKind,
    pub line1: usize,
    pub line2: usize,
}

pub fn range_print(range: &ColonLinesRange) -> String {
    match range.kind {
        RangeKind::GradToCursorTwosided => format!("{}", range.line1),
        RangeKind::GradToCursorPrefix => format!("-{}", range.line2),
        RangeKind::GradToCursorSuffix => format!("{}-", range.line1),
        RangeKind::Range => format!("{}-{}", range.line1, range.line2),
    }
}

pub fn colon_lines_range_from_arg(value: &mut String) -> Option<ColonLinesRange> {
    let re_hyphen = Regex::new(r":(\d+)?-(\d+)?$").unwrap();
    if let Some(captures) = re_hyphen.captures(value.clone().as_str()) {
        *value = re_hyphen.replace(value, "").to_string();
        return match (captures.get(1), captures.get(2)) {
            (Some(line1), Some(line2)) => {
                let line1 = line1.as_str().parse::<usize>().unwrap_or(0);
                let line2 = line2.as_str().parse::<usize>().unwrap_or(0);
                Some(ColonLinesRange { kind: RangeKind::Range, line1, line2 })
            },
            (Some(line1), None) => {
                let line1 = line1.as_str().parse::<usize>().unwrap_or(0);
                Some(ColonLinesRange { kind: RangeKind::GradToCursorSuffix, line1, line2: 0 })
            },
            (None, Some(line2)) => {
                let line2 = line2.as_str().parse::<usize>().unwrap_or(0);
                Some(ColonLinesRange { kind: RangeKind::GradToCursorPrefix, line1: 0, line2 })
            },
            _ => None,
        }
    }
    let re_one_number = Regex::new(r":(\d+)$").unwrap();
    if let Some(captures) = re_one_number.captures(value.clone().as_str()) {
        *value = re_one_number.replace(value, "").to_string();
        if let Some(line1) = captures.get(1) {
            let line = line1.as_str().parse::<usize>().unwrap_or(0);
            return Some(ColonLinesRange { kind: RangeKind::GradToCursorTwosided, line1: line, line2: 0 });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_range() {
        {
            let mut value = String::from(":10-20");
            let result = colon_lines_range_from_arg(&mut value);
            assert_eq!(result, Some(ColonLinesRange { kind: RangeKind::Range, line1: 10, line2: 20 }));
        }
        {
            let mut value = String::from(":5-");
            let result = colon_lines_range_from_arg(&mut value);
            assert_eq!(result, Some(ColonLinesRange { kind: RangeKind::GradToCursorSuffix, line1: 5, line2: 0 }));
        }
        {
            let mut value = String::from(":-15");
            let result = colon_lines_range_from_arg(&mut value);
            assert_eq!(result, Some(ColonLinesRange { kind: RangeKind::GradToCursorPrefix, line1: 0, line2: 15 }));
        }
        {
            let mut value = String::from(":25");
            let result = colon_lines_range_from_arg(&mut value);
            assert_eq!(result, Some(ColonLinesRange { kind: RangeKind::GradToCursorTwosided, line1: 25, line2: 0 }));
        }
        {
            let mut value = String::from("invalid");
            let result = colon_lines_range_from_arg(&mut value);
            assert_eq!(result, None);

        }
    }
}

fn chunks_into_context_file(
    result_above: Vec<((usize, usize), String)>,
    results_below: Vec<((usize, usize), String)>,
    file_name: &String,
) -> Vec<ContextFile> {
    let max_val = result_above.len().max(results_below.len());
    let mut usefulness_vec = vec![];
    for idx in 0..max_val + 1 {
        usefulness_vec.push(100.0 * (idx as f32 / max_val as f32));
    }
    let reversed_vec: Vec<f32> = usefulness_vec.iter().cloned().rev().collect();
    let usefulness_above: Vec<f32> = reversed_vec[..result_above.len()].to_vec().iter().cloned().rev().collect();

    let mut vector_of_context_file: Vec<ContextFile> = vec![];
    for (idx, ((line1, line2), text_above)) in result_above.iter().enumerate() {
        vector_of_context_file.push({
            ContextFile {
                file_name: file_name.clone(),
                file_content: text_above.clone(),
                line1: *line1,
                line2: *line2,
                usefulness: *usefulness_above.get(idx).unwrap_or(&100.),
            }
        })
    }

    let usefulness_below = reversed_vec[1..].to_vec();
    for (idx, ((line1, line2), text_below)) in results_below.iter().enumerate() {
        vector_of_context_file.push({
            ContextFile {
                file_name: file_name.clone(),
                file_content: text_below.clone(),
                line1: *line1,
                line2: *line2,
                usefulness: *usefulness_below.get(idx).unwrap_or(&0.0),

            }
        })
    }
    vector_of_context_file
}

async fn files_cache_rebuild_as_needed(global_context: Arc<ARwLock<GlobalContext>>)
-> (Arc<HashMap<String, String>>, Arc<Vec<String>>)
{
    let cache_dirty_arc: Arc<AMutex<bool>>;
    let mut cache_correction_arc: Arc<HashMap<String, String>>;
    let mut cache_fuzzy_arc: Arc<Vec<String>>;
    {
        let gcx_locked = global_context.read().await;
        cache_dirty_arc = gcx_locked.documents_state.cache_dirty.clone();
        cache_correction_arc = gcx_locked.documents_state.cache_correction.clone();
        cache_fuzzy_arc = gcx_locked.documents_state.cache_fuzzy.clone();
    }
    let mut cache_dirty_ref = cache_dirty_arc.lock().await;
    if *cache_dirty_ref {
        // Rebuild, cache_dirty_arc stays locked.
        // Any other thread will wait at this if until the rebuild is complete.
        // Sources:
        // - documents_state.document_map
        // - cx_locked.documents_state.workspace_files
        // - global_context.read().await.cmdline.files_jsonl_path
        info!("rebuilding files cache...");
        let file_paths_from_memory = global_context.read().await.documents_state.document_map.read().await.keys().cloned().collect::<Vec<Url>>();
        let urls_from_workspace: Vec<Url> = global_context.read().await.documents_state.workspace_files.lock().unwrap().clone();
        let paths_in_jsonl: Vec<Url> = files_in_jsonl(global_context.clone()).await.iter()
            .map(|doc| doc.uri.clone())
            .collect();

        let mut cache_correction = HashMap::<String, String>::new();
        let mut cache_fuzzy_set = HashSet::<String>::new();
        for url in file_paths_from_memory.into_iter().chain(urls_from_workspace.into_iter().chain(paths_in_jsonl.into_iter())) {
            let path = url.to_file_path().ok().map(|p| p.to_str().unwrap_or_default().to_string()).unwrap_or_default();
            let file_name_only = url.to_file_path().ok().map(|p| p.file_name().unwrap().to_str().unwrap().to_string()).unwrap_or_default();
            cache_fuzzy_set.insert(file_name_only);

            cache_correction.insert(path.clone(), path.clone());
            // chop off directory names one by one
            let mut index = 0;
            while let Some(slashpos) = path[index .. ].find(|c| c == '/' || c == '\\') {
                let absolute_slashpos = index + slashpos;
                index = absolute_slashpos + 1;
                let slashpos_to_end = &path[index .. ];
                if !slashpos_to_end.is_empty() {
                    cache_correction.insert(slashpos_to_end.to_string(), path.clone());
                }
            }
        }
        let cache_fuzzy: Vec<String> = cache_fuzzy_set.into_iter().collect();
        info!("rebuild over, cache_correction.len is now {}", cache_correction.len());
        // info!("cache_fuzzy {:?}", cache_fuzzy);
        // info!("cache_correction {:?}", cache_correction);

        cache_correction_arc = Arc::new(cache_correction);
        cache_fuzzy_arc = Arc::new(cache_fuzzy);
        {
            let mut gcx_locked = global_context.write().await;
            gcx_locked.documents_state.cache_correction = cache_correction_arc.clone();
            gcx_locked.documents_state.cache_fuzzy = cache_fuzzy_arc.clone();
        }
        *cache_dirty_ref = false;
    }
    return (cache_correction_arc, cache_fuzzy_arc)
}

fn put_colon_back_to_arg(value: &mut String, colon: &Option<ColonLinesRange>) {
    if let Some(colon) = colon {
        value.push_str(":");
        value.push_str(range_print(colon).as_str());
    }
}

async fn parameter_repair_candidates(
    value: &String,
    context: &AtCommandsContext,
    top_n: usize
) -> Vec<String>
{
    let mut correction_candidate = value.clone();
    let colon_mb = colon_lines_range_from_arg(&mut correction_candidate);

    let (cache_correction_arc, cache_fuzzy_arc) = files_cache_rebuild_as_needed(context.global_context.clone()).await;
    // it's dangerous to use cache_correction_arc without a mutex, but should be fine as long as it's read-only
    // (another thread never writes to the map itself, it can only replace the arc with a different map)

    if let Some(fixed) = (*cache_correction_arc).get(&correction_candidate) {
        let mut x = fixed.clone();
        put_colon_back_to_arg(&mut x, &colon_mb);
        info!("@file found {:?} in cache_correction, returning [{:?}]", correction_candidate, x);
        return vec![x];
    }

    info!("fuzzy search {:?}, cache_fuzzy_arc.len={}", correction_candidate, cache_fuzzy_arc.len());
    // fuzzy has only filenames without path
    let mut top_n_records: Vec<(String, f64)> = Vec::with_capacity(top_n);
    for p in cache_fuzzy_arc.iter() {
        let dist = normalized_damerau_levenshtein(&correction_candidate, p);
        top_n_records.push((p.clone(), dist));
        if top_n_records.len() >= top_n {
            top_n_records.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            top_n_records.pop();
        }
    }

    let sorted_paths = top_n_records.iter()
        .map(|(path, _)| {
            let mut x = path.clone();
            // upgrade to full path
            if let Some(fixed) = (*cache_correction_arc).get(&x) {
                x = fixed.clone();
            }
            put_colon_back_to_arg(&mut x, &colon_mb);
            x
        })
        .collect::<Vec<String>>();
    // info!("sorted_paths: {:?}", sorted_paths);
    sorted_paths
}

#[derive(Debug)]
pub struct AtParamFilePath {
    pub name: String,
}

impl AtParamFilePath {
    pub fn new() -> Self {
        Self {
            name: "file_path".to_string()
        }
    }
}

#[async_trait]
impl AtParam for AtParamFilePath {
    fn name(&self) -> &String {
        &self.name
    }

    async fn is_value_valid(&self, value: &String, context: &AtCommandsContext) -> bool {
        let mut value = value.clone();
        colon_lines_range_from_arg(&mut value);
        let (cache_correction_arc, _cache_fuzzy_arc) = files_cache_rebuild_as_needed(context.global_context.clone()).await;
        // it's dangerous to use cache_correction_arc without a mutex, but should be fine as long as it's read-only
        // (another thread never writes to the map itself, it can only replace the arc with a different map)
        if (*cache_correction_arc).contains_key(&value) {
            info!("@file found {:?} in cache_correction", value);
            return true;
        }
        info!("@file not found {:?} in cache_correction", value);
        false
    }

    async fn complete(&self, value: &String, context: &AtCommandsContext, top_n: usize) -> Vec<String> {
        return parameter_repair_candidates(value, context, top_n).await;
    }
}

#[async_trait]
impl AtCommand for AtFile {
    fn name(&self) -> &String {
        &self.name
    }
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn can_execute(&self, args: &Vec<String>, _context: &AtCommandsContext) -> bool {
        args.len() == 1
        // let param = self.params.get(0).unwrap();
        // if let Some(arg) = args.get(0) {
        //     let mut arg_clone = arg.clone();
        //     colon_lines_range_from_arg(&mut arg_clone);
        //     if param.lock().await.is_value_valid(&arg_clone, context).await { // FIXME: is_value_valid is @file-specific, move here
        //         return true;
        //     }
        // }
        // false
    }

    async fn execute(&self, _query: &String, args: &Vec<String>, top_n: usize, context: &AtCommandsContext) -> Result<ChatMessage, String> {
        let can_execute = self.can_execute(args, context).await;
        if !can_execute {
            return Err("incorrect arguments".to_string());
        }
        let correctable_file_path = args[0].clone();
        let candidates = parameter_repair_candidates(&correctable_file_path, context, top_n).await;
        if candidates.len() == 0 {
            info!("parameter {:?} is uncorrectable :/", &correctable_file_path);
            return Err(format!("parameter {:?} is uncorrectable :/", &correctable_file_path));
        }
        let mut file_path = candidates[0].clone();

        let mut split_into_chunks = false;
        let mut cursor = 0;
        let mut line1 = 0;
        let mut line2 = 0;

        let colon = match colon_lines_range_from_arg(&mut file_path) {
            Some(x) => {
                info!("@file range: {:?}", x);
                if x.kind == RangeKind::GradToCursorTwosided {
                    split_into_chunks = true;
                    cursor = x.line1;
                }
                if x.kind == RangeKind::GradToCursorPrefix {
                    split_into_chunks = true;
                    cursor = x.line2;
                }
                if x.kind == RangeKind::GradToCursorSuffix {
                    split_into_chunks = true;
                    cursor = x.line1;
                }
                if x.kind == RangeKind::Range {
                    line1 = x.line1;
                    line2 = x.line2;
                }
                x
            },
            None => {
                split_into_chunks = true;
                cursor = 0;
                ColonLinesRange { kind: RangeKind::GradToCursorSuffix, line1: 0, line2: 0 }  // not used if split_into_chunks is true
            }
        };
        info!("@file {:?} execute range {:?}", file_path, colon);

        let mut file_text = get_file_text_from_memory_or_disk(context.global_context.clone(), &file_path).await?;
        let mut file_lines: Vec<String> = file_text.lines().map(String::from).collect();
        let lines_cnt = file_lines.len();

        if split_into_chunks {
            cursor = cursor.max(0).min(lines_cnt);
            let (mut res_above, mut res_below) = split_file_into_chunks_from_line_inside(cursor, &mut file_lines, 20);
            info!("split_into_chunks cursor: {} <= {}", cursor, lines_cnt);
            if colon.kind == RangeKind::GradToCursorPrefix {
                res_below.clear();
            }
            if colon.kind == RangeKind::GradToCursorSuffix {
                res_above.clear();
            }
            for ((line1, line2), _text) in res_above.iter() {
                info!("above: {}-{}", line1, line2);
            }
            for ((line1, line2), _text) in res_below.iter() {
                info!("below: {}-{}", line1, line2);
            }
            return Ok(ChatMessage {
                role: "context_file".to_string(),
                content: json!(chunks_into_context_file(res_above, res_below, &file_path)).to_string(),
            })
        }

        if line1 == 0 || line2 == 0 {
            return Err(format!("{} incorrect range: {}-{}", file_path, colon.line1, colon.line2));
        }
        line1 = (line1 - 1).max(0).min(lines_cnt);
        line2 = line2.max(0).min(lines_cnt);
        let lines: Vec<&str> = file_text.lines().collect();
        file_text = lines[line1 .. line2].join("\n");

        let mut vector_of_context_file: Vec<ContextFile> = vec![];
        vector_of_context_file.push(ContextFile {
            file_name: file_path.clone(),
            file_content: file_text,
            line1: line1 + 1,
            line2: line2,
            usefulness: 100.0,
        });
        Ok(ChatMessage {
            role: "context_file".to_string(),
            content: json!(vector_of_context_file).to_string(),
        })
    }
}
