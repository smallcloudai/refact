use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
use chrono::Local;
use serde_json::Value;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};

use crate::{
    at_commands::{
        at_commands::AtCommandsContext,
        at_tree::{construct_tree_out_of_flat_list_of_paths, PathsHolderNodeArc},
    },
    cached_tokenizers,
    call_validation::{ChatContent, ChatMessage, ChatUsage, ContextEnum, ContextFile, PostprocessSettings},
    files_correction::{get_project_dirs, paths_from_anywhere},
    files_in_workspace::{get_file_text_from_memory_or_disk, ls_files},
    global_context::GlobalContext,
    postprocessing::pp_context_files::postprocess_context_files,
    subchat::subchat,
    tools::tools_description::Tool,
};
use crate::global_context::try_load_caps_quickly_if_not_present;

const MAX_EXPLORATION_STEPS: usize = 1000;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct ExplorationTarget {
    target_name: String,
}

#[derive(Debug)]
struct ExplorationState {
    explored: HashSet<ExplorationTarget>,
    to_explore: Vec<ExplorationTarget>,
    project_tree: Option<Vec<PathsHolderNodeArc>>,
}

impl ExplorationState {
    fn get_tree_stats(tree: &[PathsHolderNodeArc]) -> (usize, f64) {
        fn traverse(node: &PathsHolderNodeArc) -> (usize, Vec<usize>) {
            let node_ref = node.read();
            let children = node_ref.child_paths();
            if children.is_empty() {
                (1, vec![1])
            } else {
                let child_stats: Vec<_> = children.iter().map(traverse).collect();
                let max_depth = 1 + child_stats.iter().map(|(d, _)| *d).max().unwrap_or(0);
                let mut sizes = vec![children.len()];
                sizes.extend(child_stats.into_iter().flat_map(|(_, s)| s));
                (max_depth, sizes)
            }
        }

        let stats: Vec<_> = tree.iter().map(traverse).collect();
        let max_depth = stats.iter().map(|(d, _)| *d).max().unwrap_or(1);
        let sizes: Vec<_> = stats.into_iter().flat_map(|(_, s)| s).collect();
        let avg_size = sizes.iter().sum::<usize>() as f64 / sizes.len() as f64;
        (max_depth, avg_size)
    }

    fn calculate_importance_score(
        node: &PathsHolderNodeArc,
        depth: usize,
        max_tree_depth: usize,
        avg_dir_size: f64,
        project_dirs: &[std::path::PathBuf],
    ) -> Option<f64> {
        let node_ref = node.read();
        let node_path = node_ref.get_path();

        // Check if the current node is one of the project directories
        let is_project_dir = project_dirs.iter().any(|pd| pd == node_path);

        // Only filter out node if it is NOT a project directory
        if !is_project_dir && (node_ref.file_name().starts_with('.') || node_ref.child_paths().is_empty()) {
            return None;
        }

        let relative_depth = depth as f64 / max_tree_depth as f64;
        let direct_children = node_ref.child_paths().len() as f64;
        let total_children = {
            fn count(n: &PathsHolderNodeArc) -> usize {
                let count_direct = n.read().child_paths().len();
                count_direct + n.read().child_paths().iter().map(count).sum::<usize>()
            }
            count(node) as f64
        };

        // For deep-first exploration: lower score = higher priority (we sort ascending)
        // Invert relative_depth so deeper directories get lower scores
        let depth_score = 1.0 - relative_depth;  // Now deeper dirs get higher relative_depth but lower depth_score
        
        // Size score - smaller directories get lower scores (preferred)
        let size_score = ((direct_children + total_children) as f64 / avg_dir_size).min(1.0);
        
        // Deep directory bonus (subtracts from score for deeper directories)
        let deep_bonus = if relative_depth > 0.8 { 1.0 } else { 0.0 };

        // Calculate final score - lower scores will be explored first
        // Increased depth weight, reduced size impact, increased deep bonus
        Some(depth_score * 0.8 + size_score * 0.1 - deep_bonus * 0.2)
    }

