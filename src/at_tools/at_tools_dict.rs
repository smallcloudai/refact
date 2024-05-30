use serde::{Deserialize, Serialize};


const AT_DICT: &str = r####"
tools:
  - name: "workspace"
    description: "Using given query, find all pieces of code in the project by vectorizing query and finding all similar pieces of code by comparing their cosien distances."
    parameters:
      - "query"
    parameters_required:
      - "query"
  - name: "file"
    description: "Read the file located using given file_path and provide its content"
    parameters:
      - "file_path"
    parameters_required:
      - "file_path"
  - name: "definition"
    description: "Find definition of a symbol in a project using AST. Symbol could be: function, method, class, type alias."
    parameters:
      - "symbol"
    parameters_required:
      - "symbol"
  - name: "references"
    description: "Find usages of a symbol in a project using AST. Symbol could be: function, method, class, type alias."
    parameters:
      - "symbol"
    parameters_required:
      - "symbol"
  - name: "symbols-at"
    description: "Using a file_path and a line_number, find all symbols at the given line number of the file."
    parameters:
      - "file_path"
      - "line_number"
    parameters_required:
      - "file_path"
      - "line_number"

parameters:
  - name: "query"
    type: "string"
    description: "Short text written in natural language. Single line or a paragraph. Query will be vectorized and used to find similar pieces of code in the project."
  - name: "file_path"
    type: "string"
    description: "absolute path to the file or filename to be found within the project."
  - name: "symbol"
    type: "string"
    description: "The name of the symbol (function, method, class, type alias) to find within the project."
  - name: "line_number"
    type: "integer"
    description: "The line number of the file"
"####;


#[derive(Deserialize)]
pub struct AtDictDeserialize {
    pub tools: Vec<AtToolDictDeserialize>,
    pub parameters: Vec<AtParamDict>,
}

#[derive(Deserialize)]
pub struct AtToolDictDeserialize{
    pub name: String,
    pub description: String,
    pub parameters: Vec<String>,
    pub parameters_required: Vec<String>,
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
    pub fn new(cmd: &AtToolDictDeserialize, params: &Vec<AtParamDict>) -> Self {
        AtToolDict {
            name: cmd.name.clone(),
            description: cmd.description.clone(),
            parameters: cmd.parameters.iter()
                .map(
                    |name| params.iter()
                        .find(|param| &param.name == name).unwrap()
                )
                .cloned().collect(),
            parameters_required: cmd.parameters_required.clone(),
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
    
pub fn at_tools_dicts() -> Result<Vec<AtToolDict>, String> {
    let at_dict: AtDictDeserialize = serde_yaml::from_str(AT_DICT)
        .map_err(|e|format!("Failed to parse AT_DICT: {}", e))?;

    let at_command_dicts = at_dict.tools.iter()
        .map(|x| AtToolDict::new(x, &at_dict.parameters))
        .collect::<Vec<AtToolDict>>();

    Ok(at_command_dicts)
}
