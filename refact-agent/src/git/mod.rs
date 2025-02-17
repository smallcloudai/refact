pub mod checkpoints;
pub mod commit_info;
pub mod operations;

use serde::{Serialize, Deserialize};
use std::path::PathBuf;

use crate::files_correction::{serialize_path, deserialize_path};

#[derive(Serialize, Deserialize, Debug)]
pub struct CommitInfo {
    pub project_path: url::Url,
    pub commit_message: String,
    pub file_changes: Vec<FileChange>,
}

impl CommitInfo {
    pub fn get_project_name(&self) -> String {
        self.project_path.to_file_path().ok()
            .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "".to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileChange {
    #[serde(serialize_with = "serialize_path", deserialize_with = "deserialize_path")]
    pub relative_path: PathBuf,
    #[serde(serialize_with = "serialize_path", deserialize_with = "deserialize_path")]
    pub absolute_path: PathBuf,
    pub status: FileChangeStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum FileChangeStatus {
    ADDED,
    MODIFIED,
    DELETED,
}

impl FileChangeStatus {
    pub fn initial(&self) -> char {
        match self {
            FileChangeStatus::ADDED => 'A',
            FileChangeStatus::MODIFIED => 'M',
            FileChangeStatus::DELETED => 'D',
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum DiffStatusType {
    IndexToHead,
    WorkdirToIndex,
}

/// It's not equivalent or good match, just best effort so that it works in most cases.
/// Making a 1-to-1 mapping would be very hard.
pub fn from_unix_glob_pattern_to_gitignore(pattern: &str) -> String {
    let parts = pattern.split('/')
        .skip_while(|&p| p.is_empty())
        .map(|part| if part == "*" { "**" } else { part })
        .collect::<Vec<_>>();
    
    if parts.first() != Some(&"**") {
        format!("**/{}", parts.join("/"))
    } else {
        parts.join("/")
    }
}