    async fn collect_targets_from_tree(
        tree: &[PathsHolderNodeArc],
        gcx: Arc<ARwLock<GlobalContext>>,
    ) -> Vec<ExplorationTarget> {
        let (max_depth, avg_size) = Self::get_tree_stats(tree);
        let project_dirs = get_project_dirs(gcx.clone()).await;
        
        fn traverse(
            node: &PathsHolderNodeArc,
            depth: usize,
            max_depth: usize,
            avg_size: f64,
            project_dirs: &[std::path::PathBuf],
        ) -> Vec<(ExplorationTarget, f64)> {
            let mut targets = Vec::new();
            
            if let Some(score) = ExplorationState::calculate_importance_score(node, depth, max_depth, avg_size, project_dirs) {
                let node_ref = node.read();
                targets.push((
                    ExplorationTarget {
                        target_name: node_ref.get_path().to_string_lossy().to_string(),
                    },
                    score
                ));
                
                for child in node_ref.child_paths() {
                    targets.extend(traverse(child, depth + 1, max_depth, avg_size, project_dirs));
                }
            }
            targets
        }
        
        let mut scored_targets: Vec<_> = tree.iter()
            .flat_map(|node| traverse(node, 0, max_depth, avg_size, &project_dirs))
            .collect();
        
        scored_targets.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        scored_targets.into_iter().map(|(target, _)| target).collect()
    }

    async fn new(gcx: Arc<ARwLock<GlobalContext>>) -> Result<Self, String> {
        let project_dirs = get_project_dirs(gcx.clone()).await;
        let relative_paths: Vec<PathBuf> = paths_from_anywhere(gcx.clone()).await
            .into_iter()
            .filter_map(|path| 
                project_dirs.iter()
                    .find(|dir| path.starts_with(dir))
                    .map(|dir| {
                        // Get the project directory name
                        let project_name = dir.file_name()
                            .map(|name| name.to_string_lossy().to_string())
                            .unwrap_or_default();
                        
                        // If path is deeper than project dir, append the rest of the path
                        if let Ok(rest) = path.strip_prefix(dir) {
                            if rest.as_os_str().is_empty() {
                                PathBuf::from(&project_name)
                            } else {
                                PathBuf::from(&project_name).join(rest)
                            }
                        } else {
                            PathBuf::from(&project_name)
                        }
                    }))
            .collect();

        let tree = construct_tree_out_of_flat_list_of_paths(&relative_paths);
        let to_explore = Self::collect_targets_from_tree(&tree, gcx.clone()).await;

        Ok(Self {
            explored: HashSet::new(),
            to_explore,
            project_tree: Some(tree),
        })
    }

    fn get_next_target(&self) -> Option<ExplorationTarget> {
        self.to_explore.first().cloned()
    }

    fn mark_explored(&mut self, target: ExplorationTarget) {
        self.explored.insert(target.clone());
        self.to_explore.retain(|x| x != &target);
    }

    fn has_unexplored_targets(&self) -> bool {
        !self.to_explore.is_empty()
    }

    fn get_exploration_summary(&self) -> String {
        let dir_count = self.explored.len();
        format!(
            "Explored {} directories",
            dir_count
        )
    }

    fn project_tree_summary(&self) -> String {
        self.project_tree.as_ref().map_or_else(String::new, |nodes| {
            fn traverse(node: &PathsHolderNodeArc, depth: usize) -> String {
                let node_ref = node.read();
                let mut result = format!("{}{}\n", "  ".repeat(depth), node_ref.file_name());
                for child in node_ref.child_paths() {
                    result.push_str(&traverse(child, depth + 1));
                }
                result
            }
            nodes.iter().map(|n| traverse(n, 0)).collect()
        })
    }
}

