use std::path::PathBuf;
use async_trait::async_trait;
use regex::Regex;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tracing::info;
use std::sync::Arc;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam, vec_context_file_to_context_tools};
use crate::at_commands::execute_at::{AtCommandMember, correct_at_arg};
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::call_validation::{ContextFile, ContextEnum};
use crate::files_correction::correct_to_nearest_dir_path;
use crate::global_context::GlobalContext;

pub struct AtFile {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtFile {
    pub fn new() -> Self {
        AtFile {
            params: vec![
                Arc::new(AMutex::new(AtParamFilePath::new()))
            ],
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum RangeKind {
    GradToCursorTwoSided,
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
        RangeKind::GradToCursorTwoSided => format!("{}", range.line1),
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
            return Some(ColonLinesRange { kind: RangeKind::GradToCursorTwoSided, line1: line, line2: 0 });
        }
    }
    None
}

fn gradient_type_from_range_kind(range: &Option<ColonLinesRange>) -> i32 {
    if let Some(range) = range {
        match range.kind {
            RangeKind::GradToCursorTwoSided => 1,
            RangeKind::GradToCursorPrefix => 2,
            RangeKind::GradToCursorSuffix => 3,
            RangeKind::Range => 4,
        }
    } else {
        0
    }
}

fn put_colon_back_to_arg(value: &mut String, colon: &Option<ColonLinesRange>) {
    if let Some(colon) = colon {
        value.push_str(":");
        value.push_str(range_print(colon).as_str());
    }
}

pub async fn file_repair_candidates(
    gcx: Arc<ARwLock<GlobalContext>>,
    value: &String,
    top_n: usize,
    fuzzy: bool
) -> Vec<String> {
    let mut correction_candidate = value.clone();
    let colon_mb = colon_lines_range_from_arg(&mut correction_candidate);

    let result: Vec<String> = crate::files_correction::correct_to_nearest_filename(
        gcx.clone(),
        &correction_candidate,
        fuzzy,
        top_n,
    ).await;

    result.iter().map(|x| {
        let mut x = x.clone();
        put_colon_back_to_arg(&mut x, &colon_mb);
        x
    }).collect()
}

pub async fn real_file_path_candidate(
    gcx: Arc<ARwLock<GlobalContext>>,
    file_path: &String,
    candidates: &Vec<String>,
    project_paths: &Vec<PathBuf>,
    dirs: bool,
) -> Result<String, String>{
    let mut f_path = PathBuf::from(file_path);

    if candidates.is_empty() {
        let similar_paths_str = if dirs {
            correct_to_nearest_dir_path(gcx.clone(), file_path, true, 10).await.join("\n")
        } else {
            let file_name = f_path.file_name().ok_or(format!("unable to get file name from path: {:?}", f_path))?.to_string_lossy().to_string();
            file_repair_candidates(gcx.clone(), &file_name, 10, true).await.iter().take(10).cloned().collect::<Vec<_>>().join("\n")
        };
        if f_path.is_absolute() {
            if !project_paths.iter().any(|x|f_path.starts_with(x)) {
                return Err(format!("Path {:?} is outside of project directories:\n\n{:?}\n\nThere are paths with similar names:\n{}", f_path, project_paths, similar_paths_str));
            }
            return if similar_paths_str.is_empty() {
                Err(format!("The path {:?} does not exist. There are no similar names either.", f_path))
            } else {
                Err(format!("The path {:?} does not exist.\n\nThere are paths with similar names however:\n{}", f_path, similar_paths_str))
            }
        }
        if f_path.is_relative() {
            let projpath_options = project_paths.iter().map(|x|x.join(&f_path)).filter(|x|x.is_file()).collect::<Vec<_>>();
            if projpath_options.len() > 1 {
                let projpath_options_str = projpath_options.iter().map(|x|x.to_string_lossy().to_string()).collect::<Vec<_>>().join("\n");
                return Err(format!("The path {:?} is ambiguous.\n\nAdding project path, it might be:\n{:?}\n\nAlso, there are similar filepaths:\n{}", f_path, projpath_options_str, similar_paths_str));
            }
            return if projpath_options.is_empty() {
                if similar_paths_str.is_empty() {
                    Err(format!("The path {:?} does not exist. There are no similar names either.", f_path))
                } else {
                    Err(format!("The path {:?} does not exist.\n\nThere are paths with similar names however:\n{}", f_path, similar_paths_str))
                }
            } else {
                f_path = projpath_options[0].clone();
                Ok(f_path.to_string_lossy().to_string())
            }
        }
    }

    if candidates.len() > 1 {
        return Err(format!("The path {:?} is ambiguous.\n\nIt could be interpreted as:\n{}", file_path, candidates.join("\n")));
    }
    let candidate = candidates.get(0).unwrap_or(&"".to_string()).clone();
    if !PathBuf::from(&candidate).exists() {
        return Err(format!("The path {:?} was not found on disk.", candidate));
    }
    Ok(candidate)
}

pub fn text_on_clip(result: &ContextFile, from_tool_call: bool) -> String {
    if !from_tool_call {
        return "".to_string();
    }
    return format!("attached file: {}", result.file_name.clone());
}

#[derive(Debug)]
pub struct AtParamFilePath {}

impl AtParamFilePath {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl AtParam for AtParamFilePath {
    async fn is_value_valid(
        &self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        _value: &String,
    ) -> bool {
        return true;
    }

