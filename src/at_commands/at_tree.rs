use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use tokio::sync::{Mutex as AMutex, MutexGuard};
use tracing::warn;

use crate::ast::ast_index::{AstIndex, RequestSymbolType};
use crate::ast::treesitter::structs::SymbolType;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_file::real_file_path_candidate;
use crate::at_commands::execute_at::AtCommandMember;
use crate::call_validation::{ChatMessage, ContextEnum};
use crate::files_correction::{correct_to_nearest_dir_path, get_project_paths, paths_from_anywhere};
use crate::files_in_workspace::Document;


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

impl PathsHolderNode {
    pub fn file_name(&self) -> String {
        self.path.file_name().unwrap_or_default().to_string_lossy().to_string()
    }
}

pub fn construct_tree_out_of_flat_list_of_paths(paths_from_anywhere: &Vec<PathBuf>) -> Vec<PathsHolderNodeArc> {
    let mut root_nodes: Vec<PathsHolderNodeArc> = Vec::new();
    let mut nodes_map: HashMap<PathBuf, PathsHolderNodeArc> = HashMap::new();

    for path in paths_from_anywhere {
        let components: Vec<_> = path.components().collect();
        let components_count = components.len();

        let mut current_path = PathBuf::new();
        let mut parent_node: Option<PathsHolderNodeArc> = None;

        for (index, component) in components.into_iter().enumerate() {
            current_path.push(component);

            let is_last = index == components_count - 1;
            let depth = index;
            let node = nodes_map.entry(current_path.clone()).or_insert_with(|| {
                PathsHolderNodeArc(Arc::new(RwLock::new(
                    PathsHolderNode {
                        path: current_path.clone(),
                        is_dir: !is_last,
                        child_paths: Vec::new(),
                        depth,
                    }
                )))
            });

            if node.0.read().unwrap().depth != depth {
                node.0.write().unwrap().depth = depth;
            }

            if let Some(parent) = parent_node {
                if !parent.0.read().unwrap().child_paths.contains(node) {
                    parent.0.write().unwrap().child_paths.push(node.clone());
                }
            } else {
                if !root_nodes.contains(node) {
                    root_nodes.push(node.clone());
                }
            }

            parent_node = Some(node.clone());
        }
    }
    root_nodes
}

fn _print_symbols(ast_index_maybe: Option<&AstIndex>, entry: &PathsHolderNode) -> String
{
    if let Some(ast) = ast_index_maybe {
        let doc = Document { path: entry.path.clone(), text: None };
        match ast.get_by_file_path(RequestSymbolType::Declaration, &doc) {
            Ok(symbols) => {
                let symbols_list = symbols
                    .iter()
                    .filter(|x| x.symbol_type == SymbolType::StructDeclaration
                        || x.symbol_type == SymbolType::FunctionDeclaration)
                    .filter(|x| !x.name.is_empty() && !x.name.starts_with("anon-"))
                    .map(|x| x.name.clone())
                    .collect::<Vec<String>>()
                    .join(", ");
                if !symbols_list.is_empty() { format!(" ({symbols_list})") } else { "".to_string() }
            }
            Err(_) => "".to_string()
        }
    } else {
        "".to_string()
    }
}

