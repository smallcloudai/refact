use std::fs;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock as ARwLock;
use tracing::info;

use crate::global_context::GlobalContext;
use crate::http::http_post_json;
use crate::http::routers::v1::system_prompt::{SystemPromptPost, SystemPromptResponse};
use crate::integrations::docker::docker_container_manager::docker_container_get_host_lsp_port_to_connect;


pub async fn get_default_system_prompt(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_mode: crate::call_validation::ChatMode,
) -> String {
    let tconfig = match crate::yaml_configs::customization_loader::load_customization(gcx.clone(), true).await {
        Ok(tconfig) => tconfig,
        Err(e) => {
            tracing::error!("cannot load_customization: {e}");
            return String::new();
        },
    };
    let prompt_key = match chat_mode {
        crate::call_validation::ChatMode::NO_TOOLS => "default",
        crate::call_validation::ChatMode::EXPLORE => "exploration_tools",
        crate::call_validation::ChatMode::AGENT => "agentic_tools",
        crate::call_validation::ChatMode::CONFIGURE => "configurator",
        crate::call_validation::ChatMode::PROJECT_SUMMARY => "project_summary",
    };
    let system_prompt = tconfig.system_prompts.get(prompt_key).map_or_else(|| {
        tracing::error!("cannot find system prompt `{}`", prompt_key);
        String::new()
    }, |x| x.text.clone());
    system_prompt
}

pub async fn get_default_system_prompt_from_remote(
    gcx: Arc<ARwLock<GlobalContext>>,
    have_exploration_tools: bool,
    have_agentic_tools: bool,
    chat_id: &str,
) -> Result<String, String>
{
    let post = SystemPromptPost {
        have_exploration_tools,
        have_agentic_tools
    };

    let port = docker_container_get_host_lsp_port_to_connect(gcx.clone(), chat_id).await?;
    let url = format!("http://localhost:{port}/v1/system-prompt");
    let response: SystemPromptResponse = http_post_json(&url, &post).await?;
    info!("get_default_system_prompt_from_remote: got response: {:?}", response);
    Ok(response.system_prompt)
}

async fn _workspace_info(
    workspace_dirs: &[String],
    active_file_path: &Option<PathBuf>,
) -> String
{
    async fn get_vcs_info(detect_vcs_at: &PathBuf) -> String {
        let mut info = String::new();
        if let Some((vcs_path, vcs_type)) = crate::files_in_workspace::detect_vcs_for_a_file_path(detect_vcs_at).await {
            info.push_str(&format!("\nThe project is under {} version control, located at:\n{}", vcs_type, vcs_path.display()));
        } else {
            info.push_str("\nThere's no version control detected, complain to user if they want to use anything git/hg/svn/etc.");
        }
        info
    }
    let mut info = String::new();
    if !workspace_dirs.is_empty() {
        info.push_str(&format!("The current IDE workspace has these project directories:\n{}", workspace_dirs.join("\n")));
    }
    let detect_vcs_at_option = active_file_path.clone().or_else(|| workspace_dirs.get(0).map(PathBuf::from));
    if let Some(detect_vcs_at) = detect_vcs_at_option {
        let vcs_info = get_vcs_info(&detect_vcs_at).await;
        if let Some(active_file) = active_file_path {
            info.push_str(&format!("\n\nThe active IDE file is:\n{}", active_file.display()));
        } else {
            info.push_str("\n\nThere is no active file currently open in the IDE.");
        }
        info.push_str(&vcs_info);
    } else {
        info.push_str("\n\nThere is no active file with version control, complain to user if they want to use anything git/hg/svn/etc and ask to open a file in IDE for you to know which project is active.");
    }
    info
}

pub async fn dig_for_project_summarization_file(gcx: Arc<ARwLock<GlobalContext>>) -> (bool, Option<String>) {
    match crate::files_correction::get_active_project_path(gcx.clone()).await {
        Some(active_project_path) => {
            let summary_path = active_project_path.join(".refact").join("project_summary.yaml");
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

async fn _read_project_summary(
    summary_path: String,
) -> Option<String> {
    match fs::read_to_string(summary_path) {
        Ok(content) => {
            match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                Ok(yaml) => {
                    if let Some(project_summary) = yaml.get("project_summary") {
                        match serde_yaml::to_string(project_summary) {
                            Ok(summary_str) => return Some(summary_str),
                            Err(e) => {
                                tracing::error!("Failed to convert project summary to string: {}", e);
                                return None;
                            }
                        }
                    } else {
                        tracing::error!("Key 'project_summary' not found in YAML file.");
                        return None;
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to parse project summary YAML file: {}", e);
                    return None;
                }
            }
        },
        Err(e) => {
            tracing::error!("Failed to read project summary file: {}", e);
            return None;
        }
    }
}

pub async fn system_prompt_add_workspace_info(
    gcx: Arc<ARwLock<GlobalContext>>,
    system_prompt: &String,
) -> String {
    async fn workspace_files_info(gcx: &Arc<ARwLock<GlobalContext>>) -> (Vec<String>, Option<PathBuf>) {
        let gcx_locked = gcx.read().await;
        let documents_state = &gcx_locked.documents_state;
        let dirs_locked = documents_state.workspace_folders.lock().unwrap();
        let workspace_dirs = dirs_locked.clone().into_iter().map(|x| x.to_string_lossy().to_string()).collect();
        let active_file_path = documents_state.active_file_path.clone();
        (workspace_dirs, active_file_path)
    }

    let mut system_prompt = system_prompt.clone();
    if system_prompt.contains("%WORKSPACE_INFO%") {
        let (workspace_dirs, active_file_path) = workspace_files_info(&gcx).await;
        let info = _workspace_info(&workspace_dirs, &active_file_path).await;
        system_prompt = system_prompt.replace("%WORKSPACE_INFO%", &info);
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

    tracing::info!("system_prompt\n{}", system_prompt);

    system_prompt
}
