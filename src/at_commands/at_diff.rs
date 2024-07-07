use std::sync::Arc;
use std::path::PathBuf;
use tracing::info;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::Mutex as AMutex;
use tokio::process::Command;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_file::AtParamFilePath;
use crate::at_commands::execute_at::AtCommandMember;
use crate::call_validation::{ContextEnum, DiffChunk, ChatMessage};
use crate::files_in_workspace::detect_vcs_in_dir;


pub struct AtDiff {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtDiff {
    pub fn new() -> Self {
        AtDiff { params: vec![] }
    }
}

fn process_diff_line(line: &str, current_chunk: &mut DiffChunk) {
    if line.starts_with('-') {
        current_chunk.lines_remove.push_str(&line[1..]);
        current_chunk.lines_remove.push('\n');
    } else if line.starts_with('+') {
        current_chunk.lines_add.push_str(&line[1..]);
        current_chunk.lines_add.push('\n');
    } else if line.starts_with(' ') {
        current_chunk.lines_remove.push_str(&line[1..]);
        current_chunk.lines_remove.push('\n');
        current_chunk.lines_add.push_str(&line[1..]);
        current_chunk.lines_add.push('\n');
    }
}

fn process_diff_stdout(stdout: &str) -> Vec<DiffChunk> {
    let mut diff_chunks = Vec::new();
    let mut current_chunk = DiffChunk::default();
    let mut file_name = String::new();
    let mut in_diff_block = false;

    for line in stdout.lines() {
        if line.starts_with("diff --git") || line.starts_with("Index:") || line.starts_with("diff -r") {
            file_name = line.split_whitespace().last().unwrap_or("").to_string();
            if in_diff_block {
                diff_chunks.push(current_chunk);
            }
            current_chunk = DiffChunk {
                file_name: file_name.clone(),
                file_action: "edit".to_string(),
                ..Default::default()
            };
            in_diff_block = true;
        } else if line.starts_with("@@") {
            if !current_chunk.lines_remove.is_empty() || !current_chunk.lines_add.is_empty() {
                current_chunk.lines_add = current_chunk.lines_add.trim_end_matches('\n').to_string();
                current_chunk.lines_remove = current_chunk.lines_remove.trim_end_matches('\n').to_string();
                diff_chunks.push(current_chunk);
                current_chunk = DiffChunk {
                    file_name: file_name.clone(),
                    file_action: "edit".to_string(),
                    ..Default::default()
                };
            }
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() > 2 {
                let l1_numbers = parts[1].split(',').collect::<Vec<_>>();
                let l2_numbers = parts[2].split(',').collect::<Vec<_>>();
                if !l1_numbers.is_empty() && l2_numbers.len() > 1 {
                    current_chunk.line1 = l1_numbers[0].trim_start_matches('-').parse().unwrap_or(0);
                    current_chunk.line2 = current_chunk.line1 + l2_numbers[1].trim_start_matches('+').trim_start_matches(',').parse().unwrap_or(0);
                }
            }
        }
        process_diff_line(line, &mut current_chunk);
    }
    if in_diff_block && (!current_chunk.lines_remove.is_empty() || !current_chunk.lines_add.is_empty()) {
        diff_chunks.push(current_chunk);
    }
    diff_chunks
}

async fn execute_diff(vcs: &str, parent_dir: &str, args: &[&str]) -> Result<Vec<DiffChunk>, String> {
    let output = Command::new(vcs)
        .arg("diff")
        .args(args)
        .current_dir(PathBuf::from(parent_dir))
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !stderr.is_empty() {
        return Err(stderr);
    }
    Ok(process_diff_stdout(&stdout))
}

async fn execute_git_diff(parent_dir: &str, args: &[&str]) -> Result<Vec<DiffChunk>, String> {
    execute_diff("git", parent_dir, args).await
}

async fn execute_svn_diff(parent_dir: &str, args: &[&str]) -> Result<Vec<DiffChunk>, String> {
    execute_diff("svn", parent_dir, args).await
}

async fn execute_hg_diff(parent_dir: &str, args: &[&str]) -> Result<Vec<DiffChunk>, String> {
    execute_diff("hg", parent_dir, args).await
}

pub async fn execute_diff_for_vcs(parent_dir: &str, args: &[&str]) -> Result<Vec<DiffChunk>, String> {
    if let Some(res) = detect_vcs_in_dir(&PathBuf::from(parent_dir)).await {
        match res {
            "git" => execute_git_diff(parent_dir, args).await,
            "svn" => execute_svn_diff(parent_dir, args).await,
            "hg" => execute_hg_diff(parent_dir, args).await,
            _ => Err("No VCS found".to_string())
        }
    } else {
        return Err("No VCS found".to_string())
    }
}

pub fn text_on_clip(args: &Vec<AtCommandMember>) -> String {
    let text = match args.len() { 
        0 => "executed: git diff".to_string(),
        1 => format!("executed: git diff {}", args[0].text),
        _ => "".to_string(),
    };
    text
}

pub async fn get_last_accessed_file(ccx: &mut AtCommandsContext) -> Result<PathBuf, String> {
    return match ccx.global_context.read().await.documents_state.last_accessed_file.lock().unwrap().clone() {
        Some(file) => Ok(file),
        // TODO: improve error text?
        None => Err("Couldn't find last used file. Try again later".to_string())
    }
}

async fn validate_and_complete_file_path(ccx: &mut AtCommandsContext, file_path: &PathBuf) -> Result<PathBuf, String>{
    if !file_path.is_file() {
        return match AtParamFilePath::new().param_completion(&file_path.to_string_lossy().to_string(), ccx).await.get(0) {
            Some(candidate) => Ok(PathBuf::from(candidate)),
            None => return Err(format!("File {:?} doesn't exist and wasn't found in index", file_path)),
        }
    }
    Ok(file_path.clone())
}

#[async_trait]
impl AtCommand for AtDiff {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn execute(&self, ccx: &mut AtCommandsContext, cmd: &mut AtCommandMember, args: &mut Vec<AtCommandMember>) -> Result<(Vec<ContextEnum>, String), String> {
        let diff_chunks = match args.iter().take_while(|arg| arg.text != "\n").take(2).count() {
            0 => {
                // No arguments: git diff for all tracked files
                let last_accessed_file = get_last_accessed_file(ccx).await?;
                let parent_dir = last_accessed_file.parent().unwrap().to_string_lossy().to_string();
                args.clear();
                execute_diff_for_vcs(&parent_dir, &[]).await.map_err(|e|format!("Couldn't execute git diff.\nError: {}", e))
            },
            1 => {
                // TODO: if file_path is rel, complete it
                // 1 argument: git diff for a specific file
                args.truncate(1);
                
                let file_path = match validate_and_complete_file_path(ccx, &PathBuf::from(cmd.text.as_str())).await {
                    Ok(file_path) => file_path,
                    Err(e) => {
                        cmd.ok = false; cmd.reason = Some(e.clone());
                        args.clear();
                        return Err(e);
                    }
                };
                
                let parent_dir = file_path.parent().unwrap().to_string_lossy().to_string();
                execute_diff_for_vcs(&parent_dir, &[&file_path.to_string_lossy().to_string()]).await.map_err(|e|format!("Couldn't execute git diff {:?}.\nError: {}", file_path, e))
            },
            _ => {
                cmd.ok = false; cmd.reason = Some("Invalid number of arguments".to_string());
                args.clear();
                return Err("Invalid number of arguments".to_string()); 
            },
        }?;

