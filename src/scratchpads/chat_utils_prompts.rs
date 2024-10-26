use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock as ARwLock;
use crate::global_context::GlobalContext;


pub async fn get_default_system_prompt(
    gcx: Arc<ARwLock<GlobalContext>>,
    have_exploration_tools: bool,
    have_agentic_tools: bool,
) -> String {
    let tconfig = match crate::yaml_configs::customization_loader::load_customization(gcx.clone(), true).await {
        Ok(tconfig) => tconfig,
        Err(e) => {
            tracing::error!("cannot load_customization: {e}");
            return String::new();
        },
    };
    let prompt_key = if have_agentic_tools {
        "agentic_tools"
    } else if have_exploration_tools {
        "exploration_tools"
    } else {
        "default"
    };
    tconfig.system_prompts.get(prompt_key).map_or_else(|| {
        tracing::error!("cannot find system prompt `{}`", prompt_key);
        String::new()
    },|x| x.text.clone())
}

async fn _workspace_info(
    workspace_dirs: &[String],
    active_file_path: &Option<PathBuf>,
    // background_tools_running: Vec<String>
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
    // if !background_tools_running.is_empty() {
    //     info.push_str(&format!("\n\nThe tools that are currently in a background:\n{}", background_tools_running.join(", ")));
    // }
    info
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
        // let background_tools_running = gcx.read().await.integration_sessions.keys()
        //     .filter(|k| k.starts_with("custom_service_"))
        //     .map(|k| k.trim_start_matches("custom_service_").to_string())
        //     .collect::<Vec<_>>();
        let info = _workspace_info(&workspace_dirs, &active_file_path).await;
        system_prompt = system_prompt.replace("%WORKSPACE_INFO%", &info);
        tracing::info!("system prompt:\n{}", system_prompt);
    }

    system_prompt
}

