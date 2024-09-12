use serde_yaml;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use tracing::{error, info};

use indexmap::IndexMap;
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::{ChatMessage, SubchatParameters};
use crate::global_context::{GlobalContext, try_load_caps_quickly_if_not_present};
use crate::yaml_configs::create_configs::yaml_customization_exists_or_create;
use crate::yaml_configs::customization_compiled_in::COMPILED_IN_CUSTOMIZATION_YAML;


#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CustomizationYaml {
    #[serde(default)]
    pub system_prompts: IndexMap<String, SystemPrompt>,
    #[serde(default)]
    pub subchat_tool_parameters: IndexMap<String, SubchatParameters>,
    #[serde(default)]
    pub toolbox_commands: IndexMap<String, ToolboxCommand>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemPrompt {
    #[serde(default)]
    pub description: String,
    pub text: String,
    #[serde(default)]
    pub show: String,  // "always" (same as "") "never" "experimental"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolboxCommand {
    pub description: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub selection_needed: Vec<usize>,
    #[serde(default)]
    pub selection_unwanted: bool,
    #[serde(default)]
    pub insert_at_cursor: bool,
}

fn extract_mapping_values(mapping: &Option<&serde_yaml::Mapping>, variables: &mut HashMap<String, String>) {
    if let Some(mapping) = mapping {
        for (k, v) in mapping.iter() {
            if let (serde_yaml::Value::String(key), serde_yaml::Value::String(value)) = (k, v) {
                variables.insert(key.clone(), value.clone());
            }
        }
    }
}

fn replace_variables_in_text(text: &mut String, variables: &HashMap<String, String>) {
    for (vname, vtext) in variables.iter() {
        *text = text.replace(&format!("%{}%", vname), vtext);
    }
}

fn replace_variables_in_messages(config: &mut CustomizationYaml, variables: &HashMap<String, String>) {
    for command in config.toolbox_commands.values_mut() {
        for msg in command.messages.iter_mut() {
            replace_variables_in_text(&mut msg.content, variables);
        }
    }
}

fn replace_variables_in_system_prompts(config: &mut CustomizationYaml, variables: &HashMap<String, String>) {
    for prompt in config.system_prompts.values_mut() {
        replace_variables_in_text(&mut prompt.text, variables);
    }
}