        info!("executed @diff {:?}", args.iter().map(|x|x.text.clone()).collect::<Vec<_>>().join(" "));
        
        let message = ChatMessage::new(
            "diff".to_string(),
            json!(diff_chunks).to_string(),
        );
        Ok((vec![ContextEnum::ChatMessage(message)], text_on_clip(args)))
    }

    fn depends_on(&self) -> Vec<String> {
        vec![]
    }
}

async fn execute_diff_with_revs(parent_dir: &PathBuf, rev1: &str, rev2: &str, file_path: &PathBuf) -> Result<Vec<DiffChunk>, String> {
    let mut command = match detect_vcs_in_dir(parent_dir).await {
        Some("git") => {
            let mut cmd = Command::new("git");
            cmd.arg("diff").arg(format!("{}..{}", rev1, rev2));
            cmd
        },
        Some("hg") => {
            let mut cmd = Command::new("hg");
            cmd.arg("diff").arg("-r").arg(rev1).arg("-r").arg(rev2);
            cmd
        },
        Some("svn") => {
            let mut cmd = Command::new("svn");
            cmd.arg("diff").arg("-r").arg(format!("{}:{}", rev1, rev2));
            cmd
        },
        _ => { return Err("Unknown or missing VCS".to_string()); }
    };

    command.arg("--").arg(file_path);

    let output = command
        .current_dir(PathBuf::from(parent_dir))
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !stderr.is_empty() {
        return Err(stderr);
    }
    Ok(process_diff_stdout(&stdout))
}

pub struct AtDiffRev {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtDiffRev {
    pub fn new() -> Self {
        AtDiffRev { params: vec![] }
    }
}

#[async_trait]
impl AtCommand for AtDiffRev {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn execute(&self, ccx: &mut AtCommandsContext, cmd: &mut AtCommandMember, args: &mut Vec<AtCommandMember>) -> Result<(Vec<ContextEnum>, String), String> {
        if args.len() < 3 {
            cmd.ok = false; cmd.reason = Some("Invalid number of arguments".to_string());
            args.clear();
            return Err("Invalid number of arguments".to_string());
        }
        
        let rev1 = args[0].clone();
        let rev2 = args[1].clone();

        let file_path = match validate_and_complete_file_path(ccx, &PathBuf::from(cmd.text.as_str())).await {
            Ok(file_path) => file_path,
            Err(e) => {
                cmd.ok = false; cmd.reason = Some(e.clone());
                args.clear();
                return Err(e);
            }
        };
        let parent_path = PathBuf::from(file_path.parent().unwrap());

        args.truncate(3);
        
        let diff_chunks = execute_diff_with_revs(&parent_path, &rev1.text, &rev2.text, &file_path).await?;
        
        let message = ChatMessage::new(
            "diff".to_string(),
            json!(diff_chunks).to_string(),
        );

        info!("executed @diff-rev {} {} {:?}", rev1.text, rev2.text, file_path);
        Ok((vec![ContextEnum::ChatMessage(message)], text_on_clip(args)))
    }

    fn depends_on(&self) -> Vec<String> {
        vec![]
    }
}
