use std::path::PathBuf;
use async_trait::async_trait;
use regex::Regex;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tracing::info;
use std::sync::Arc;
use uuid::Uuid;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam, vec_context_file_to_context_tools};
use crate::at_commands::execute_at::{AtCommandMember, correct_at_arg};
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::call_validation::{ContextFile, ContextEnum};
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
    value: &String, 
    gcx: Arc<ARwLock<GlobalContext>>, 
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

pub async fn at_file_repair_candidates(
    value: &String,
    ccx: &AtCommandsContext,
    fuzzy: bool,
) -> Vec<String> {
    file_repair_candidates(value, ccx.global_context.clone(), ccx.top_n, fuzzy).await
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
    async fn is_value_valid(&self, _value: &String, _ccx: &AtCommandsContext) -> bool {
        return true;
    }

    async fn param_completion(&self, value: &String, ccx: &AtCommandsContext) -> Vec<String> {
        let candidates =  at_file_repair_candidates(value, ccx, false).await;
        if !candidates.is_empty() {
            return candidates;
        }
        let file_path = PathBuf::from(value);
        if file_path.is_relative() {
            let project_paths = get_project_paths(ccx).await;
            let options = project_paths.iter().map(|x|x.join(&file_path)).filter(|x|x.is_file()).collect::<Vec<_>>();
            if !options.is_empty() {
                return options.iter().map(|x| x.to_string_lossy().to_string()).collect();
            }
        }
        return at_file_repair_candidates(value, ccx, true).await;
    }

    fn param_completion_valid(&self) -> bool {true}
}

pub async fn get_project_paths(ccx: &AtCommandsContext) -> Vec<PathBuf> {
    let cx = ccx.global_context.read().await;
    let workspace_folders = cx.documents_state.workspace_folders.lock().unwrap();
    workspace_folders.iter().cloned().collect::<Vec<_>>()
}

pub async fn context_file_from_file_path(
    ccx: &mut AtCommandsContext,
    candidates: Vec<String>,
    file_path: String,
) -> Result<ContextFile, String> {
    let mut file_path_from_c = candidates.get(0).map(|x|x.clone()).unwrap_or(file_path.clone());
    let mut line1 = 0;
    let mut line2 = 0;
    let colon_kind_mb = colon_lines_range_from_arg(&mut file_path_from_c);
    let gradient_type = gradient_type_from_range_kind(&colon_kind_mb);

    let file_content = get_file_text_from_memory_or_disk(ccx.global_context.clone(), &PathBuf::from(&file_path_from_c)).await?;

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
        symbol: Uuid::default(),
        gradient_type,
        usefulness: 100.0,
        is_body_important: false
    })
}

pub async fn execute_at_file(ccx: &mut AtCommandsContext, file_path: String) -> Result<ContextFile, String> {
    let candidates = at_file_repair_candidates(&file_path, ccx, false).await;
    
    match context_file_from_file_path(ccx, candidates, file_path.clone()).await {
        Ok(x) => { return Ok(x) },
        Err(e) => { info!("non-fuzzy at file has failed to get file_path: {:?}", e); }
    }

    let candidates_fuzzy = at_file_repair_candidates(&file_path, ccx, true).await;
    context_file_from_file_path(ccx, candidates_fuzzy, file_path).await
}

#[async_trait]
impl AtCommand for AtFile {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }
    async fn execute(&self, ccx: &mut AtCommandsContext, cmd: &mut AtCommandMember, args: &mut Vec<AtCommandMember>) -> Result<(Vec<ContextEnum>, String), String> {
        let mut file_path = match args.get(0) {
            Some(x) => x.clone(),
            None => {
                cmd.ok = false; cmd.reason = Some("missing file path".to_string());
                args.clear();
                return Err("missing file path".to_string());
            }
        };
        correct_at_arg(ccx, self.params[0].clone(), &mut file_path).await;
        args.clear();
        args.push(file_path.clone());

        if !file_path.ok {
            return Err(format!("file_path is incorrect: {:?}. Reason: {:?}", file_path.text, file_path.reason));
        }

        let context_file = execute_at_file(ccx, file_path.text.clone()).await?;
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