async fn read_and_compress_directory(
    gcx: Arc<ARwLock<GlobalContext>>,
    dir_relative: String,
    tokens_limit: usize,
    model: String,
) -> Result<String, String> {
    let project_dirs = get_project_dirs(gcx.clone()).await;
    let base_dir = project_dirs.get(0).ok_or("No project directory found")?;
    let abs_dir = base_dir.join(&dir_relative);

    let files = ls_files(
        &*crate::files_blocklist::reload_indexing_everywhere_if_needed(gcx.clone()).await,
        &abs_dir,
        false
    ).unwrap_or_default();
    tracing::info!(
        target = "memory_bank",
        directory = dir_relative,
        files_count = files.len(),
        token_limit = tokens_limit,
        "Reading and compressing directory"
    );

    if files.is_empty() {
        return Ok("Directory is empty; no files to read.".to_string());
    }

    let mut context_files = Vec::with_capacity(files.len());
    for f in &files {
        let text = get_file_text_from_memory_or_disk(gcx.clone(), f)
            .await
            .unwrap_or_default();
        let lines = text.lines().count().max(1);
        context_files.push(ContextFile {
            file_name: f.to_string_lossy().to_string(),
            file_content: text,
            line1: 1,
            line2: lines,
            symbols: vec![],
            gradient_type: -1,
            usefulness: 0.0,
        });
    }

    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await.map_err(|x| x.message)?;
    let tokenizer = cached_tokenizers::cached_tokenizer(caps, gcx.clone(), model).await.map_err(|e| format!("Tokenizer error: {}", e))?;
    let mut pp_settings = PostprocessSettings::new();
    pp_settings.max_files_n = context_files.len();
    let compressed = postprocess_context_files(
        gcx.clone(),
        &mut context_files,
        tokenizer,
        tokens_limit,
        false,
        &pp_settings,
    ).await;

    Ok(compressed.into_iter()
        .map(|cf| format!("Filename: {}\n```\n{}\n```\n\n", cf.file_name, cf.file_content))
        .collect())
}

pub struct ToolCreateMemoryBank;

const MB_SYSTEM_PROMPT: &str = r###"• Objective:
  – Create a clear, natural language description of the project structure while building a comprehensive architectural understanding.
    Do NOT call create_knowledge() until instructed
    
• Analysis Guidelines:
  1. Start with knowledge(); examine existing context:
     - Review previous descriptions of related components
     - Understand known architectural patterns
     - Map existing module relationships

  2. Describe project structure in natural language:
     - Explain what this directory/module is for
     - Describe key files and their purposes
     - Detail how files work together
     - Note any interesting implementation details
     - Explain naming patterns and organization

  3. Analyze code architecture:
     - Module's role and responsibilities
     - Key types, traits, functions, and their purposes
     - Public interfaces and abstraction boundaries
     - Error handling and data flow patterns
     - Cross-cutting concerns and utilities

  4. Document relationships:
     - Which modules use this one and how
     - What this module uses from others
     - How components communicate
     - Integration patterns and dependencies

  5. Map architectural patterns:
     - Design patterns used and why
     - How it fits in the layered architecture
     - State management approaches
     - Extension and plugin points

  6. Compare with existing knowledge:
     - "This builds upon X from module Y by..."
     - "Unlike module X, this takes a different approach to Y by..."
     - "This introduces a new way to handle X through..."

  7. Use structured format:
     • Purpose: [clear description of what this does]
     • Files: [key files and their roles]
     • Architecture: [design patterns and module relationships]
     • Key Symbols: [important types/traits/functions]
     • Integration: [how it works with other parts]

• Operational Constraint:
  – Do NOT call create_knowledge() until instructed."###;

const MB_EXPERT_WRAP_UP: &str = r###"Call create_knowledge() now with your complete and full analysis from the previous step if you haven't called it yet. Otherwise just type "Finished"."###;

impl ToolCreateMemoryBank {
    fn build_step_prompt(
        state: &ExplorationState,
        target: &ExplorationTarget,
        file_context: Option<&String>,
    ) -> String {
        let mut prompt = String::new();
        prompt.push_str(MB_SYSTEM_PROMPT);
        prompt.push_str(&format!("\n\nNow exploring directory: '{}' from the project '{}'", target.target_name, target.target_name.split('/').next().unwrap_or("")));
        {
            prompt.push_str("\nFocus on details like purpose, organization, and notable files. Here is the project structure:\n");
            prompt.push_str(&state.project_tree_summary());
            if let Some(ctx) = file_context {
                prompt.push_str("\n\nFiles context:\n");
                prompt.push_str(ctx);
            }
        }
        prompt
    }
}

#[async_trait]
impl Tool for ToolCreateMemoryBank {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        _args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let gcx = ccx.lock().await.global_context.clone();
        let params = crate::tools::tools_execute::unwrap_subchat_params(ccx.clone(), "create_memory_bank").await?;
        
