use std::collections::HashSet;
use std::fs;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock as ARwLock;

use crate::call_validation;
use crate::files_correction::get_project_dirs;
use crate::global_context::GlobalContext;
use crate::http::http_post_json;
use crate::http::routers::v1::system_prompt::{PrependSystemPromptPost, PrependSystemPromptResponse};
use crate::integrations::docker::docker_container_manager::docker_container_get_host_lsp_port_to_connect;
use crate::scratchpads::scratchpad_utils::HasRagResults;
use crate::scratchpads::system_context::{
    self, create_instruction_files_message, gather_system_context, generate_git_info_prompt,
    gather_git_info, INTERNAL_CONTEXT_GUIDANCE,
};
use crate::call_validation::{ChatMessage, ChatContent, ChatMode};


pub async fn get_default_system_prompt(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_mode: ChatMode,
) -> String {
    let mut error_log = Vec::new();
    let tconfig = crate::yaml_configs::customization_loader::load_customization(gcx.clone(), true, &mut error_log).await;
    for e in error_log.iter() {
        tracing::error!("{e}");
    }
    let prompt_key = match chat_mode {
        ChatMode::NO_TOOLS => "default",
        ChatMode::EXPLORE => "exploration_tools",
        ChatMode::AGENT => "agentic_tools",
        ChatMode::CONFIGURE => "configurator",
        ChatMode::PROJECT_SUMMARY => "project_summary",
    };
    let system_prompt = tconfig.system_prompts.get(prompt_key).map_or_else(|| {
        tracing::error!("cannot find system prompt `{}`", prompt_key);
        String::new()
    }, |x| x.text.clone());
    system_prompt
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
        },
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
    async fn workspace_files_info(gcx: &Arc<ARwLock<GlobalContext>>) -> (Vec<String>, Option<PathBuf>) {
        let gcx_locked = gcx.read().await;
        let documents_state = &gcx_locked.documents_state;
        let dirs_locked = documents_state.workspace_folders.lock().unwrap();
        let workspace_dirs = dirs_locked.clone().into_iter().map(|x| x.to_string_lossy().to_string()).collect();
        let active_file_path = documents_state.active_file_path.clone();
        (workspace_dirs, active_file_path)
    }

    let mut system_prompt = system_prompt.clone();

    // New: %SYSTEM_INFO% - OS, datetime, username, architecture
    if system_prompt.contains("%SYSTEM_INFO%") {
        let system_info = system_context::SystemInfo::gather();
        system_prompt = system_prompt.replace("%SYSTEM_INFO%", &system_info.to_prompt_string());
    }

    // New: %ENVIRONMENT_INFO% - Detected environments and usage instructions
    if system_prompt.contains("%ENVIRONMENT_INFO%") {
        let project_dirs = get_project_dirs(gcx.clone()).await;
        let environments = system_context::detect_environments(&project_dirs).await;
        let env_instructions = system_context::generate_environment_instructions(&environments);
        system_prompt = system_prompt.replace("%ENVIRONMENT_INFO%", &env_instructions);
    }

    // New: %PROJECT_CONFIGS% - Detected project configuration files
    if system_prompt.contains("%PROJECT_CONFIGS%") {
        let project_dirs = get_project_dirs(gcx.clone()).await;
        let configs = system_context::find_project_configs(&project_dirs).await;
        if !configs.is_empty() {
            let config_list = configs
                .iter()
                .map(|c| format!("- {} ({})", c.file_name, c.category))
                .collect::<Vec<_>>()
                .join("\n");
            let config_section = format!("## Project Configuration Files\n{}", config_list);
            system_prompt = system_prompt.replace("%PROJECT_CONFIGS%", &config_section);
        } else {
            system_prompt = system_prompt.replace("%PROJECT_CONFIGS%", "");
        }
    }

    if system_prompt.contains("%PROJECT_TREE%") {
        match system_context::generate_compact_project_tree(gcx.clone(), 4).await {
            Ok(tree) if !tree.is_empty() => {
                let tree_section = format!("## Project Structure\n```\n{}```", tree);
                system_prompt = system_prompt.replace("%PROJECT_TREE%", &tree_section);
            }
            _ => {
                system_prompt = system_prompt.replace("%PROJECT_TREE%", "");
            }
        }
    }

    if system_prompt.contains("%GIT_INFO%") {
        let project_dirs = get_project_dirs(gcx.clone()).await;
        let git_infos = gather_git_info(&project_dirs).await;
        let git_section = generate_git_info_prompt(&git_infos);
        system_prompt = system_prompt.replace("%GIT_INFO%", &git_section);
    }

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
            if let Some(core_memories) = crate::memories::memories_get_core(gcx.clone()).await.ok() {
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
        let replacement = if tool_names.contains("create_textdoc") || tool_names.contains("update_textdoc") {
            "- Then use `*_textdoc()` tools to make changes.\n"
        } else {
            ""
        };

        system_prompt = system_prompt.replace("%EXPLORE_FILE_EDIT_INSTRUCTIONS%", replacement);
    }

    if system_prompt.contains("%AGENT_EXPLORATION_INSTRUCTIONS%") {
        let cfg = crate::yaml_configs::customization_loader::load_customization_compiled_in();
        let replacement = cfg.get("AGENT_EXPLORATION_INSTRUCTIONS")
            .and_then(|x| x.as_str())
            .unwrap_or("- Call available tools to find relevant files.\n");
        system_prompt = system_prompt.replace("%AGENT_EXPLORATION_INSTRUCTIONS%", replacement);
    }

    if system_prompt.contains("%AGENT_EXECUTION_INSTRUCTIONS%") {
        let has_edit_tools = tool_names.contains("create_textdoc") || tool_names.contains("update_textdoc");
        let replacement = if has_edit_tools {
            let cfg = crate::yaml_configs::customization_loader::load_customization_compiled_in();
            cfg.get("AGENT_EXECUTION_INSTRUCTIONS")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string()
        } else {
"  - Propose the changes to the user
    - the suspected root cause
    - the exact files/functions to modify or create
    - the new or updated tests to add
    - the expected outcome and success criteria
".to_string()
        };
        system_prompt = system_prompt.replace("%AGENT_EXECUTION_INSTRUCTIONS%", &replacement);
    }

    system_prompt
}

