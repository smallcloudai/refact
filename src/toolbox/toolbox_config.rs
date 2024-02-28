use serde_yaml::Value;
use serde_yaml;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::call_validation::ChatMessage;
use std::io::Write;


#[derive(Debug, Serialize, Deserialize)]
pub struct ToolboxConfig {
    pub system_prompts: HashMap<String, SystemPrompt>,
    pub toolbox_commands: HashMap<String, ToolboxCommand>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemPrompt {
    #[serde(default)]
    pub description: String,
    pub text: String,
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

fn _extract_mapping_values(mapping: &Option<&serde_yaml::Mapping>, variables: &mut HashMap<String, String>)
{
    if let Some(mapping) = mapping {
        for (k, v) in mapping.iter() {
            if let (Value::String(key), Value::String(value)) = (k, v) {
                variables.insert(key.clone(), value.clone());
            }
        }
    }
}

fn _replace_variables_in_messages(config: &mut ToolboxConfig, variables: &HashMap<String, String>)
{
    for (_, command) in config.toolbox_commands.iter_mut() {
        for (_i, msg) in command.messages.iter_mut().enumerate() {
            let mut tmp = msg.content.clone();
            for (vname, vtext) in variables.iter() {
                tmp = tmp.replace(&format!("%{}%", vname), vtext);
            }
            msg.content = tmp;
        }
    }
}

fn _replace_variables_in_system_prompts(config: &mut ToolboxConfig, variables: &HashMap<String, String>)
{
    for (_, prompt) in config.system_prompts.iter_mut() {
        let mut tmp = prompt.text.clone();
        for (vname, vtext) in variables.iter() {
            tmp = tmp.replace(&format!("%{}%", vname), vtext);
        }
        prompt.text = tmp;
    }
}

fn _load_and_mix_with_users_config(user_yaml: &str) -> Result<ToolboxConfig, String> {
    let default_unstructured: serde_yaml::Value = serde_yaml::from_str(crate::toolbox::toolbox_compiled_in::COMPILED_IN_CUSTOMIZATION_YAML)
        .map_err(|e| format!("Error parsing default YAML: {}", e))?;
    let user_unstructured: serde_yaml::Value = serde_yaml::from_str(user_yaml)
        .map_err(|e| format!("Error parsing customization.yaml: {}", e))?;

    let mut variables: HashMap<String, String> = HashMap::new();
    _extract_mapping_values(&default_unstructured.as_mapping(), &mut variables);
    _extract_mapping_values(&user_unstructured.as_mapping(), &mut variables);

    let mut work_config: ToolboxConfig = serde_yaml::from_str(crate::toolbox::toolbox_compiled_in::COMPILED_IN_CUSTOMIZATION_YAML)
        .map_err(|e| format!("Error parsing default ToolboxConfig: {}", e))?;
    let mut user_config: ToolboxConfig = serde_yaml::from_str(user_yaml)
        .map_err(|e| format!("Error parsing user ToolboxConfig: {}", e))?;

    _replace_variables_in_messages(&mut work_config, &variables);
    _replace_variables_in_messages(&mut user_config, &variables);
    _replace_variables_in_system_prompts(&mut work_config, &variables);
    _replace_variables_in_system_prompts(&mut user_config, &variables);

    work_config.toolbox_commands.extend(user_config.toolbox_commands.iter().map(|(k, v)| (k.clone(), v.clone())));
    work_config.system_prompts.extend(user_config.system_prompts.iter().map(|(k, v)| (k.clone(), v.clone())));
    Ok(work_config)
}

pub fn load_customization_high_level(cache_dir: std::path::PathBuf) -> Result<ToolboxConfig, String> {
    let user_config_path = cache_dir.join("customization.yaml");

    if !user_config_path.exists() {
        let mut file = std::fs::File::create(&user_config_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        file.write_all(crate::toolbox::toolbox_compiled_in::COMPILED_IN_INITIAL_USER_YAML.as_bytes())
            .map_err(|e| format!("Failed to write to file: {}", e))?;

        let the_default = String::from(crate::toolbox::toolbox_compiled_in::COMPILED_IN_CUSTOMIZATION_YAML);
        for line in the_default.split('\n') {
            let mut comment = String::from("# ");
            comment.push_str(line);
            comment.push('\n');
            file.write_all(comment.as_bytes())
                .map_err(|e| format!("Failed to write to file: {}", e))?;
        }
    }

    let user_config_text = std::fs::read_to_string(&user_config_path).map_err(|e| format!("Failed to read file: {}", e))?;
    _load_and_mix_with_users_config(&user_config_text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_compiled_in_toolbox_valid_toml() {
        let _config = _load_and_mix_with_users_config(crate::toolbox::toolbox_compiled_in::COMPILED_IN_INITIAL_USER_YAML);
    }
}
