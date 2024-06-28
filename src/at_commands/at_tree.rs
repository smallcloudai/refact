use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs::read_dir;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use tracing::info;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_file::{parameter_repair_candidates};
use crate::at_commands::execute_at::{AtCommandMember, correct_at_arg};
use crate::call_validation::{ChatMessage, ContextEnum};
use crate::files_correction::paths_from_anywhere;

pub static CONTEXT_SIZE_LIMIT: usize = 4096;
pub static SYMBOLS_PER_TOKEN: f32 = 3.5;


pub struct AtTree {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtTree {
    pub fn new() -> Self {
        AtTree {
            params: vec![],
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct PathsHolder {
    filename: String,
    child_paths: Vec<Arc<RefCell<PathsHolder>>>,
    is_complete: bool,
}

pub fn tree_from_path(paths: &Vec<PathBuf>, budget: usize) -> Result<String, String> {
    fn path_to_filename(path: &PathBuf) -> String {
        path.file_name().unwrap_or_default().to_string_lossy().to_string()
    }

    fn recursive_print_path_holders(
        tree_str: &mut String,
        prefix: &String,
        paths_holder: Arc<RefCell<PathsHolder>>,
        is_last: bool,
    ) {
        let more = if is_last { "└─ " } else { "├─ " };
        tree_str.push_str(&format!("{}{}{}\n", prefix, more, paths_holder.borrow().filename));
        let new_prefix = if is_last { prefix.to_owned() + "  " } else { prefix.to_owned() + "│ " };
        for (idx, sub_path) in paths_holder.borrow().child_paths.iter().enumerate() {
            let is_last = idx == paths_holder.borrow().child_paths.len() - 1 && paths_holder.borrow().is_complete;
            recursive_print_path_holders(tree_str, &new_prefix, sub_path.clone(), is_last);
        }
        if !paths_holder.borrow().is_complete {
            recursive_print_path_holders(
                tree_str, &new_prefix, Arc::new(RefCell::new(
                    PathsHolder { filename: "...".to_string(), child_paths: vec![], is_complete: true }
                )), true);
        }
    }

    let mut queue: VecDeque<(PathBuf, Arc<RefCell<PathsHolder>>)> = VecDeque::new();
    let mut collected_paths: Vec<Arc<RefCell<PathsHolder>>> = Vec::new();
    for path in paths.iter() {
        let paths_holder = Arc::new(RefCell::new(
            PathsHolder { filename: path.to_string_lossy().to_string(), child_paths: vec![], is_complete: true }
        ));
        queue.push_back((path.clone(), paths_holder.clone()));
        collected_paths.push(paths_holder);
    }

    // First stage: Collect paths within the budget
    let mut total_symbols = 0;
    while let Some((path, paths_holder)) = queue.pop_front() {
        if path.is_dir() {
            let entries = read_dir(&path).map_err(|e| e.to_string())?;
            let mut dir_entries = Vec::new();
            for entry in entries {
                let entry = entry.map_err(|e| e.to_string())?;
                dir_entries.push(entry);
            }
            dir_entries.sort_by_key(|dir| dir.path());

            for entry in dir_entries.iter() {
                let name = entry.file_name().to_string_lossy().to_string();
                total_symbols += name.len() + 5;  // 5 is a small budget for special symbols

                if total_symbols >= budget {
                    paths_holder.borrow_mut().is_complete = false;
                    break;
                }

                let sub_path_holder = Arc::new(RefCell::new(
                    PathsHolder { filename: path_to_filename(&entry.path()), child_paths: vec![], is_complete: true }
                ));
                paths_holder.borrow_mut().child_paths.push(sub_path_holder.clone());
                queue.push_back((entry.path(), sub_path_holder));
            }
        }
    }

    // Second stage: Format the collected paths into the final output string
    let mut tree_str = String::new();
    for (idx, paths_holder) in collected_paths.iter().enumerate() {
        let is_last = idx == collected_paths.len() - 1 && paths_holder.borrow().is_complete;
        recursive_print_path_holders(&mut tree_str, &"".to_string(), paths_holder.clone(), is_last);
    }

    Ok(tree_str)
}


#[async_trait]
impl AtCommand for AtTree {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> { &self.params }
    async fn execute(&self, ccx: &mut AtCommandsContext, cmd: &mut AtCommandMember, args: &mut Vec<AtCommandMember>) -> Result<(Vec<ContextEnum>, String), String> {
        let paths = if args.is_empty() {
            paths_from_anywhere(ccx.global_context.clone()).await
        } else {
            let mut file_path = match args.get(0) {
                Some(x) => x.clone(),
                None => {
                    cmd.ok = false;
                    cmd.reason = Some("missing file path".to_string());
                    args.clear();
                    return Err("missing file path".to_string());
                }
            };
            correct_at_arg(ccx, self.params[0].clone(), &mut file_path).await;
            args.clear();
            let candidates = parameter_repair_candidates(&file_path.text, ccx).await;
            if candidates.is_empty() {
                info!("parameter {:?} is uncorrectable :/", &file_path);
                return Err(format!("parameter {:?} is uncorrectable :/", &file_path));
            }
            vec![crate::files_correction::canonical_path(&candidates.get(0).unwrap().clone())]
        };
        let context = match tree_from_path(&paths, CONTEXT_SIZE_LIMIT * SYMBOLS_PER_TOKEN as usize) {
            Ok(tree) => {
                ContextEnum::ChatMessage(ChatMessage::new(
                    "context_text".to_string(),
                    tree,
                ))
            }
            Err(err) => {
                info!("{}", err);
                return Err(err);
            }
        };
        Ok((vec![context], "".to_string()))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_from_path_empty_dir() {
        let result = tree_from_path(&vec![
            PathBuf::from("/home/svakhreev/Downloads/helloworld-20240320-111226"),
            PathBuf::from("/home/svakhreev/Downloads/helloworld-20240320-111226"),
        ], 4096).unwrap();
        print!("{}", result);
        assert_eq!(result, "");
    }
}