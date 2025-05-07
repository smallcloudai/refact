use std::collections::HashMap;
use std::sync::Arc;
use indexmap::IndexMap;
use serde_json::{Value, json};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatUsage, ContextEnum};
use crate::global_context::try_load_caps_quickly_if_not_present;
use crate::global_context::GlobalContext;
use crate::integrations::integr_abstract::IntegrationConfirmation;
use crate::tools::tools_execute::{command_should_be_confirmed_by_user, command_should_be_denied};
// use crate::integrations::docker::integr_docker::ToolDocker;


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

#[async_trait]
pub trait Tool: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
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

    fn tool_name(&self) -> String  {
        return "".to_string();
    }
}

pub async fn tools_merged_and_filtered(
    gcx: Arc<ARwLock<GlobalContext>>,
    _supports_clicks: bool,  // XXX
) -> Result<IndexMap<String, Box<dyn Tool + Send>>, String> {
    let (ast_on, vecdb_on, allow_experimental) = {
        let gcx_locked = gcx.read().await;
        let vecdb_on = gcx_locked.vec_db.lock().await.is_some();
        (gcx_locked.ast_service.is_some(), vecdb_on, gcx_locked.cmdline.experimental)
    };

    let mut tools_all = IndexMap::from([
        ("search_symbol_definition".to_string(), Box::new(crate::tools::tool_ast_definition::ToolAstDefinition{}) as Box<dyn Tool + Send>),
        ("search_symbol_usages".to_string(), Box::new(crate::tools::tool_ast_reference::ToolAstReference{}) as Box<dyn Tool + Send>),
        ("tree".to_string(), Box::new(crate::tools::tool_tree::ToolTree{}) as Box<dyn Tool + Send>),
        ("create_textdoc".to_string(), Box::new(crate::tools::file_edit::tool_create_textdoc::ToolCreateTextDoc{}) as Box<dyn Tool + Send>),
        ("update_textdoc".to_string(), Box::new(crate::tools::file_edit::tool_update_textdoc::ToolUpdateTextDoc {}) as Box<dyn Tool + Send>),
        ("update_textdoc_regex".to_string(), Box::new(crate::tools::file_edit::tool_update_textdoc_regex::ToolUpdateTextDocRegex {}) as Box<dyn Tool + Send>),
        ("web".to_string(), Box::new(crate::tools::tool_web::ToolWeb{}) as Box<dyn Tool + Send>),
        ("cat".to_string(), Box::new(crate::tools::tool_cat::ToolCat{}) as Box<dyn Tool + Send>),
        ("rm".to_string(), Box::new(crate::tools::tool_rm::ToolRm{}) as Box<dyn Tool + Send>),
        ("mv".to_string(), Box::new(crate::tools::tool_mv::ToolMv{}) as Box<dyn Tool + Send>),
        ("strategic_planning".to_string(), Box::new(crate::tools::tool_strategic_planning::ToolStrategicPlanning{}) as Box<dyn Tool + Send>),
        ("search_pattern".to_string(), Box::new(crate::tools::tool_regex_search::ToolRegexSearch{}) as Box<dyn Tool + Send>),
        ("knowledge".to_string(), Box::new(crate::tools::tool_knowledge::ToolGetKnowledge{}) as Box<dyn Tool + Send>),
        ("create_knowledge".to_string(), Box::new(crate::tools::tool_create_knowledge::ToolCreateKnowledge{}) as Box<dyn Tool + Send>),
        ("create_memory_bank".to_string(), Box::new(crate::tools::tool_create_memory_bank::ToolCreateMemoryBank{}) as Box<dyn Tool + Send>),
        ("search_semantic".to_string(), Box::new(crate::tools::tool_search::ToolSearch{}) as Box<dyn Tool + Send>),
        ("locate".to_string(), Box::new(crate::tools::tool_locate_search::ToolLocateSearch{}) as Box<dyn Tool + Send>),
    ]);

    let integrations = crate::integrations::running_integrations::load_integration_tools(
        gcx.clone(),
        allow_experimental,
    ).await;
    tools_all.extend(integrations);

    let (is_there_a_thinking_model, allow_knowledge) = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => {
            (caps.chat_models.get(&caps.defaults.chat_thinking_model).is_some(),
             caps.metadata.features.contains(&"knowledge".to_string()))
        },
        Err(_) => (false, false),
    };

    let mut filtered_tools = IndexMap::new();
    for (tool_name, tool) in tools_all {
        let dependencies = tool.tool_depends_on();
        if dependencies.contains(&"ast".to_string()) && !ast_on {
            continue;
        }
        if dependencies.contains(&"vecdb".to_string()) && !vecdb_on {
            continue;
        }
        if dependencies.contains(&"thinking".to_string()) && !is_there_a_thinking_model {
            continue;
        }
        if dependencies.contains(&"knowledge".to_string()) && !allow_knowledge {
            continue;
        }
        filtered_tools.insert(tool_name, tool);
    }

    Ok(filtered_tools)
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
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolParam {
    #[serde(deserialize_with = "validate_snake_case")]
    pub name: String,
    #[serde(rename = "type", default = "default_param_type")]
    pub param_type: String,
    pub description: String,
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

#[derive(Deserialize)]
pub struct ToolDictDeserialize {
    pub tools: Vec<ToolDesc>,
}

pub async fn tool_description_list_from_yaml(
    tools: IndexMap<String, Box<dyn Tool + Send>>,
    turned_on: Option<&Vec<String>>,
    allow_experimental: bool,
) -> Result<Vec<ToolDesc>, String> {
    let tool_desc_deser: ToolDictDeserialize = serde_yaml::from_str(BUILT_IN_TOOLS)
        .map_err(|e|format!("Failed to parse BUILT_IN_TOOLS: {}", e))?;

    let mut tool_desc_vec = vec![];
    tool_desc_vec.extend(tool_desc_deser.tools.iter().cloned());

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

const BUILT_IN_TOOLS: &str = r####"
tools:
  - name: "search_semantic"
    description: "Find semantically similar pieces of code or text using vector database (semantic search)"
    parameters:
      - name: "queries"
        type: "string"
        description: "Comma-separated list of queries. Each query can be a single line, paragraph or code sample to search for semantically similar content."
      - name: "scope"
        type: "string"
        description: "'workspace' to search all files in workspace, 'dir/subdir/' to search in files within a directory, 'dir/file.ext' to search in a single file."
    parameters_required:
      - "queries"
      - "scope"
"####;



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
