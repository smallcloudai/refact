use serde_yaml;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use indexmap::IndexMap;
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::{ChatMessage, SubchatParameters};
use crate::global_context::{GlobalContext, try_load_caps_quickly_if_not_present};
use crate::custom_error::YamlError;


#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CustomizationYaml {
    #[serde(default)]
    pub system_prompts: IndexMap<String, SystemPrompt>,
    #[serde(default)]
    pub subchat_tool_parameters: IndexMap<String, SubchatParameters>,
    #[serde(default)]
    pub toolbox_commands: IndexMap<String, ToolboxCommand>,
    #[serde(default)]
    pub code_lens: IndexMap<String, CodeLensCommand>,
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

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeLensCommand {
    pub label: String,
    pub auto_submit: bool,
    #[serde(default = "default_true")]
    pub new_tab: bool,
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
}

fn _extract_mapping_values(mapping: &Option<&serde_yaml::Mapping>, variables: &mut HashMap<String, String>) {
    if let Some(mapping) = mapping {
        for (k, v) in mapping.iter() {
            if let (serde_yaml::Value::String(key), serde_yaml::Value::String(value)) = (k, v) {
                variables.insert(key.clone(), value.clone());
            }
        }
    }
}

fn _replace_variables_in_text(text: &mut String, variables: &HashMap<String, String>) -> bool {
    let mut replaced = false;
    for (vname, vtext) in variables.iter() {
        let placeholder = format!("%{}%", vname);
        if text.contains(&placeholder) {
            *text = text.replace(&placeholder, vtext);
            replaced = true;
        }
        if text.contains(&placeholder) {
            tracing::error!("replacement might be buggy {placeholder} in {}...", text.chars().take(100).collect::<String>().replace("\n", "\\n"));
        }
    }
    replaced
}

fn _replace_variables_in_messages(config: &mut CustomizationYaml, variables: &HashMap<String, String>)
{
    // this is about pre-filled messages in tools, there are no images
    for command in config.toolbox_commands.values_mut() {
        for msg in command.messages.iter_mut() {
            let mut replaced = true;
            let mut countdown = 10;
            while replaced && countdown > 0 {
                replaced = _replace_variables_in_text(&mut msg.content.content_text_only(), variables);
                countdown -= 1;
            }
        }
    }
    for command in config.code_lens.values_mut() {
        for msg in command.messages.iter_mut() {
            let mut replaced = true;
            let mut countdown = 10;
            while replaced && countdown > 0 {
                replaced = _replace_variables_in_text(&mut msg.content.content_text_only(), variables);
                countdown -= 1;
            }
        }
    }
}

fn _replace_variables_in_system_prompts(config: &mut CustomizationYaml, variables: &HashMap<String, String>) {
    for prompt in config.system_prompts.values_mut() {
        let mut replaced = true;
        let mut countdown = 10;
        while replaced && countdown > 0 {
            replaced = _replace_variables_in_text(&mut prompt.text, variables);
            countdown -= 1;
        }
    }
}

