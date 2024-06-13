use serde_yaml;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tokio::sync::RwLock as ARwLock;
use crate::call_validation::ChatMessage;
use std::io::Write;
use std::sync::Arc;
use crate::global_context::{GlobalContext, try_load_caps_quickly_if_not_present};
use crate::at_tools::at_tools::{AtParamDict, make_openai_tool_value};


#[derive(Deserialize)]
pub struct ToolboxConfigDeserialize {
    #[serde(default)]
    pub system_prompts: HashMap<String, SystemPrompt>,
    #[serde(default)]
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

fn extract_mapping_values(mapping: &Option<&serde_yaml::Mapping>, variables: &mut HashMap<String, String>)
{
    if let Some(mapping) = mapping {
        for (k, v) in mapping.iter() {
            if let (serde_yaml::Value::String(key), serde_yaml::Value::String(value)) = (k, v) {
                variables.insert(key.clone(), value.clone());
            }
        }
    }
}

fn replace_variables_in_messages(config: &mut ToolboxConfig, variables: &HashMap<String, String>)
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

fn replace_variables_in_system_prompts(config: &mut ToolboxConfig, variables: &HashMap<String, String>)
{
    for (_, prompt) in config.system_prompts.iter_mut() {
        let mut tmp = prompt.text.clone();
        for (vname, vtext) in variables.iter() {
            tmp = tmp.replace(&format!("%{}%", vname), vtext);
        }
        prompt.text = tmp;
    }
}

fn load_and_mix_with_users_config(user_yaml: &str, caps_yaml: &str, caps_default_system_prompt: &str) -> Result<ToolboxConfig, String> {
    let default_unstructured: serde_yaml::Value = serde_yaml::from_str(crate::toolbox::toolbox_compiled_in::COMPILED_IN_CUSTOMIZATION_YAML)
        .map_err(|e| format!("Error parsing default YAML: {}\n{}", e, crate::toolbox::toolbox_compiled_in::COMPILED_IN_CUSTOMIZATION_YAML))?;
    let user_unstructured: serde_yaml::Value = serde_yaml::from_str(user_yaml)
        .map_err(|e| format!("Error parsing customization.yaml: {}\n{}", e, user_yaml))?;

    let mut variables: HashMap<String, String> = HashMap::new();
    extract_mapping_values(&default_unstructured.as_mapping(), &mut variables);
    extract_mapping_values(&user_unstructured.as_mapping(), &mut variables);

    let work_config_deserialize: ToolboxConfigDeserialize = serde_yaml::from_str(crate::toolbox::toolbox_compiled_in::COMPILED_IN_CUSTOMIZATION_YAML)
        .map_err(|e| format!("Error parsing default ToolboxConfig: {}\n{}", e, crate::toolbox::toolbox_compiled_in::COMPILED_IN_CUSTOMIZATION_YAML))?;
    let tools = work_config_deserialize.tools.iter()
        .map(|x|AtToolCustDict::new(x, &work_config_deserialize.tools_parameters))
        .collect::<Vec<AtToolCustDict>>();

    let mut work_config = ToolboxConfig {
        system_prompts: work_config_deserialize.system_prompts,
        toolbox_commands: work_config_deserialize.toolbox_commands,
        tools,
    };

    let user_config_deserialize: ToolboxConfigDeserialize = serde_yaml::from_str(user_yaml)
        .map_err(|e| format!("Error parsing user ToolboxConfig: {}\n{}", e, user_yaml))?;
    let user_tools = user_config_deserialize.tools.iter()
       .map(|x|AtToolCustDict::new(x, &user_config_deserialize.tools_parameters))
       .collect::<Vec<AtToolCustDict>>();

    let mut user_config = ToolboxConfig {
        system_prompts: user_config_deserialize.system_prompts,
        toolbox_commands: user_config_deserialize.toolbox_commands,
        tools: user_tools,
    };

    replace_variables_in_messages(&mut work_config, &variables);
    replace_variables_in_messages(&mut user_config, &variables);
    replace_variables_in_system_prompts(&mut work_config, &variables);
    replace_variables_in_system_prompts(&mut user_config, &variables);
    
    let caps_config_deserialize: ToolboxConfigDeserialize = serde_yaml::from_str(caps_yaml)
        .map_err(|e| format!("Error parsing default ToolboxConfig: {}\n{}", e, caps_yaml))?;
    let caps_config = ToolboxConfig {
        system_prompts: caps_config_deserialize.system_prompts,
        toolbox_commands: caps_config_deserialize.toolbox_commands,
        tools: vec![],
    };
    
    work_config.system_prompts.extend(caps_config.system_prompts.iter().map(|(k, v)| (k.clone(), v.clone())));
    work_config.toolbox_commands.extend(caps_config.toolbox_commands.iter().map(|(k, v)| (k.clone(), v.clone())));

    work_config.system_prompts.extend(user_config.system_prompts.iter().map(|(k, v)| (k.clone(), v.clone())));
    work_config.toolbox_commands.extend(user_config.toolbox_commands.iter().map(|(k, v)| (k.clone(), v.clone())));
    work_config.tools.extend(user_config.tools.iter().map(|x|x.clone()));
    
    if !caps_default_system_prompt.is_empty() && work_config.system_prompts.get(caps_default_system_prompt).is_some() {
        work_config.system_prompts.insert("default".to_string(), work_config.system_prompts.get(caps_default_system_prompt).map(|x|x.clone()).unwrap());
    }
    
    Ok(work_config)
}

pub async fn load_customization(gcx: Arc<ARwLock<GlobalContext>>) -> Result<ToolboxConfig, String> {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let caps = try_load_caps_quickly_if_not_present(gcx, 0).await.map_err(|e|format!("error loading caps: {e}"))?;
    
    let (caps_config_text, caps_default_system_prompt) = {
        let caps_locked = caps.read().unwrap();
        (caps_locked.customization.clone(), caps_locked.code_chat_default_system_prompt.clone())
    };
    
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
    load_and_mix_with_users_config(&user_config_text, &caps_config_text, &caps_default_system_prompt).map_err(|e| e.to_string())
}

pub async fn get_default_system_prompt(global_context: Arc<ARwLock<GlobalContext>>) -> Result<String, String> {
    let tconfig = load_customization(global_context.clone()).await?;
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
        let _config = load_and_mix_with_users_config(crate::toolbox::toolbox_compiled_in::COMPILED_IN_INITIAL_USER_YAML, "", "");
    }
}