    async fn param_completion(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        value: &String,
    ) -> Vec<String> {
        let (gcx, top_n) = {
            let ccx_lock = ccx.lock().await;
            (ccx_lock.global_context.clone(), ccx_lock.top_n)
        };
        let candidates = file_repair_candidates(gcx.clone(), value, top_n, false).await;
        if !candidates.is_empty() {
            return candidates;
        }
        let file_path = PathBuf::from(value);
        if file_path.is_relative() {
            let project_paths = get_project_paths(gcx.clone()).await;
            let options = project_paths.iter().map(|x|x.join(&file_path)).filter(|x|x.is_file()).collect::<Vec<_>>();
            if !options.is_empty() {
                return options.iter().map(|x| x.to_string_lossy().to_string()).collect();
            }
        }
        return file_repair_candidates(gcx.clone(), value, top_n, true).await;
    }

    fn param_completion_valid(&self) -> bool {true}
}

pub async fn get_project_paths(gcx: Arc<ARwLock<GlobalContext>>) -> Vec<PathBuf> {
    let gcx_locked = gcx.write().await;
    let workspace_folders = gcx_locked.documents_state.workspace_folders.lock().unwrap();
    workspace_folders.iter().cloned().collect::<Vec<_>>()
}

pub async fn context_file_from_file_path(
    ccx: Arc<AMutex<AtCommandsContext>>,
    candidates: Vec<String>,
    file_path: String,
) -> Result<ContextFile, String> {
    let mut file_path_from_c = candidates.get(0).map(|x|x.clone()).unwrap_or(file_path.clone());
    let mut line1 = 0;
    let mut line2 = 0;
    let colon_kind_mb = colon_lines_range_from_arg(&mut file_path_from_c);
    let gradient_type = gradient_type_from_range_kind(&colon_kind_mb);

    let gcx = ccx.lock().await.global_context.clone();
    let file_content = get_file_text_from_memory_or_disk(gcx.clone(), &PathBuf::from(&file_path_from_c)).await?;

    if let Some(colon) = &colon_kind_mb {
        line1 = colon.line1;
        line2 = colon.line2;
    }
    if line1 == 0 && line2 == 0 {
        line2 = file_content.lines().count();
    }

    Ok(ContextFile {
        file_name: file_path_from_c,
        file_content,
        line1,
        line2,
        symbols: vec![],
        gradient_type,
        usefulness: 100.0,
        is_body_important: false
    })
}

pub async fn execute_at_file(
    ccx: Arc<AMutex<AtCommandsContext>>,
    file_path: String,
) -> Result<ContextFile, String>
{
    let (gcx, top_n) = {
        let ccx_lock = ccx.lock().await;
        (ccx_lock.global_context.clone(), ccx_lock.top_n)
    };
    let candidates = file_repair_candidates(gcx.clone(), &file_path, top_n, false).await;
    match context_file_from_file_path(ccx.clone(), candidates, file_path.clone()).await {
        Ok(x) => { return Ok(x) },
        Err(e) => { info!("non-fuzzy at file has failed to get file_path: {:?}", e); }
    }

    let candidates_fuzzy = file_repair_candidates(gcx.clone(), &file_path, top_n, true).await;
    context_file_from_file_path(ccx.clone(), candidates_fuzzy, file_path).await
}

#[async_trait]
impl AtCommand for AtFile {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn at_execute(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        cmd: &mut AtCommandMember,
        args: &mut Vec<AtCommandMember>,
    ) -> Result<(Vec<ContextEnum>, String), String> {
        let mut file_path = match args.get(0) {
            Some(x) => x.clone(),
            None => {
                cmd.ok = false; cmd.reason = Some("missing file path".to_string());
                args.clear();
                return Err("missing file path".to_string());
            }
        };
        correct_at_arg(ccx.clone(), self.params[0].clone(), &mut file_path).await;
        args.clear();
        args.push(file_path.clone());

        if !file_path.ok {
            return Err(format!("file_path is incorrect: {:?}. Reason: {:?}", file_path.text, file_path.reason));
        }

        let context_file = execute_at_file(ccx.clone(), file_path.text.clone()).await?;
        let text = text_on_clip(&context_file, false);
        Ok((vec_context_file_to_context_tools(vec![context_file]), text))
    }
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
            assert_eq!(result, Some(ColonLinesRange { kind: RangeKind::GradToCursorTwoSided, line1: 25, line2: 0 }));
        }
        {
            let mut value = String::from("invalid");
            let result = colon_lines_range_from_arg(&mut value);
            assert_eq!(result, None);
        }
    }
}