pub async fn prepend_the_right_system_prompt_and_maybe_more_initial_messages(
    gcx: Arc<ARwLock<GlobalContext>>,
    mut messages: Vec<call_validation::ChatMessage>,
    chat_meta: &call_validation::ChatMeta,
    stream_back_to_user: &mut HasRagResults,
    tool_names: HashSet<String>,
) -> Vec<call_validation::ChatMessage> {
    let have_system = !messages.is_empty() && messages[0].role == "system";
    if have_system {
        return messages;
    }
    if messages.len() == 0 {
        tracing::error!("What's that? Messages list is empty");
        return messages;
    }

    let is_inside_container = gcx.read().await.cmdline.inside_container;
    if chat_meta.chat_remote && !is_inside_container {
        messages = match prepend_system_prompt_and_maybe_more_initial_messages_from_remote(gcx.clone(), &messages, chat_meta, stream_back_to_user).await {
            Ok(messages_from_remote) => messages_from_remote,
            Err(e) => {
                tracing::error!("prepend_the_right_system_prompt_and_maybe_more_initial_messages_from_remote: {}", e);
                messages
            },
        };
        return messages;
    }

    match chat_meta.chat_mode {
        ChatMode::EXPLORE | ChatMode::AGENT | ChatMode::NO_TOOLS => {
            let system_message_content = system_prompt_add_extra_instructions(
                gcx.clone(),
                get_default_system_prompt(gcx.clone(), chat_meta.chat_mode.clone()).await,
                tool_names,
            ).await;
            let msg = ChatMessage {
                role: "system".to_string(),
                content: ChatContent::SimpleText(system_message_content),
                ..Default::default()
            };
            stream_back_to_user.push_in_json(serde_json::json!(msg));
            messages.insert(0, msg);
        },
        ChatMode::CONFIGURE => {
            crate::integrations::config_chat::mix_config_messages(
                gcx.clone(),
                &chat_meta,
                &mut messages,
                stream_back_to_user,
            ).await;
        },
        ChatMode::PROJECT_SUMMARY => {
            crate::integrations::project_summary_chat::mix_project_summary_messages(
                gcx.clone(),
                &chat_meta,
                &mut messages,
                stream_back_to_user,
            ).await;
        },
    }

    match gather_and_inject_system_context(&gcx, &mut messages, stream_back_to_user).await {
        Ok(()) => {},
        Err(e) => {
            tracing::warn!("Failed to gather system context: {}", e);
        },
    }

    tracing::info!("\n\nSYSTEM PROMPT MIXER chat_mode={:?}\n{:#?}", chat_meta.chat_mode, messages);
    messages
}

async fn gather_and_inject_system_context(
    gcx: &Arc<ARwLock<GlobalContext>>,
    messages: &mut Vec<ChatMessage>,
    stream_back_to_user: &mut HasRagResults,
) -> Result<(), String> {
    let context = gather_system_context(gcx.clone(), false, 4).await?;

    if !context.instruction_files.is_empty() {
        match create_instruction_files_message(&context.instruction_files).await {
            Ok(instr_msg) => {
                let first_user_pos = messages.iter().position(|m| m.role == "user");

                if let Some(pos) = first_user_pos {
                    stream_back_to_user.push_in_json(serde_json::json!(instr_msg));
                    messages.insert(pos, instr_msg);

                    if let Some(system_msg) = messages.iter_mut().find(|m| m.role == "system") {
                        if let ChatContent::SimpleText(ref mut text) = system_msg.content {
                            text.push_str(INTERNAL_CONTEXT_GUIDANCE);
                        }
                    }

                    tracing::info!(
                        "Injected {} instruction files before first user message: {:?}",
                        context.instruction_files.len(),
                        context.instruction_files.iter().map(|f| &f.file_name).collect::<Vec<_>>()
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create instruction files message: {}", e);
            }
        }
    }

    if !context.detected_environments.is_empty() {
        tracing::info!(
            "Detected {} environments: {:?}",
            context.detected_environments.len(),
            context.detected_environments.iter().map(|e| &e.env_type).collect::<Vec<_>>()
        );
    }

    Ok(())
}

pub async fn prepend_system_prompt_and_maybe_more_initial_messages_from_remote(
    gcx: Arc<ARwLock<GlobalContext>>,
    messages: &[call_validation::ChatMessage],
    chat_meta: &call_validation::ChatMeta,
    stream_back_to_user: &mut HasRagResults,
) -> Result<Vec<call_validation::ChatMessage>, String> {
    let post = PrependSystemPromptPost {
        messages: messages.to_vec(),
        chat_meta: chat_meta.clone(),
    };

    let port = docker_container_get_host_lsp_port_to_connect(gcx.clone(), &chat_meta.chat_id).await?;
    let url = format!("http://localhost:{port}/v1/prepend-system-prompt-and-maybe-more-initial-messages");
    let response: PrependSystemPromptResponse = http_post_json(&url, &post).await?;

    for msg in response.messages_to_stream_back {
        stream_back_to_user.push_in_json(msg);
    }

    Ok(response.messages)
}
