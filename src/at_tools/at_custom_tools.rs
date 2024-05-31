use serde::{Deserialize, Serialize};
use crate::at_tools::at_tools_dict::{AtParamDict, make_openai_tool_value};


const AT_CUSTOM_TOOLS_DICT: &str = r####"
tools:
  - name: "compile"
    description: "Compile the project"
    parameters:
    parameters_required:
    command: "cd /Users/$USER/code/refact-lsp && cargo build"
    timeout: 300
    postprocess: "last_100_lines"

parameters:

"####;


#[derive(Deserialize)]
pub struct AtToolCustDictDeserialize{
    pub name: String,
    pub description: String,
    pub parameters: Vec<String>,
    pub parameters_required: Vec<String>,
    pub command: String,
    pub timeout: usize,
    pub postprocess: String,
}

#[derive(Deserialize)]
pub struct AtCustDictDeserialize {
    pub tools: Vec<AtToolCustDictDeserialize>,
    pub parameters: Vec<AtParamDict>,
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

pub fn at_custom_tools_dicts() -> Result<Vec<AtToolCustDict>, String> {
    let at_cust_dict: AtCustDictDeserialize = serde_yaml::from_str(AT_CUSTOM_TOOLS_DICT)
        .map_err(|e|format!("Failed to parse AT_CUSTOM_TOOLS_DICT: {}", e))?;

    let at_cust_command_dicts = at_cust_dict.tools.iter()
        .map(|x|AtToolCustDict::new(x, &at_cust_dict.parameters))
       .collect::<Vec<AtToolCustDict>>();

    Ok(at_cust_command_dicts)
}
