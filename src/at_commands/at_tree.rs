use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{Mutex as AMutex, MutexGuard};
use tokio::sync::RwLock as ARwLock;
use tracing::info;

use crate::ast::ast_index::{AstIndex, RequestSymbolType};
use crate::ast::treesitter::structs::SymbolType;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_file::{at_file_repair_candidates, AtParamFilePath};
use crate::at_commands::execute_at::{AtCommandMember, correct_at_arg};
use crate::call_validation::{ChatMessage, ContextEnum};
use crate::files_correction::{canonical_path, paths_from_anywhere};
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;

pub static CONTEXT_SIZE_LIMIT: usize = 4096;
pub static SYMBOLS_PER_TOKEN: f32 = 3.5;
pub static RETRIEVE_SYMBOLS: bool = true;


pub struct AtTree {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtTree {
    pub fn new() -> Self {
        AtTree {
            params: vec![
                Arc::new(AMutex::new(AtParamFilePath::new()))
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct PathsHolderNodeRef(Arc<RwLock<PathsHolderNode>>);

impl PartialEq for PathsHolderNodeRef {
    fn eq(&self, other: &Self) -> bool {
        self.0.read().unwrap().path == other.0.read().unwrap().path
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathsHolderNode {
    path: PathBuf,
    child_paths: Vec<PathsHolderNodeRef>,
}

impl PathsHolderNode {
    pub fn file_name(&self) -> String {
        self.path.file_name().unwrap_or_default().to_string_lossy().to_string()
    }
}

pub fn make_files_tree_by_paths_from_anywhere(paths_from_anywhere: &Vec<PathBuf>) -> Vec<PathsHolderNodeRef> {
    let mut root_nodes: Vec<PathsHolderNodeRef> = Vec::new();
    let mut nodes_map: HashMap<PathBuf, PathsHolderNodeRef> = HashMap::new();

    for path in paths_from_anywhere {
        let mut current_path = PathBuf::new();
        let mut parent_node: Option<PathsHolderNodeRef> = None;

        for component in path.components() {
            current_path.push(component);

            let node = nodes_map.entry(current_path.clone()).or_insert_with(|| {
                PathsHolderNodeRef(Arc::new(RwLock::new(
                    PathsHolderNode {
                        path: current_path.clone(),
                        child_paths: Vec::new(),
                    }
                )))
            });

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


pub fn print_files_tree_with_budget_internal(
    tree: Vec<PathsHolderNodeRef>,
    budget: usize,
    maybe_ast_module: Option<MutexGuard<AstIndex>>
) -> Result<String, String> {
    #[derive(Debug, Clone, Eq, PartialEq)]
    struct PathInfoNode {
        filename: String,
        symbols: String,
        child_paths: Vec<Arc<RefCell<PathInfoNode>>>,
        is_complete: bool,
    }

    fn recursive_print_path_holders(
        tree_str: &mut String,
        prefix: &String,
        paths_holder: Arc<RefCell<PathInfoNode>>,
        is_last: bool
    ) {
        let more = if is_last { "└─ " } else { "├─ " };
        tree_str.push_str(&format!("{}{}{}{}\n", prefix, more, paths_holder.borrow().filename, paths_holder.borrow().symbols));
        let new_prefix = if is_last { prefix.to_owned() + "  " } else { prefix.to_owned() + "│ " };
        for (idx, sub_path) in paths_holder.borrow().child_paths.iter().enumerate() {
            let is_last = idx == paths_holder.borrow().child_paths.len() - 1 && paths_holder.borrow().is_complete;
            recursive_print_path_holders(tree_str, &new_prefix, sub_path.clone(), is_last);
        }
        if !paths_holder.borrow().is_complete {
            recursive_print_path_holders(
                tree_str, &new_prefix, Arc::new(RefCell::new(
                    PathInfoNode { filename: "...".to_string(), symbols: "".to_string(), child_paths: vec![], is_complete: true }
                )), true);
        }
    }

    let mut queue: VecDeque<(PathsHolderNodeRef, Arc<RefCell<PathInfoNode>>)> = VecDeque::new();
    let mut collected_paths: Vec<Arc<RefCell<PathInfoNode>>> = Vec::new();
    for node in tree.iter() {
        let paths_holder = Arc::new(RefCell::new(
            PathInfoNode { filename: node.0.read().unwrap().file_name(), symbols: "".to_string(), child_paths: vec![], is_complete: true }
        ));
        queue.push_back((node.clone(), paths_holder.clone()));
        collected_paths.push(paths_holder);
    }


    // First stage: Collect paths within the budget
    let mut total_symbols = 0;
    while let Some((node, paths_holder)) = queue.pop_front() {
        let mut node_entries = node.0.read().unwrap().child_paths.clone();
        node_entries.sort_by_key(|dir| dir.0.read().unwrap().path.clone());

        for entry in node_entries.iter() {
            let ast_symbols = match &maybe_ast_module {
                Some(ast) => {
                    let doc = Document { path: entry.0.read().unwrap().path.clone(), text: None };
                    match ast.get_by_file_path(RequestSymbolType::Declaration, &doc) {
                        Ok(symbols) => {
                            let symbols_list = symbols
                                .iter()
                                .filter(|x| x.symbol_type == SymbolType::StructDeclaration
                                    || x.symbol_type == SymbolType::FunctionDeclaration)
                                .filter(|x| !x.name.is_empty())
                                .map(|x| x.name.clone())
                                .collect::<Vec<String>>()
                                .join(", ");
                            if !symbols_list.is_empty() { format!(" ({symbols_list})") } else { "".to_string() }
                        }

                        Err(_) => "".to_string()
                    }
                }
                None => "".to_string()
            };
            let filename = entry.0.read().unwrap().file_name();
            total_symbols += filename.len() + ast_symbols.len() + 5;  // 5 is a small budget for special symbols
            if total_symbols >= budget {
                paths_holder.borrow_mut().is_complete = false;
                break;
            }
            let sub_path_holder = Arc::new(RefCell::new(
                PathInfoNode { filename, symbols: ast_symbols, child_paths: vec![], is_complete: true }
            ));
            paths_holder.borrow_mut().child_paths.push(sub_path_holder.clone());
            queue.push_back((entry.clone(), sub_path_holder));
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

pub async fn print_files_tree_with_budget(
    gcx: Arc<ARwLock<GlobalContext>>,
    tree: Vec<PathsHolderNodeRef>,
) -> Result<String, String> {
    let context_limit = CONTEXT_SIZE_LIMIT * SYMBOLS_PER_TOKEN as usize;
    return if RETRIEVE_SYMBOLS {
        let maybe_ast_module = gcx.read().await.ast_module.clone();
        if let Some(ast_module) = maybe_ast_module {
            let ast_module = ast_module.read().await;
            let maybe_ast_index = ast_module.read_ast(Duration::from_millis(25)).await;
            if let Ok(ast_index) = maybe_ast_index {
                print_files_tree_with_budget_internal(tree, context_limit, Some(ast_index))
            } else { print_files_tree_with_budget_internal(tree, context_limit, None) }
        } else { print_files_tree_with_budget_internal(tree, context_limit, None) }
    } else { print_files_tree_with_budget_internal(tree, context_limit, None) };
}


#[async_trait]
impl AtCommand for AtTree {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> { &self.params }
    async fn execute(&self, ccx: &mut AtCommandsContext, cmd: &mut AtCommandMember, args: &mut Vec<AtCommandMember>) -> Result<(Vec<ContextEnum>, String), String> {
        let paths_from_anywhere = paths_from_anywhere(ccx.global_context.clone()).await;
        let all_args_are_empty = args.iter().all(|x| x.text.is_empty());
        let tree = if args.is_empty() || all_args_are_empty {
            make_files_tree_by_paths_from_anywhere(&paths_from_anywhere)
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
            let candidates = at_file_repair_candidates(&file_path.text, ccx, false).await;
            if candidates.is_empty() {
                info!("parameter {:?} is uncorrectable :/", &file_path);
                return Err(format!("parameter {:?} is uncorrectable :/", &file_path));
            }
            let base_path = canonical_path(&candidates.get(0).unwrap().clone());
            let filtered_paths_from_anywhere = paths_from_anywhere
                .iter()
                .filter(|file| file.starts_with(&base_path))
                .cloned()
                .collect();
            make_files_tree_by_paths_from_anywhere(&filtered_paths_from_anywhere)
        };

        let context = match print_files_tree_with_budget(
            ccx.global_context.clone(), tree
        ).await {
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
