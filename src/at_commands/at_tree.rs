use std::collections::HashMap;
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use async_trait::async_trait;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use tokio::sync::Mutex as AMutex;
use tracing::info;

use crate::at_commands::execute_at::AtCommandMember;
use crate::call_validation::{ContextEnum, ChatMessage};
use crate::files_correction::paths_from_anywhere;


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

#[allow(dead_code)]
fn tree_from_path(path: PathBuf, prefix: &str) -> Result<String, String> {
    let mut output = String::new();

    if path.is_dir() {
        let entries = read_dir(&path).map_err(|e| e.to_string())?;
        let mut dir_entries = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            dir_entries.push(entry);
        }

        dir_entries.sort_by_key(|dir| dir.path());

        for (i, entry) in dir_entries.iter().enumerate() {
            let is_last = i == dir_entries.len() - 1;
            let more = if is_last { "└── " } else { "├── " };
            let line = format!("{}{}{}\n", prefix, more, entry.file_name().to_string_lossy());
            output.push_str(&line);

            let new_prefix = if is_last { "    " } else { "│   " };
            let path = entry.path();
            if path.is_dir() {
                output.push_str(&tree_from_path(path, &(prefix.to_owned() + new_prefix))?);
            }
        }
    }
    Ok(output)
}

fn build_tree(paths: Vec<PathBuf>) -> String {
    let mut tree = Tree::new();
    for path in paths {
        tree.insert(&path);
    }
    tree.to_string()
}

struct Tree {
    root: Node,
}

impl Tree {
    fn new() -> Self {
        Tree {
            root: Node::new("".into()),
        }
    }

    fn insert(&mut self, path: &Path) {
        let mut current = &mut self.root;
        for component in path.components() {
            let component_str = component.as_os_str().to_string_lossy().into_owned();
            current = current.children.entry(component_str.clone())
                .or_insert_with(|| Node::new(component_str));
        }
    }

    fn to_string(&self) -> String {
        self.root.to_string(0)
    }
}

struct Node {
    name: String,
    children: HashMap<String, Node>,
}

impl Node {
    fn new(name: String) -> Self {
        Node {
            name,
            children: HashMap::new(),
        }
    }

    fn to_string(&self, level: usize) -> String {
        let mut result = String::new();
        if !self.name.is_empty() {
            result.push_str(&format!("{}{}\n", "  ".repeat(level), self.name));
        }
        let mut children: Vec<_> = self.children.values().collect();
        children.sort_by(|a, b| a.name.cmp(&b.name));
        for child in children {
            result.push_str(&child.to_string(level + 1));
        }
        result
    }
}

#[async_trait]
impl AtCommand for AtTree {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> { &self.params }
    async fn execute(&self, ccx: &mut AtCommandsContext, _cmd: &mut AtCommandMember, args: &mut Vec<AtCommandMember>) -> Result<(Vec<ContextEnum>, String), String> {
        args.clear();

        // let project_paths = ccx.global_context.read().await.documents_state.workspace_folders.lock().unwrap().clone();
        // let p_path = match project_paths.get(0) {
        //     Some(x) => x.clone(),
        //     None => return Err("no workspace folder found".to_string()),
        // };
        // let tree = tree_from_path(PathBuf::from("/Users/valaises/RustroverProjects/refact-lsp"), "")?;
        
        let paths = paths_from_anywhere(ccx.global_context.clone()).await;
        let tree = build_tree(paths);
        
        // info!("tree:\n{}", tree);
        
        let res = ContextEnum::ChatMessage(ChatMessage::new(
            "context_text".to_string(),
            tree,
        ));

        Ok((vec![res], "".to_string()))
    }
}
