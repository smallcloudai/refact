use std::collections::HashMap;
use std::sync::Arc;
use indexmap::IndexMap;
use serde_json::{Value, json};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatUsage, ContextEnum};
use crate::integrations::integr_abstract::IntegrationConfirmation;
use crate::tools::tools_execute::{command_should_be_confirmed_by_user, command_should_be_denied};

#[derive(Clone, Debug)]
pub enum MatchConfirmDenyResult {
    PASS,
    CONFIRMATION,
    DENY,
}

#[derive(Clone, Debug)]
pub struct MatchConfirmDeny {
    pub result: MatchConfirmDenyResult,
    pub command: String,
    pub rule: String,
}

pub enum ToolGroupCategory {
    Builtin,
    Integration,
    MCP,
}

pub struct ToolGroup {
    pub name: String,
    pub description: String,
    pub category: ToolGroupCategory,
    pub tools: Vec<Box<dyn Tool + Send>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ToolSourceType {
    Builtin,
    Integration,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolSource {
    pub source_type: ToolSourceType,
    pub config_path: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolDesc {
    pub name: String,
    #[serde(default)]
    pub agentic: bool,
    #[serde(default)]
    pub experimental: bool,
    pub description: String,
    pub parameters: Vec<ToolParam>,
    pub parameters_required: Vec<String>,
    pub display_name: String,
    pub source: ToolSource,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolParam {
    #[serde(deserialize_with = "validate_snake_case")]
    pub name: String,
    #[serde(rename = "type", default = "default_param_type")]
    pub param_type: String,
    pub description: String,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String>;

    fn tool_description(&self) -> ToolDesc;

    async fn match_against_confirm_deny(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>
    ) -> Result<MatchConfirmDeny, String> {
        let command_to_match = self.command_to_match_against_confirm_deny(ccx.clone(), &args).await.map_err(|e| {
            format!("Error getting tool command to match: {}", e)
        })?;

        if !command_to_match.is_empty() {
            if let Some(rules) = &self.confirm_deny_rules() {
                tracing::info!("confirmation: match {:?} against {:?}", command_to_match, rules);
                let (is_denied, deny_rule) = command_should_be_denied(&command_to_match, &rules.deny);
                if is_denied {
                    return Ok(MatchConfirmDeny {
                        result: MatchConfirmDenyResult::DENY,
                        command: command_to_match.clone(),
                        rule: deny_rule.clone(),
                    });
                }
                let (needs_confirmation, confirmation_rule) = command_should_be_confirmed_by_user(&command_to_match, &rules.ask_user);
                if needs_confirmation {
                    return Ok(MatchConfirmDeny {
                        result: MatchConfirmDenyResult::CONFIRMATION,
                        command: command_to_match.clone(),
                        rule: confirmation_rule.clone(),
                    });
                }
            } else {
                tracing::error!("No confirmation info available for {:?}", command_to_match);
            }
        }
        Ok(MatchConfirmDeny {
            result: MatchConfirmDenyResult::PASS,
            command: command_to_match.clone(),
            rule: "".to_string(),
        })
    }

    async fn command_to_match_against_confirm_deny(
        &self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        _args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        Ok("".to_string())
    }

    fn confirm_deny_rules(
        &self,
    ) -> Option<IntegrationConfirmation> {
        None
    }

    fn has_config_path(&self) -> Option<String> {
        return None;
    }

    fn tool_depends_on(&self) -> Vec<String> { vec![] }   // "ast", "vecdb"

    fn usage(&mut self) -> &mut Option<ChatUsage> {
        static mut DEFAULT_USAGE: Option<ChatUsage> = None;
        #[allow(static_mut_refs)]
        unsafe { &mut DEFAULT_USAGE }
    }
}

fn validate_snake_case<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if !s.chars().next().map_or(false, |c| c.is_ascii_lowercase())
        || !s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        || s.contains("__")
        || s.ends_with('_')
    {
        return Err(serde::de::Error::custom(
            format!("name {:?} must be in snake_case format: lowercase letters, numbers and single underscores, must start with letter", s)
        ));
    }
    Ok(s)
}

fn default_param_type() -> String {
    "string".to_string()
}

/// TODO: Think a better way to know if we can send array type to the model
/// 
/// For now, anthropic models support it, gpt models don't, for other, we'll need to test
pub fn model_supports_array_param_type(model_id: &str) -> bool {
    model_id.contains("claude")
}

pub fn make_openai_tool_value(
    name: String,
    agentic: bool,
    description: String,
    parameters_required: Vec<String>,
    parameters: Vec<ToolParam>,
) -> Value {
    let params_properties = parameters.iter().map(|param| {
        (
            param.name.clone(),
            json!({
                "type": param.param_type,
                "description": param.description
            })
        )
    }).collect::<serde_json::Map<_, _>>();

    let function_json = json!({
        "type": "function",
        "function": {
            "name": name,
            "agentic": agentic, // this field is not OpenAI's
            "description": description,
            "parameters": {
                "type": "object",
                "properties": params_properties,
                "required": parameters_required
            }
        }
    });
    function_json
}

impl ToolDesc {
    pub fn into_openai_style(self) -> Value {
        make_openai_tool_value(
            self.name,
            self.agentic,
            self.description,
            self.parameters_required,
            self.parameters,
        )
    }

    pub fn is_supported_by(&self, model: &str) -> bool {
        if !model_supports_array_param_type(model) {
            for param in &self.parameters {
                if param.param_type == "array" {
                    tracing::warn!("Tool {} has array parameter, but model {} does not support it", self.name, model);
                    return false;
                }
            }
        }
        true
    }
}

pub async fn tool_description_list_from_yaml(
    tools: IndexMap<String, Box<dyn Tool + Send>>,
    turned_on: Option<&Vec<String>>,
    allow_experimental: bool,
) -> Result<Vec<ToolDesc>, String> {
    let mut tool_desc_vec: Vec<ToolDesc> = vec![];

    for (tool_name, tool) in tools {
        if !tool_desc_vec.iter().any(|desc| desc.name == tool_name) {
            tool_desc_vec.push(tool.tool_description());
        }
    }

    Ok(tool_desc_vec.iter()
        .filter(|x| {
            turned_on.map_or(true, |turned_on_vec| turned_on_vec.contains(&x.name)) &&
            (allow_experimental || !x.experimental)
        })
        .cloned()
        .collect::<Vec<_>>())
}

#[allow(dead_code)]
const NOT_READY_TOOLS: &str = r####"
  - name: "diff"
    description: "Perform a diff operation. Can be used to get git diff for a project (no arguments) or git diff for a specific file (file_path)"
    parameters:
      - name: "file_path"
        type: "string"
        description: "Path to the specific file to diff (optional)."
    parameters_required:
"####;