pub fn load_and_mix_with_users_config(
    user_yaml: &str,
    caps_yaml: &str,
    skip_visibility_filtering: bool,
    allow_experimental: bool,
    error_log: &mut Vec<YamlError>,
) -> CustomizationYaml {
    let compiled_in_customization = include_str!("customization_compiled_in.yaml");

    let default_unstructured: serde_yaml::Value = serde_yaml::from_str(compiled_in_customization).unwrap();   // compiled-in cannot fail
    let user_unstructured: serde_yaml::Value = serde_yaml::from_str(user_yaml)
        .map_err(|e| {
            error_log.push(YamlError {
                path: "customization.yaml".to_string(),
                error_line: 0,
                error_msg: e.to_string(),
            });
            format!("Error parsing customization.yaml: {}\n{}", e, user_yaml)
        }).unwrap_or_default();

    let mut variables = HashMap::new();
    _extract_mapping_values(&default_unstructured.as_mapping(), &mut variables);
    _extract_mapping_values(&user_unstructured.as_mapping(), &mut variables);

    let mut work_config: CustomizationYaml = serde_yaml::from_str(compiled_in_customization).unwrap();
    let mut user_config: CustomizationYaml = serde_yaml::from_str(user_yaml)
        .map_err(|e| {
            error_log.push(YamlError {
                path: "customization.yaml".to_string(),
                error_line: 0,
                error_msg: e.to_string(),
            });
            format!("Error parsing user ToolboxConfig: {}\n{}", e, user_yaml)
        }).unwrap_or_default();
    let caps_config: CustomizationYaml = serde_yaml::from_str(caps_yaml)
        .map_err(|e| {
            error_log.push(YamlError {
                path: "caps.yaml".to_string(),
                error_line: 0,
                error_msg: e.to_string(),
            });
            format!("Error parsing default ToolboxConfig: {}\n{}", e, caps_yaml)
        }).unwrap_or_default();

    _replace_variables_in_messages(&mut work_config, &variables);
    _replace_variables_in_messages(&mut user_config, &variables);
    _replace_variables_in_system_prompts(&mut work_config, &variables);
    _replace_variables_in_system_prompts(&mut user_config, &variables);

    work_config.system_prompts.extend(caps_config.system_prompts.iter().map(|(k, v)| (k.clone(), v.clone())));
    work_config.toolbox_commands.extend(caps_config.toolbox_commands.iter().map(|(k, v)| (k.clone(), v.clone())));
    work_config.code_lens.extend(caps_config.code_lens.iter().map(|(k, v)| (k.clone(), v.clone())));

    work_config.system_prompts.extend(user_config.system_prompts.iter().map(|(k, v)| (k.clone(), v.clone())));
    work_config.toolbox_commands.extend(user_config.toolbox_commands.iter().map(|(k, v)| (k.clone(), v.clone())));
    work_config.code_lens.extend(user_config.code_lens.iter().map(|(k, v)| (k.clone(), v.clone())));

    let filtered_system_prompts = work_config.system_prompts
        .iter()
        .filter(|(_key, system_prompt_struct)| {
            skip_visibility_filtering || match system_prompt_struct.show.as_str() {
                "always" => true,
                "never" => false,
                "experimental" => allow_experimental,
                _ => true,
            }
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    work_config.system_prompts = filtered_system_prompts;
    work_config
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Competency {
    #[serde(default)]
    system_prompt_vars: HashMap<String, String>,
}

pub async fn load_customization(
    gcx: Arc<ARwLock<GlobalContext>>,
    skip_visibility_filtering: bool,
    error_log: &mut Vec<YamlError>,
) -> CustomizationYaml {
    let allow_experimental = gcx.read().await.cmdline.experimental;
    let caps = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => caps,
        Err(e) => {
            error_log.push(YamlError {
                path: "bring-your-own-key.yaml".to_string(),
                error_line: 0,
                error_msg: format!("error loading caps: {e}"),
            });
            return CustomizationYaml::default();
        }
    };

    let caps_config_text = {
        let caps_locked = caps.read().unwrap();
        caps_locked.customization.clone()
    };

    let config_dir = gcx.read().await.config_dir.clone();
    let customization_yaml_path = config_dir.join("customization.yaml");
    let user_config_text = std::fs::read_to_string(&customization_yaml_path)
        .map_err(|e| format!("Failed to read file: {}", e))
        .unwrap_or_default();

    load_and_mix_with_users_config(
        &user_config_text,
        &caps_config_text,
        skip_visibility_filtering,
        allow_experimental,
        error_log,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn are_all_system_prompts_present() {
        let mut error_log = Vec::new();
        let config = load_and_mix_with_users_config(
            "", "", true, true, &mut error_log,
        );
        for e in error_log.iter() {
            eprintln!("{e}");
        }
        assert!(error_log.is_empty(), "There were errors in the error_log");
        assert_eq!(config.system_prompts.get("default").is_some(), true);
        assert_eq!(config.system_prompts.get("exploration_tools").is_some(), true);
        assert_eq!(config.system_prompts.get("agentic_tools").is_some(), true);
        assert_eq!(config.system_prompts.get("configurator").is_some(), true);
        assert_eq!(config.system_prompts.get("project_summary").is_some(), true);
    }
}
