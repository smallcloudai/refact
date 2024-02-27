use serde_yaml::Value;
use serde_yaml;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::call_validation::ChatMessage;


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


fn _extract_mapping_values(mapping: &serde_yaml::Value, variables: &mut HashMap<String, String>) {
    if let Some(mapping) = mapping.as_mapping() {
        for (k, v) in mapping {
            if let (Value::String(key), Value::String(value)) = (k, v) {
                variables.insert(key.clone(), value.clone());
            }
        }
    }
}

fn _replace_variables_in_messages(config: &mut ToolboxConfig, variables: &HashMap<String, String>) {
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

pub fn load_and_mix_with_users_config() -> ToolboxConfig {
    let default_unstructured: serde_yaml::Value = serde_yaml::from_str(crate::toolbox::toolbox_compiled_in::COMPILED_IN_CUSTOMIZATION_YAML).unwrap();
    let user_unstructured: serde_yaml::Value = serde_yaml::from_str(crate::toolbox::toolbox_compiled_in::COMPILED_IN_INITIAL_USER_YAML).unwrap();
    let mut variables = HashMap::<String, String>::new();
    _extract_mapping_values(&default_unstructured, &mut variables);
    _extract_mapping_values(&user_unstructured, &mut variables);
    let mut work_config: ToolboxConfig = serde_yaml::from_str(crate::toolbox::toolbox_compiled_in::COMPILED_IN_CUSTOMIZATION_YAML).unwrap();
    let mut user_config: ToolboxConfig = serde_yaml::from_str(crate::toolbox::toolbox_compiled_in::COMPILED_IN_INITIAL_USER_YAML).unwrap();
    _replace_variables_in_messages(&mut work_config, &variables);
    _replace_variables_in_messages(&mut user_config, &variables);
    work_config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_compiled_in_toolbox_valid_toml() {
        let _yaml: serde_yaml::Value = serde_yaml::from_str(crate::toolbox::toolbox_compiled_in::COMPILED_IN_CUSTOMIZATION_YAML).unwrap();
    }

    #[test]
    fn does_compiled_in_toolbox_fit_structs() {
        let _yaml: ToolboxConfig = serde_yaml::from_str(crate::toolbox::toolbox_compiled_in::COMPILED_IN_CUSTOMIZATION_YAML).unwrap();
    }
}
