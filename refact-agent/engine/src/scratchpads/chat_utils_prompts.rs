use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::global_context::GlobalContext;


async fn _workspace_info(workspace_dirs: &[String], active_file_path: &Option<PathBuf>) -> String {
    async fn get_vcs_info(detect_vcs_at: &PathBuf) -> String {
        let mut info = String::new();
        if let Some((vcs_path, vcs_type)) =
            crate::files_in_workspace::detect_vcs_for_a_file_path(detect_vcs_at).await
        {
            info.push_str(&format!(
                "\nThe project is under {} version control, located at:\n{}",
                vcs_type,
                vcs_path.display()
            ));
        } else {
            info.push_str("\nThere's no version control detected, complain to user if they want to use anything git/hg/svn/etc.");
        }
        info
    }
    let mut info = String::new();
    if !workspace_dirs.is_empty() {
        info.push_str(&format!(
            "The current IDE workspace has these project directories:\n{}",
            workspace_dirs.join("\n")
        ));
    }
    let detect_vcs_at_option = active_file_path
        .clone()
        .or_else(|| workspace_dirs.get(0).map(PathBuf::from));
    if let Some(detect_vcs_at) = detect_vcs_at_option {
        let vcs_info = get_vcs_info(&detect_vcs_at).await;
        if let Some(active_file) = active_file_path {
            info.push_str(&format!(
                "\n\nThe active IDE file is:\n{}",
                active_file.display()
            ));
        } else {
            info.push_str("\n\nThere is no active file currently open in the IDE.");
        }
        info.push_str(&vcs_info);
    } else {
        info.push_str("\n\nThere is no active file with version control, complain to user if they want to use anything git/hg/svn/etc and ask to open a file in IDE for you to know which project is active.");
    }
    info
}

pub async fn dig_for_project_summarization_file(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> (bool, Option<String>) {
    match crate::files_correction::get_active_project_path(gcx.clone()).await {
        Some(active_project_path) => {
            let summary_path = active_project_path
                .join(".refact")
                .join("project_summary.yaml");
            if !summary_path.exists() {
                (false, Some(summary_path.to_string_lossy().to_string()))
            } else {
                (true, Some(summary_path.to_string_lossy().to_string()))
            }
        }
        None => {
            tracing::info!("No projects found, project summarization is not relevant.");
            (false, None)
        }
    }
}

async fn _read_project_summary(summary_path: String) -> Option<String> {
    match fs::read_to_string(summary_path) {
        Ok(content) => {
            if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                if let Some(project_summary) = yaml.get("project_summary") {
                    match project_summary {
                        serde_yaml::Value::String(s) => Some(s.clone()),
                        _ => {
                            tracing::error!("'project_summary' is not a string in YAML file.");
                            None
                        }
                    }
                } else {
                    tracing::error!("Key 'project_summary' not found in YAML file.");
                    None
                }
            } else {
                tracing::error!("Failed to parse project summary YAML file.");
                None
            }
        }
        Err(e) => {
            tracing::error!("Failed to read project summary file: {}", e);
            None
        }
    }
}

