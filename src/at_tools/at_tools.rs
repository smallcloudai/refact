use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::ContextEnum;
use crate::global_context::GlobalContext;


#[async_trait]
pub trait AtTool: Send + Sync {
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String>;
    fn depends_on(&self) -> Vec<String> { vec![] }   // "ast", "vecdb"
}

pub async fn at_tools_merged(gcx: Arc<ARwLock<GlobalContext>>) -> HashMap<String, Arc<AMutex<Box<dyn AtTool + Send>>>>
{
    let mut result =  HashMap::from([
        ("workspace".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_workspace::AttWorkspace{}) as Box<dyn AtTool + Send>))),
        ("file".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_file::AttFile{}) as Box<dyn AtTool + Send>))),
        ("definition".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_ast_definition::AttAstDefinition{}) as Box<dyn AtTool + Send>))),
        ("references".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_ast_reference::AttAstReference{}) as Box<dyn AtTool + Send>))),
        // ("symbols_at".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_ast_lookup_symbols::AttAstLookupSymbols{}) as Box<dyn AtTool + Send>))),
        ("note_to_self".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_note_to_self::AtNoteToSelf{}) as Box<dyn AtTool + Send>))),
    ]);

    let tconfig_maybe = crate::toolbox::toolbox_config::load_customization_high_level(gcx.clone()).await;
    if tconfig_maybe.is_err() {
        tracing::error!("Error loading toolbox config: {:?}", tconfig_maybe.err().unwrap());
    } else {
        for cust in tconfig_maybe.unwrap().tools {
            result.insert(
                cust.name.clone(),
                Arc::new(AMutex::new(Box::new(crate::at_tools::att_execute_cmd::AttExecuteCommand {
                    command: cust.command,
                    timeout: cust.timeout,
                    postprocess: cust.postprocess,
                }) as Box<dyn AtTool + Send>)));
        }
    }

    result
}

const AT_DICT: &str = r####"
tools:
  - name: "workspace"
    description: "Using given query, find all pieces of code in the project by vectorizing query and finding all similar pieces of code by comparing their cosien distances."
    parameters:
      - name: "query"
        type: "string"
        description: "Short text written in natural language. Single line or a paragraph. Query will be vectorized and used to find similar pieces of code in the project."
    parameters_required:
      - "query"
  - name: "file"
    description: "Read the file located using given file_path and provide its content"
    parameters:
      - name: "file_path"
        type: "string"
        description: "absolute path to the file or filename to be found within the project."
    parameters_required:
      - "file_path"
  - name: "definition"
    description: "Find definition of a symbol in a project using AST. Symbol could be: function, method, class, type alias."
    parameters:
      - name: "symbol"
        type: "string"
        description: "The name of the symbol (function, method, class, type alias) to find within the project."
    parameters_required:
      - "symbol"
  - name: "references"
    description: "Find usages of a symbol in a project using AST. Symbol could be: function, method, class, type alias."
    parameters:
      - name: "symbol"
        type: "string"
        description: "The name of the symbol (function, method, class, type alias) to find within the project."
    parameters_required:
      - "symbol"
  - name: "note_to_self"
    description: "Tool to save notes into memory after user correction"
    parameters:
      - name: "text"
        type: "string"
        description: "How to behave better next time?"
    parameters_required:
      - "text"

"####;


#[derive(Deserialize)]
pub struct AtDictDeserialize {
    pub tools: Vec<AtToolDict>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AtToolDict {
    pub name: String,
    pub description: String,
    pub parameters: Vec<AtParamDict>,
    pub parameters_required: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AtParamDict {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub description: String,
}

pub fn make_openai_tool_value(
    name: String,
    description: String,
    parameters_required: Vec<String>,
    parameters: Vec<AtParamDict>,
) -> serde_json::Value {
    let params_properties = parameters.iter().map(|param| {
        (
            param.name.clone(),
            serde_json::json!({
                "type": param.param_type,
                "description": param.description
            })
        )
    }).collect::<serde_json::Map<_, _>>();

    let function_json = serde_json::json!({
            "type": "function",
            "function": {
                "name": name,
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

impl AtToolDict {
    pub fn into_openai_style(self) -> serde_json::Value {
        make_openai_tool_value(
            self.name,
            self.description,
            self.parameters_required,
            self.parameters,
        )
    }
}

pub fn at_tools_compiled_in_only() -> Result<Vec<AtToolDict>, String> {
    let at_dict: AtDictDeserialize = serde_yaml::from_str(AT_DICT)
        .map_err(|e|format!("Failed to parse AT_DICT: {}", e))?;

    Ok(at_dict.tools)
}