fn _print_files_tree(
    tree: &Vec<PathsHolderNodeArc>,
    ast_index_maybe: Option<&AstIndex>,
    maxdepth: usize,
) -> String {
    fn traverse(node: &PathsHolderNodeArc, depth: usize, maxdepth: usize, ast_index: Option<&AstIndex>) -> Option<String> {
        if depth > maxdepth {
            return None;
        }
        let node: std::sync::RwLockReadGuard<PathsHolderNode> = node.0.read().unwrap();
        let mut output = String::new();
        let indent = "  ".repeat(depth);
        let name = if node.is_dir { format!("{}/", node.file_name()) } else { node.file_name() };
        if !node.is_dir {
            output.push_str(&format!("{}{}{}\n", indent, name, _print_symbols(ast_index, &node)));
            return Some(output);
        }
        output.push_str(&format!("{}{}\n", indent, name));
        let (mut dirs, mut files) = (0, 0);
        let mut child_output = String::new();
        for child in &node.child_paths {
            if let Some(child_str) = traverse(child, depth + 1, maxdepth, ast_index) {
                child_output.push_str(&child_str);
            } else {
                dirs += child.0.read().unwrap().is_dir as usize;
                files += !child.0.read().unwrap().is_dir as usize;
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
    for node in tree {
        if let Some(output) = traverse(&node, 0, maxdepth, ast_index_maybe) {
            result.push_str(&output);
        } else {
            break;
        }
    }
    result
}

fn _print_files_tree_with_budget(
    tree: Vec<PathsHolderNodeArc>,
    char_limit: usize,
    ast_index_maybe: Option<&AstIndex>,
) -> String {
    let mut good_enough = String::new();
    for maxdepth in 1..20 {
        let bigger_tree_str = _print_files_tree(&tree, ast_index_maybe, maxdepth);
        if bigger_tree_str.len() > char_limit {
            break;
        }
        good_enough = bigger_tree_str;
    }
    return good_enough;
}

pub async fn print_files_tree_with_budget(
    ccx: Arc<AMutex<AtCommandsContext>>,
    tree: Vec<PathsHolderNodeArc>,
    use_ast: bool,
) -> Result<String, String> {
    let (gcx, tokens_for_rag) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.global_context.clone(), ccx_locked.tokens_for_rag)
    };
    tracing::info!("tree() tokens_for_rag={}", tokens_for_rag);
    const SYMBOLS_PER_TOKEN: f32 = 3.5;
    let char_limit = tokens_for_rag * SYMBOLS_PER_TOKEN as usize;
    let mut ast_module_option = gcx.read().await.ast_module.clone();
    if !use_ast {
        ast_module_option = None;
    }
    // This function sucks and there's no way to fix it
    match ast_module_option {
        Some(ast_module) => {
            let ast_index: Arc<AMutex<AstIndex>> = ast_module.read().await.ast_index.clone();
            let ast_index_guard: MutexGuard<AstIndex> = ast_index.lock().await;
            Ok(_print_files_tree_with_budget(tree, char_limit, Some(&*ast_index_guard)))
        }
        None => Ok(_print_files_tree_with_budget(tree, char_limit, None)),
    }
}


#[async_trait]
impl AtCommand for AtTree {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> { &self.params }

    async fn at_execute(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        cmd: &mut AtCommandMember,
        args: &mut Vec<AtCommandMember>,
    ) -> Result<(Vec<ContextEnum>, String), String> {
        let gcx = ccx.lock().await.global_context.clone();
        let paths_from_anywhere = paths_from_anywhere(gcx.clone()).await;
        *args = args.iter().take_while(|arg| arg.text != "\n" || arg.text == "--ast").take(2).cloned().collect();

        let tree = match args.iter().find(|x| x.text != "--ast") {
            None => construct_tree_out_of_flat_list_of_paths(&paths_from_anywhere),
            Some(arg) => {
                let path = arg.text.clone();
                let candidates = correct_to_nearest_dir_path(gcx.clone(), &path, false, 10).await;
                let candidate = real_file_path_candidate(gcx.clone(), &path, &candidates, &get_project_paths(gcx.clone()).await, true).await.map_err(|e| {
                    cmd.ok = false; cmd.reason = Some(e.clone()); args.clear();
                    e
                })?;
                let true_path = PathBuf::from(candidate);
                let filtered_paths_from_anywhere = paths_from_anywhere.iter().filter(|f|f.starts_with(&true_path)).cloned().collect::<Vec<_>>();
                construct_tree_out_of_flat_list_of_paths(&filtered_paths_from_anywhere)
            }
        };

        let use_ast = args.iter().any(|x| x.text == "--ast");

        let tree = print_files_tree_with_budget(ccx.clone(), tree, use_ast).await.map_err(|err| {
            warn!("{}", err);
            err
        })?;

        let context = ContextEnum::ChatMessage(ChatMessage::new(
            "plain_text".to_string(),
            tree,
        ));
        Ok((vec![context], "".to_string()))
    }
}
