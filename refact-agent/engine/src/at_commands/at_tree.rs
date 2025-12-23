use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use tracing::warn;

use crate::ast::ast_structs::{AstDB, SymbolType};
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_file::return_one_candidate_or_a_good_error;
use crate::at_commands::execute_at::AtCommandMember;
use crate::call_validation::{ChatMessage, ContextEnum};
use crate::files_correction::{correct_to_nearest_dir_path, get_project_dirs, paths_from_anywhere};


pub struct AtTree {
    pub params: Vec<Box<dyn AtParam>>,
}

impl AtTree {
    pub fn new() -> Self {
        AtTree {
            params: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct PathsHolderNodeArc(Arc<RwLock<PathsHolderNode>>);

impl PartialEq for PathsHolderNodeArc {
    fn eq(&self, other: &Self) -> bool {
        self.0.read().unwrap().path == other.0.read().unwrap().path
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathsHolderNode {
    path: PathBuf,
    is_dir: bool,
    child_paths: Vec<PathsHolderNodeArc>,
    depth: usize,
}

pub struct TreeNode {
    pub children: HashMap<String, TreeNode>,
    // NOTE: we can store here more info like depth, sub files count, etc.
}

impl TreeNode {
    pub fn new() -> Self {
        TreeNode {
            children: HashMap::new(),
        }
    }

    pub fn build(paths: &Vec<PathBuf>) -> Self {
        let mut root = TreeNode::new();
        for path in paths {
            let mut node = &mut root;
            for component in path.components() {
                let key = component.as_os_str().to_string_lossy().to_string();
                node = node.children.entry(key).or_insert_with(TreeNode::new);
            }
        }
        root
    }

    pub fn is_dir(&self) -> bool {
        !self.children.is_empty()
    }
}

fn _print_symbols(db: Arc<AstDB>, path: &PathBuf) -> String {
    let cpath = path.to_string_lossy().to_string();
    let defs = crate::ast::ast_db::doc_defs(db.clone(), &cpath);
    let symbols_list = defs
        .iter()
        .filter(|x| match x.symbol_type {
            SymbolType::StructDeclaration | SymbolType::TypeAlias | SymbolType::FunctionDeclaration => true,
            _ => false
        })
        .map(|x| x.name())
        .collect::<Vec<String>>()
        .join(", ");
    if !symbols_list.is_empty() { format!(" ({symbols_list})") } else { "".to_string() }
}

async fn _print_files_tree(
    tree: &TreeNode,
    ast_db: Option<Arc<AstDB>>,
    maxdepth: usize,
) -> String {
    fn traverse(
        node: &TreeNode,
        path: PathBuf,
        depth: usize,
        maxdepth: usize,
        ast_db: Option<Arc<AstDB>>,
    ) -> Option<String> {
        if depth > maxdepth {
            return None;
        }
        let mut output = String::new();
        let indent = "  ".repeat(depth);
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        if !node.is_dir() {
            if let Some(db) = ast_db.clone() {
                output.push_str(&format!("{}{}{}\n", indent, name, _print_symbols(db, &path)));
            } else {
                output.push_str(&format!("{}{}\n", indent, name));
            }
            return Some(output);
        } else {
            output.push_str(&format!("{}{}/\n", indent, name));
        }

        let (mut dirs, mut files) = (0, 0);
        let mut child_output = String::new();
        for (name, child) in &node.children {
            let mut child_path = path.clone();
            child_path.push(name);
            if let Some(child_str) = traverse(child, child_path, depth + 1, maxdepth, ast_db.clone()) {
                child_output.push_str(&child_str);
            } else {
                dirs += child.is_dir() as usize;
                files += !child.is_dir() as usize;
            }
        }

        if dirs > 0 || files > 0 {
            let summary = format!("{}  ...{} subdirs, {} files...\n", indent, dirs, files);
            child_output.push_str(&summary);
        }
        output.push_str(&child_output);
        Some(output)
    }

    let mut result = String::new();
    for (name, node) in &tree.children {
        if let Some(output) = traverse(node, PathBuf::from(name), 0, maxdepth, ast_db.clone()) {
            result.push_str(&output);
        } else {
            break;
        }
    }
    result
}

async fn _print_files_tree_with_budget(
    tree: &TreeNode,
    char_limit: usize,
    ast_db: Option<Arc<AstDB>>,
) -> String {
    let mut good_enough = String::new();
    for maxdepth in 1..20 {
        let bigger_tree_str = _print_files_tree(&tree, ast_db.clone(), maxdepth).await;
        if bigger_tree_str.len() > char_limit {
            break;
        }
        good_enough = bigger_tree_str;
    }
    good_enough
}

pub async fn print_files_tree_with_budget(
    ccx: Arc<AMutex<AtCommandsContext>>,
    tree: &TreeNode,
    use_ast: bool,
) -> Result<String, String> {
    let (gcx, tokens_for_rag) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.global_context.clone(), ccx_locked.tokens_for_rag)
    };
    tracing::info!("tree() tokens_for_rag={}", tokens_for_rag);
    const SYMBOLS_PER_TOKEN: f32 = 3.5;
    let char_limit = tokens_for_rag * SYMBOLS_PER_TOKEN as usize;
    let mut ast_module_option = gcx.read().await.ast_service.clone();
    if !use_ast {
        ast_module_option = None;
    }
    match ast_module_option {
        Some(ast_module) => {
            crate::ast::ast_indexer_thread::ast_indexer_block_until_finished(ast_module.clone(), 20_000, true).await;
            let ast_db: Option<Arc<AstDB>> = Some(ast_module.lock().await.ast_index.clone());
            Ok(_print_files_tree_with_budget(tree, char_limit, ast_db.clone()).await)
        }
        None => Ok(_print_files_tree_with_budget(tree, char_limit, None).await),
    }
}


#[async_trait]
impl AtCommand for AtTree {
    fn params(&self) -> &Vec<Box<dyn AtParam>> { &self.params }

    async fn at_execute(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        cmd: &mut AtCommandMember,
        args: &mut Vec<AtCommandMember>,
    ) -> Result<(Vec<ContextEnum>, String), String> {
        let gcx = ccx.lock().await.global_context.clone();
        let paths_from_anywhere = paths_from_anywhere(gcx.clone()).await;
        let paths_from_anywhere_len = paths_from_anywhere.len();

        let project_dirs = get_project_dirs(gcx.clone()).await;
        let filtered_paths: Vec<PathBuf> = paths_from_anywhere.into_iter()
            .filter(|path| project_dirs.iter().any(|project_dir| path.starts_with(project_dir)))
            .collect();
        tracing::info!("tree: project_dirs={:?} file paths {} filtered project dirs only => {} paths", project_dirs, paths_from_anywhere_len, filtered_paths.len());

        *args = args.iter().take_while(|arg| arg.text != "\n" || arg.text == "--ast").take(2).cloned().collect();

        let tree = match args.iter().find(|x| x.text != "--ast") {
            None => TreeNode::build(&filtered_paths),
            Some(arg) => {
                let path = arg.text.clone();
                let candidates = correct_to_nearest_dir_path(gcx.clone(), &path, false, 10).await;
                let candidate = return_one_candidate_or_a_good_error(gcx.clone(), &path, &candidates, &project_dirs, true).await.map_err(|e| {
                    cmd.ok = false;
                    cmd.reason = Some(e.clone());
                    args.clear();
                    e
                })?;
                let start_dir = PathBuf::from(candidate);
                let paths_start_with_start_dir = filtered_paths.iter()
                    .filter(|f|f.starts_with(&start_dir)).cloned().collect::<Vec<_>>();
                TreeNode::build(&paths_start_with_start_dir)
            }
        };

        let use_ast = args.iter().any(|x| x.text == "--ast");
        let tree = print_files_tree_with_budget(ccx.clone(), &tree, use_ast).await.map_err(|err| {
            warn!("{}", err);
            err
        })?;
        let tree = if tree.is_empty() {
            "tree(): directory is empty".to_string()
        } else {
            tree
        };

        let context = ContextEnum::ChatMessage(ChatMessage::new(
            "plain_text".to_string(),
            tree,
        ));
        Ok((vec![context], "".to_string()))
    }
}