pub async fn system_prompt_add_extra_instructions(
    gcx: Arc<ARwLock<GlobalContext>>,
    system_prompt: String,
    tool_names: HashSet<String>,
) -> String {
    async fn workspace_files_info(
        gcx: &Arc<ARwLock<GlobalContext>>,
    ) -> (Vec<String>, Option<PathBuf>) {
        let gcx_locked = gcx.read().await;
        let documents_state = &gcx_locked.documents_state;
        let dirs_locked = documents_state.workspace_folders.lock().unwrap();
        let workspace_dirs = dirs_locked
            .clone()
            .into_iter()
            .map(|x| x.to_string_lossy().to_string())
            .collect();
        let active_file_path = documents_state.active_file_path.clone();
        (workspace_dirs, active_file_path)
    }

    let mut system_prompt = system_prompt.clone();
    if system_prompt.contains("%WORKSPACE_INFO%") {
        let (workspace_dirs, active_file_path) = workspace_files_info(&gcx).await;
        let info = _workspace_info(&workspace_dirs, &active_file_path).await;
        system_prompt = system_prompt.replace("%WORKSPACE_INFO%", &info);
    }
    if system_prompt.contains("%KNOWLEDGE_INSTRUCTIONS%") {
        let active_group_id = gcx.read().await.active_group_id.clone();
        if active_group_id.is_some() {
            let cfg = crate::yaml_configs::customization_loader::load_customization_compiled_in();
            let mut knowledge_instructions = cfg.get("KNOWLEDGE_INSTRUCTIONS_META")
                .map(|x| x.as_str().unwrap_or("").to_string()).unwrap_or("".to_string());
            if let Some(core_memories) = crate::cloud::memories_req::memories_get_core(gcx.clone()).await.ok() {
                knowledge_instructions.push_str("\nThere are some pre-existing core memories:\n");
                for mem in core_memories {
                    knowledge_instructions.push_str(&format!("🗃️\n{}\n\n", mem.iknow_memory));
                }
            }
            system_prompt = system_prompt.replace("%KNOWLEDGE_INSTRUCTIONS%", &knowledge_instructions);
            tracing::info!("adding up extra knowledge instructions");
        } else {
            system_prompt = system_prompt.replace("%KNOWLEDGE_INSTRUCTIONS%", "");
        }
    }

    if system_prompt.contains("%PROJECT_SUMMARY%") {
        let (exists, summary_path_option) = dig_for_project_summarization_file(gcx.clone()).await;
        if exists {
            if let Some(summary_path) = summary_path_option {
                if let Some(project_info) = _read_project_summary(summary_path).await {
                    system_prompt = system_prompt.replace("%PROJECT_SUMMARY%", &project_info);
                } else {
                    system_prompt = system_prompt.replace("%PROJECT_SUMMARY%", "");
                }
            }
        } else {
            system_prompt = system_prompt.replace("%PROJECT_SUMMARY%", "");
        }
    }

    if system_prompt.contains("%EXPLORE_FILE_EDIT_INSTRUCTIONS%") {
        let replacement =
            if tool_names.contains("create_textdoc") || tool_names.contains("update_textdoc") {
                "- Then use `*_textdoc()` tools to make changes.\n"
            } else {
                ""
            };

        system_prompt = system_prompt.replace("%EXPLORE_FILE_EDIT_INSTRUCTIONS%", replacement);
    }

    if system_prompt.contains("%AGENT_EXPLORATION_INSTRUCTIONS%") {
        let replacement = if tool_names.contains("locate") {
            "- Call `locate()` tool to find relevant files.\n"
        } else {
            "- Call available tools to find relevant files.\n"
        };

        system_prompt = system_prompt.replace("%AGENT_EXPLORATION_INSTRUCTIONS%", replacement);
    }

    if system_prompt.contains("%AGENT_EXECUTION_INSTRUCTIONS%") {
        let replacement = if tool_names.contains("create_textdoc")
            || tool_names.contains("update_textdoc")
        {
            "3. Confirm the Plan with the User — No Coding Until Approved
  - Post a concise, bullet-point summary that includes
    • the suspected root cause
    • the exact files/functions you will modify or create
    • the new or updated tests you will add
    • the expected outcome and success criteria
  - Explicitly ask “Does this align with your vision?
  - Wait for the user’s approval or revisions before proceeding.
4. Implement the Fix
  - Apply the approved changes directly to project files using `update_textdoc()` and `create_textdoc()` tools.
5. Validate and Improve
  - Run all available tooling to ensure the project compiles and your fix works.
  - Add or update tests that reproduce the original bug and verify they pass.
  - Execute the full test suite to guard against regressions.
  - Iterate until everything is green.
"
        } else {
            "  - Propose the changes to the user
    • the suspected root cause
    • the exact files/functions to modify or create
    • the new or updated tests to add
    • the expected outcome and success criteria
"
        };

        system_prompt = system_prompt.replace("%AGENT_EXECUTION_INSTRUCTIONS%", replacement);
    }

    system_prompt
}
