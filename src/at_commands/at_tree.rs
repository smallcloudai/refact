use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{Mutex as AMutex, MutexGuard};
use tokio::sync::RwLock as ARwLock;
use tracing::{info, warn};

use crate::ast::ast_index::{AstIndex, RequestSymbolType};
use crate::ast::treesitter::structs::SymbolType;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_file::get_project_paths;
use crate::at_commands::execute_at::AtCommandMember;
use crate::at_tools::att_file::real_file_path_candidate;
use crate::call_validation::{ChatMessage, ContextEnum};
use crate::files_correction::{correct_to_nearest_dir_path, paths_from_anywhere};
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;

pub static CONTEXT_SIZE_LIMIT: usize = 32000;
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
            let node = nodes_map.entry(current_path.clone()).or_insert_with(|| {
                PathsHolderNodeArc(Arc::new(RwLock::new(
                    PathsHolderNode {
                        path: current_path.clone(),
                        is_dir: !is_last,
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
    tree: Vec<PathsHolderNodeArc>,
    budget: usize,
    maybe_ast_module: Option<MutexGuard<AstIndex>>
) -> Result<String, String> {
    #[derive(Debug, Clone, Eq, PartialEq)]
    struct PathInfoNode {
        filename: String,
        symbols: String,
        child_paths: Vec<Arc<RefCell<PathInfoNode>>>,
        is_complete: bool,
        is_directory: bool,
    }

    fn recursive_print_path_holders(
        tree_str: &mut String,
        prefix: &String,
        paths_holder: Arc<RefCell<PathInfoNode>>,
    ) {
        // let more = if is_last { "└─ " } else { "├─ " };
        // let new_prefix = if is_last { prefix.to_owned() + "  " } else { prefix.to_owned() + "│ " };
        let more = "  ";
        let filename = if paths_holder.borrow().is_directory {
            format!("{}/", paths_holder.borrow().filename)  // Add "/" for directories
        } else {
            paths_holder.borrow().filename.clone()
        };
        tree_str.push_str(&format!("{}{}{}{}\n", prefix, more, filename, paths_holder.borrow().symbols));
        let new_prefix = prefix.to_owned() + "  ";
        for (idx, sub_path) in paths_holder.borrow().child_paths.iter().enumerate() {
            let is_last = idx == paths_holder.borrow().child_paths.len() - 1 && paths_holder.borrow().is_complete;
            recursive_print_path_holders(tree_str, &new_prefix, sub_path.clone());
        }
        if !paths_holder.borrow().is_complete {
            recursive_print_path_holders(
                tree_str, &new_prefix, Arc::new(RefCell::new(
                    PathInfoNode { filename: "...".to_string(), symbols: "".to_string(), child_paths: vec![], is_complete: true, is_directory: false }
                )));
        }
    }

    let mut queue: VecDeque<(PathsHolderNodeArc, Arc<RefCell<PathInfoNode>>)> = VecDeque::new();
    let mut collected_paths: Vec<Arc<RefCell<PathInfoNode>>> = Vec::new();
    for node in tree.iter() {
        let paths_holder = Arc::new(RefCell::new(
            PathInfoNode { filename: node.0.read().unwrap().file_name(), symbols: "".to_string(), child_paths: vec![], is_complete: true, is_directory: node.0.read().unwrap().is_dir }
        ));
        queue.push_back((node.clone(), paths_holder.clone()));
        collected_paths.push(paths_holder);
    }


    // First stage: Collect paths within the budget
    let mut budget_exceeded = false;
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
                budget_exceeded = true;
                break;
            }
            let sub_path_holder = Arc::new(RefCell::new(
                PathInfoNode { filename, symbols: ast_symbols, child_paths: vec![], is_complete: true, is_directory: entry.0.read().unwrap().is_dir }
            ));
            paths_holder.borrow_mut().child_paths.push(sub_path_holder.clone());
            queue.push_back((entry.clone(), sub_path_holder));
        }
        if budget_exceeded {
            break;
        }
    }

    // Second stage: Format the collected paths into the final output string
    let mut tree_str = String::new();
    for paths_holder in collected_paths {
        recursive_print_path_holders(&mut tree_str, &"".to_string(), paths_holder.clone());
    }

    Ok(tree_str)
}

pub async fn print_files_tree_with_budget(
    gcx: Arc<ARwLock<GlobalContext>>,
    tree: Vec<PathsHolderNodeArc>,
    use_ast: bool,
) -> Result<String, String> {
    let context_limit = CONTEXT_SIZE_LIMIT * SYMBOLS_PER_TOKEN as usize;

    // retrieve symbols using AST
    if use_ast {
        if let Some(ast_module) = gcx.read().await.ast_module.clone() {
            if let Ok(ast_index) = ast_module.read().await.read_ast(Duration::from_millis(25)).await {
                return print_files_tree_with_budget_internal(tree, context_limit, Some(ast_index));
            }
        }
    }
    print_files_tree_with_budget_internal(tree, context_limit, None)
}


#[async_trait]
impl AtCommand for AtTree {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> { &self.params }
    async fn execute(&self, ccx: &mut AtCommandsContext, cmd: &mut AtCommandMember, args: &mut Vec<AtCommandMember>) -> Result<(Vec<ContextEnum>, String), String> {
        let paths_from_anywhere = paths_from_anywhere(ccx.global_context.clone()).await;
        *args = args.iter().take_while(|arg| arg.text != "\n" || arg.text == "--ast").take(2).cloned().collect();

        let tree = match args.iter().find(|x| x.text != "--ast") {
            None => construct_tree_out_of_flat_list_of_paths(&paths_from_anywhere),
            Some(arg) => {
                let path = arg.text.clone();
                let candidates = correct_to_nearest_dir_path(ccx.global_context.clone(), &path, false, 10).await;
                let candidate = real_file_path_candidate(ccx, &path, &candidates, &get_project_paths(ccx).await, true).await.map_err(|e| {
                    cmd.ok = false; cmd.reason = Some(e.clone()); args.clear();
                    e
                })?;
                let true_path = PathBuf::from(candidate);
                let filtered_paths_from_anywhere = paths_from_anywhere.iter().filter(|f|f.starts_with(&true_path)).cloned().collect::<Vec<_>>();
                construct_tree_out_of_flat_list_of_paths(&filtered_paths_from_anywhere)
            }
        };

        let use_ast = args.iter().any(|x| x.text == "--ast");

        let tree = print_files_tree_with_budget(ccx.global_context.clone(), tree, use_ast).await.map_err(|err| {
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