fn load_and_mix_with_users_config(
    user_yaml: &str, 
    caps_yaml: &str, 
    caps_default_system_prompt: &str, 
    skip_filtering: bool, 
    allow_experimental: bool
) -> Result<CustomizationYaml, String> {
    let default_unstructured: serde_yaml::Value = serde_yaml::from_str(COMPILED_IN_CUSTOMIZATION_YAML)
        .map_err(|e| format!("Error parsing default YAML: {}\n{}", e, COMPILED_IN_CUSTOMIZATION_YAML))?;
    let user_unstructured: serde_yaml::Value = serde_yaml::from_str(user_yaml)
        .map_err(|e| format!("Error parsing customization.yaml: {}\n{}", e, user_yaml))?;

    let mut variables = HashMap::new();
    extract_mapping_values(&default_unstructured.as_mapping(), &mut variables);
    extract_mapping_values(&user_unstructured.as_mapping(), &mut variables);

    let mut work_config: CustomizationYaml = serde_yaml::from_str(COMPILED_IN_CUSTOMIZATION_YAML)
        .map_err(|e| format!("Error parsing default ToolboxConfig: {}\n{}", e, COMPILED_IN_CUSTOMIZATION_YAML))?;
    let mut user_config: CustomizationYaml = serde_yaml::from_str(user_yaml)
        .map_err(|e| format!("Error parsing user ToolboxConfig: {}\n{}", e, user_yaml))?;
    let caps_config: CustomizationYaml = serde_yaml::from_str(caps_yaml)
        .map_err(|e| format!("Error parsing default ToolboxConfig: {}\n{}", e, caps_yaml))?;

    replace_variables_in_messages(&mut work_config, &variables);
    replace_variables_in_messages(&mut user_config, &variables);
    replace_variables_in_system_prompts(&mut work_config, &variables);
    replace_variables_in_system_prompts(&mut user_config, &variables);
    
    work_config.system_prompts.extend(caps_config.system_prompts.iter().map(|(k, v)| (k.clone(), v.clone())));
    work_config.toolbox_commands.extend(caps_config.toolbox_commands.iter().map(|(k, v)| (k.clone(), v.clone())));

    work_config.system_prompts.extend(user_config.system_prompts.iter().map(|(k, v)| (k.clone(), v.clone())));
    work_config.toolbox_commands.extend(user_config.toolbox_commands.iter().map(|(k, v)| (k.clone(), v.clone())));

    let filtered_system_prompts = work_config.system_prompts
        .iter()
        .filter(|(_key, system_prompt_struct)| {
            skip_filtering || match system_prompt_struct.show.as_str() {
                "always" => true,
                "never" => false,
                "experimental" => allow_experimental,
                _ => true,
            }
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    work_config.system_prompts = filtered_system_prompts;

    if let Some(default_system_prompt) = work_config.system_prompts.get(caps_default_system_prompt) {
        work_config.system_prompts.insert("default".to_string(), default_system_prompt.clone());
    }
    Ok(work_config)
}

pub async fn load_customization(gcx: Arc<ARwLock<GlobalContext>>, skip_filtering: bool) -> Result<CustomizationYaml, String> {
    let allow_experimental = gcx.read().await.cmdline.experimental;
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await.map_err(|e|format!("error loading caps: {e}"))?;

    let (caps_config_text, caps_default_system_prompt) = {
        let caps_locked = caps.read().unwrap();
        (caps_locked.customization.clone(), caps_locked.code_chat_default_system_prompt.clone())
    };

    let user_config_path = yaml_customization_exists_or_create(gcx.clone()).await?;
    
    let user_config_text = std::fs::read_to_string(&user_config_path).map_err(|e| format!("Failed to read file: {}", e))?;
    load_and_mix_with_users_config(&user_config_text, &caps_config_text, &caps_default_system_prompt, skip_filtering, allow_experimental).map_err(|e| e.to_string())
}

async fn workspace_info(workspace_dirs: &[String], active_file_path: &Option<PathBuf>) -> String {
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
        let info = workspace_info(&workspace_dirs, &active_file_path).await;
        system_prompt = system_prompt.replace("%WORKSPACE_INFO%", &info);
        info!("system prompt:\n{}", system_prompt);
    }

    system_prompt
}

pub async fn get_default_system_prompt(
    gcx: Arc<ARwLock<GlobalContext>>,
    have_exploration_tools: bool,
    have_agentic_tools: bool,
) -> String {
    let tconfig = match load_customization(gcx.clone(), true).await {
        Ok(tconfig) => tconfig,
        Err(e) => {
            error!("cannot load_customization: {e}");
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
        error!("cannot find system prompt `{}`", prompt_key);
        String::new()
    },|x| x.text.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::yaml_configs::customization_compiled_in::COMPILED_IN_INITIAL_USER_YAML;
    #[test]
    fn is_compiled_in_toolbox_valid_yaml() {
        let _config = load_and_mix_with_users_config(COMPILED_IN_INITIAL_USER_YAML, "", "", false, true);
    }
    #[test]
    fn are_all_system_prompts_present() {
        let config = load_and_mix_with_users_config(
            COMPILED_IN_INITIAL_USER_YAML, "", "", true, true
        );
        assert_eq!(config.is_ok(), true);
        let config = config.unwrap();
        
        assert_eq!(config.system_prompts.get("default").is_some(), true);
        assert_eq!(config.system_prompts.get("exploration_tools").is_some(), true);
        assert_eq!(config.system_prompts.get("agentic_tools").is_some(), true);
        assert_eq!(config.system_prompts.get("agentic_experimental_knowledge").is_some(), true);
    }
}
