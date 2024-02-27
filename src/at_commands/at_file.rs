use std::sync::Arc;

use async_trait::async_trait;
use regex::Regex;
use serde_json::json;
use tokio::sync::Mutex as AMutex;
use tracing::info;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_params::AtParamFilePath;
use crate::at_commands::utils::split_file_into_chunks_from_line_inside;
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::call_validation::{ChatMessage, ContextFile};

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


struct ColonLinesRange {
    start: i32,
    end: i32,
}

fn colon_lines_range_from_arg(value: &mut String) -> Option<ColonLinesRange> {
    let re = Regex::new(r":(\d+)(?:-(\d+))?$").unwrap();
    if let Some(captures) = re.captures(value) {
        let mut res = ColonLinesRange {start: -1, end: -1};
        if let Some(start) = captures.get(1) {
            res.start = start.as_str().parse::<i32>().unwrap_or(-1);
        }
        if let Some(end) = captures.get(2) {
            res.end = end.as_str().parse::<i32>().unwrap_or(-1);
        }
        *value = re.replace(value, "").to_string();
        return Some(res)
    }
    None
}

fn chunks_into_context_file(
    result_above: Vec<((i32, i32), String)>,
    results_below: Vec<((i32, i32), String)>,
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

#[async_trait]
impl AtCommand for AtFile {
    fn name(&self) -> &String {
        &self.name
    }
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }
    async fn can_execute(&self, args: &Vec<String>, context: &AtCommandsContext) -> bool {
        let param = self.params.get(0).unwrap();
        if let Some(arg) = args.get(0) {
            let mut arg_clone = arg.clone();
            colon_lines_range_from_arg(&mut arg_clone);
            if param.lock().await.is_value_valid(&arg_clone, context).await {
                return true;
            }
        }
        false
    }
    async fn execute(&self, _query: &String, args: &Vec<String>, _top_n: usize, context: &AtCommandsContext) -> Result<ChatMessage, String> {
        let can_execute = self.can_execute(args, context).await;
        if !can_execute {
            return Err("incorrect arguments".to_string());
        }
        let mut file_path = match args.get(0) {
            Some(x) => x.clone(),
            None => return Err("no file path".to_string()),
        };

        let mut split_into_chunks = false;

        let mut colon = match colon_lines_range_from_arg(&mut file_path) {
            Some(c) => {
                if c.end == -1 { split_into_chunks = true; }
                if c.end != -1 && c.start > c.end { return Err("start line must be less than end line".to_string()); }
                c
            },
            None => ColonLinesRange { start: 0, end: 0 },
        };

        let mut file_text = get_file_text_from_memory_or_disk(context.global_context.clone(), &file_path).await?;
        let lines_cnt = file_text.lines().count() as i32;

        if colon.end <= 0 {
            colon.end = lines_cnt;
        }

        colon.start = (colon.start - 1).max(0).min(lines_cnt);
        colon.end = colon.end.max(0).min(lines_cnt);

        if split_into_chunks {
            info!(colon.start);
            let (res_above, res_below) = split_file_into_chunks_from_line_inside(colon.start, &file_text, 20);
            return Ok(ChatMessage {
                role: "context_file".to_string(),
                content: json!(chunks_into_context_file(res_above, res_below, &file_path)).to_string(),
            })
        }

        let lines: Vec<&str> = file_text.lines().collect();
        file_text = lines[colon.start as usize..colon.end as usize].join("\n");

        let mut vector_of_context_file: Vec<ContextFile> = vec![];
        vector_of_context_file.push(ContextFile {
            file_name: file_path.clone(),
            file_content: file_text,
            line1: colon.start,
            line2: colon.end,
            usefulness: 100.0,
        });
        Ok(ChatMessage {
            role: "context_file".to_string(),
            content: json!(vector_of_context_file).to_string(),
        })
    }
}