        let ccx_subchat = {
            let ccx_lock = ccx.lock().await;
            let mut ctx = AtCommandsContext::new(
                ccx_lock.global_context.clone(),
                params.subchat_n_ctx,
                7,
                false,
                ccx_lock.messages.clone(),
                ccx_lock.chat_id.clone(),
                ccx_lock.should_execute_remotely,
            ).await;
            ctx.subchat_tx = ccx_lock.subchat_tx.clone();
            ctx.subchat_rx = ccx_lock.subchat_rx.clone();
            Arc::new(AMutex::new(ctx))
        };

        let mut state = ExplorationState::new(gcx.clone()).await?;
        let mut final_results = Vec::new();
        let mut step = 0;
        let mut usage_collector = ChatUsage::default();

        while state.has_unexplored_targets() && step < MAX_EXPLORATION_STEPS {
            step += 1;
            let log_prefix = Local::now().format("%Y%m%d-%H%M%S").to_string();
            if let Some(target) = state.get_next_target() {
                tracing::info!(
                    target = "memory_bank",
                    step = step,
                    max_steps = MAX_EXPLORATION_STEPS,
                    directory = target.target_name,
                    "Starting directory exploration"
                );
                let file_context = read_and_compress_directory(
                    gcx.clone(),
                    target.target_name.clone(),
                    params.subchat_tokens_for_rag,
                    params.subchat_model.clone(),
                ).await.map_err(|e| {
                    tracing::warn!("Failed to read/compress files for {}: {}", target.target_name, e);
                    e
                }).ok();

                let step_msg = ChatMessage::new(
                    "user".to_string(),
                    Self::build_step_prompt(&state, &target, file_context.as_ref())
                );

                let subchat_result = subchat(
                    ccx_subchat.clone(),
                    params.subchat_model.as_str(),
                    vec![step_msg],
                    vec!["knowledge".to_string(), "create_knowledge".to_string()],
                    8,
                    params.subchat_max_new_tokens,
                    MB_EXPERT_WRAP_UP,
                    1,
                    None,
                    Some(tool_call_id.clone()),
                    Some(format!("{log_prefix}-memory-bank-dir-{}", target.target_name.replace("/", "_"))),
                    Some(false),
                ).await?[0].clone();

                // Update usage from subchat result
                if let Some(last_msg) = subchat_result.last() {
                    crate::tools::tool_relevant_files::update_usage_from_message(&mut usage_collector, last_msg);
                    tracing::info!(
                        target = "memory_bank",
                        directory = target.target_name,
                        prompt_tokens = usage_collector.prompt_tokens,
                        completion_tokens = usage_collector.completion_tokens,
                        total_tokens = usage_collector.total_tokens,
                        "Updated token usage"
                    );
                }

                state.mark_explored(target.clone());
                let total = state.to_explore.len() + state.explored.len();
                tracing::info!(
                    target = "memory_bank",
                    directory = target.target_name,
                    remaining_dirs = state.to_explore.len(),
                    explored_dirs = state.explored.len(),
                    total_dirs = total,
                    progress = format!("{}/{}", state.to_explore.len(), total),
                    "Completed directory exploration"
                );
            } else {
                break;
            }
        }

        final_results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(format!(
                "Memory bank creation completed. Steps: {}, {}. Total directories: {}. Usage: {} prompt tokens, {} completion tokens",
                step,
                state.get_exploration_summary(),
                state.explored.len() + state.to_explore.len(),
                usage_collector.prompt_tokens,
                usage_collector.completion_tokens,
            )),
            usage: Some(usage_collector),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok((false, final_results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["ast".to_string(), "vecdb".to_string()]
    }

    fn tool_description(&self) -> crate::tools::tools_description::ToolDesc {
        crate::tools::tools_description::ToolDesc {
            name: "create_memory_bank".into(),
            agentic: true,
            experimental: true,
            description: "Gathers information about the project structure (modules, file relations, classes, etc.) and saves this data into the memory bank.".into(),
            parameters: Vec::new(),
            parameters_required: Vec::new(),
        }
    }
}