use serde_yaml;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tokio::sync::RwLock as ARwLock;
use crate::call_validation::ChatMessage;
use std::io::Write;
use std::sync::Arc;
use crate::global_context::GlobalContext;
use crate::at_tools::at_tools::{AtParamDict, make_openai_tool_value};


#[derive(Deserialize)]
pub struct ToolboxConfigDeserialize {
    pub system_prompts: HashMap<String, SystemPrompt>,
    pub toolbox_commands: HashMap<String, ToolboxCommand>,
    #[serde(default)]
    pub tools: Vec<AtToolCustDictDeserialize>,
    #[serde(default)]
    pub tools_parameters: Vec<AtParamDict>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolboxConfig {
    pub system_prompts: HashMap<String, SystemPrompt>,
    pub toolbox_commands: HashMap<String, ToolboxCommand>,
    pub tools: Vec<AtToolCustDict>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AtToolCustDict {
    pub name: String,
    pub description: String,
    pub parameters: Vec<AtParamDict>,
    pub parameters_required: Vec<String>,
    pub command: String,
    pub timeout: usize,
    pub postprocess: String,
}

impl AtToolCustDict {
    pub fn new(cmd: &AtToolCustDictDeserialize, params: &Vec<AtParamDict>) -> Self {
        AtToolCustDict {
            name: cmd.name.clone(),
            description: cmd.description.clone(),
            parameters: cmd.parameters.iter()
                .map(
                    |name| params.iter()
                        .find(|param| &param.name == name).unwrap()
                )
                .cloned().collect(),
            parameters_required: cmd.parameters_required.clone(),
            command: cmd.command.clone(),
            timeout: cmd.timeout,
            postprocess: cmd.postprocess.clone(),
        }
    }

    pub fn into_openai_style(self) -> serde_json::Value {
        make_openai_tool_value(
            self.name,
            self.description,
            self.parameters_required,
            self.parameters,
        )
    }
}

#[derive(Debug, Deserialize)]
pub struct AtToolCustDictDeserialize{
    pub name: String,
    pub description: String,
    pub parameters: Vec<String>,
    pub parameters_required: Vec<String>,
    pub command: String,
    pub timeout: usize,
    pub postprocess: String,
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
            if let (serde_yaml::Value::String(key), serde_yaml::Value::String(value)) = (k, v) {
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

    let work_config_deserialize: ToolboxConfigDeserialize = serde_yaml::from_str(crate::toolbox::toolbox_compiled_in::COMPILED_IN_CUSTOMIZATION_YAML)
        .map_err(|e| format!("Error parsing default ToolboxConfig: {}", e))?;
    let tools = work_config_deserialize.tools.iter()
        .map(|x|AtToolCustDict::new(x, &work_config_deserialize.tools_parameters))
        .collect::<Vec<AtToolCustDict>>();

    let mut work_config = ToolboxConfig {
        system_prompts: work_config_deserialize.system_prompts,
        toolbox_commands: work_config_deserialize.toolbox_commands,
        tools,
    };

    let user_config_deserialize: ToolboxConfigDeserialize = serde_yaml::from_str(user_yaml)
        .map_err(|e| format!("Error parsing user ToolboxConfig: {}", e))?;
    let user_tools = user_config_deserialize.tools.iter()
       .map(|x|AtToolCustDict::new(x, &user_config_deserialize.tools_parameters))
       .collect::<Vec<AtToolCustDict>>();

    let mut user_config = ToolboxConfig {
        system_prompts: user_config_deserialize.system_prompts,
        toolbox_commands: user_config_deserialize.toolbox_commands,
        tools: user_tools,
    };

    _replace_variables_in_messages(&mut work_config, &variables);
    _replace_variables_in_messages(&mut user_config, &variables);
    _replace_variables_in_system_prompts(&mut work_config, &variables);
    _replace_variables_in_system_prompts(&mut user_config, &variables);

    work_config.toolbox_commands.extend(user_config.toolbox_commands.iter().map(|(k, v)| (k.clone(), v.clone())));
    work_config.system_prompts.extend(user_config.system_prompts.iter().map(|(k, v)| (k.clone(), v.clone())));
    // TODO: deduplicate?
    work_config.tools.extend(user_config.tools.iter().map(|x|x.clone()));
    Ok(work_config)
}

pub async fn load_customization_high_level(gcx: Arc<ARwLock<GlobalContext>>) -> Result<ToolboxConfig, String> {
    let cache_dir = gcx.read().await.cache_dir.clone();
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

pub async fn get_default_system_prompt(global_context: Arc<ARwLock<GlobalContext>>) -> Result<String, String> {
    let tconfig = load_customization_high_level(global_context.clone()).await?;
    match tconfig.system_prompts.get("default").and_then(|x|Some(x.text.clone())) {
        Some(x) => Ok(x),
        None => Err("no default system prompt found".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_compiled_in_toolbox_valid_toml() {
        let _config = _load_and_mix_with_users_config(crate::toolbox::toolbox_compiled_in::COMPILED_IN_INITIAL_USER_YAML);
    }
}
