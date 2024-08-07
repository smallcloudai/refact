use std::collections::HashMap;
use std::sync::Arc;
use serde_json::{Value, json};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatUsage, ContextEnum};
use crate::global_context::GlobalContext;
use crate::toolbox::toolbox_config::ToolCustDict;


#[async_trait]
pub trait Tool: Send + Sync {
    async fn tool_execute(&mut self, ccx: Arc<AMutex<AtCommandsContext>>, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String>;
    fn tool_depends_on(&self) -> Vec<String> { vec![] }   // "ast", "vecdb"
    fn usage(&mut self) -> &mut Option<ChatUsage> {
        static mut DEFAULT_USAGE: Option<ChatUsage> = None;
        #[allow(static_mut_refs)]
        unsafe { &mut DEFAULT_USAGE }
    }
}

pub async fn at_tools_merged_and_filtered(gcx: Arc<ARwLock<GlobalContext>>) -> HashMap<String, Arc<AMutex<Box<dyn Tool + Send>>>>
{
    let tools_all =  HashMap::from([
        ("search".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_search::AttSearch{}) as Box<dyn Tool + Send>))),
        ("file".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_file::AttFile{}) as Box<dyn Tool + Send>))),
        ("definition".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_ast_definition::AttAstDefinition{}) as Box<dyn Tool + Send>))),
        ("references".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_ast_reference::AttAstReference{}) as Box<dyn Tool + Send>))),
        ("tree".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_tree::AttTree{}) as Box<dyn Tool + Send>))),
        // ("symbols_at".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_ast_lookup_symbols::AttAstLookupSymbols{}) as Box<dyn AtTool + Send>))),
        // ("remember_how_to_use_tools".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_note_to_self::AtNoteToSelf{}) as Box<dyn AtTool + Send>))),
        // ("memorize_if_user_asks".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_note_to_self::AtNoteToSelf{}) as Box<dyn AtTool + Send>))),
        ("patch".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_patch::tool::ToolPatch::new()) as Box<dyn Tool + Send>))),
        // ("save_knowledge".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_knowledge::AttSaveKnowledge{}) as Box<dyn Tool + Send>))),
        ("knowledge".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_knowledge::AttGetKnowledge{}) as Box<dyn Tool + Send>))),
        // ("diff".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_diff::AttDiff{}) as Box<dyn Tool + Send>))),
        ("web".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_web::AttWeb{}) as Box<dyn Tool + Send>))),
        ("files_skeleton".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_files_skeleton::AttFilesSkeleton{}) as Box<dyn Tool + Send>))),
        ("relevant_files".to_string(), Arc::new(AMutex::new(Box::new(crate::at_tools::att_relevant_files::AttRelevantFiles{}) as Box<dyn Tool + Send>))),
    ]);

    let (ast_on, vecdb_on) = {
        let gcx = gcx.read().await;
        let vecdb = gcx.vec_db.lock().await;
        (gcx.ast_module.is_some(), vecdb.is_some())
    };

    let mut result = HashMap::new();
    for (key, value) in tools_all {
        let command = value.lock().await;
        let depends_on = command.tool_depends_on();
        if depends_on.contains(&"ast".to_string()) && !ast_on {
            continue;
        }
        if depends_on.contains(&"vecdb".to_string()) && !vecdb_on {
            continue;
        }
        result.insert(key, value.clone());
    }

    let tconfig_maybe = crate::toolbox::toolbox_config::load_customization(gcx.clone()).await;
    if tconfig_maybe.is_err() {
        tracing::error!("Error loading toolbox config: {:?}", tconfig_maybe.err().unwrap());
    } else {
        for cust in tconfig_maybe.unwrap().tools {
            result.insert(
                cust.name.clone(),
                Arc::new(AMutex::new(Box::new(crate::at_tools::att_execute_cmd::AttExecuteCommand {
                    command: cust.command,
                    timeout: cust.timeout,
                    output_postprocess: cust.output_postprocess,
                }) as Box<dyn Tool + Send>)));
        }
    }

    result
}

const TOOLS: &str = r####"
tools:
  - name: "search"
    description: "Find similar pieces of code or text using vector database"
    parameters:
      - name: "query"
        type: "string"
        description: "Single line, paragraph or code sample to search for similar content."
      - name: "scope"
        type: "string"
        description: "'workspace' to search all files in workspace, 'dir/subdir/' to search in files within a directory, 'dir/file.ext' to search in a single file."
    parameters_required:
      - "query"
      - "scope"

  - name: "file"
    description: "Read the file, the same as cat shell command, but skeletonizes files that are too large. Doesn't work on dirs."
    parameters:
      - name: "path"
        type: "string"
        description: "Either absolute path or preceeding_dirs/file.ext"
    parameters_required:
      - "path"

  - name: "definition"
    description: "Read definition of a symbol in the project using AST"
    parameters:
      - name: "symbol"
        type: "string"
        description: "The exact name of a function, method, class, type alias. No spaces allowed."
    parameters_required:
      - "symbol"

  - name: "references"
    description: "Find usages of a symbol within a project using AST"
    parameters:
      - name: "symbol"
        type: "string"
        description: "The exact name of a function, method, class, type alias. No spaces allowed."
    parameters_required:
      - "symbol"

  - name: "patch"
    description: "A tool to fix, edit a bunch of existing source files. This tool cannot rename, create or delete files"
    parameters:
      - name: "paths"
        type: "string"
        description: "A single string that contains all necessary to edit file names separated by commas. Use absolute file paths. Do not pass list of strings!"
      - name: "todo"
        type: "string"
        description: "A complete and usefull description of required changes for the given files"
      - name: "symbols"
        type: "string"
        description: "An optional argument, symbol names that might be usefull to make necessary changes"
    parameters_required:
      - "paths"
      - "todo"

  - name: "tree"
    description: "Get a files tree with symbols for the project. Use it to get familiar with the project, file names and symbols"
    parameters:
      - name: "path"
        type: "string"
        description: "An optional absolute path to get files tree for a particular folder or file. Do not pass it if you need full project tree."
      - name: "use_ast"
        type: "boolean"
        description: "if true, for each file an array of AST symbols will appear as well as its filename"
    parameters_required: []

  - name: "web"
    description: "Fetch a web page and convert to readable plain text."
    parameters:
      - name: "url"
        type: "string"
        description: "URL of the web page to fetch."
    parameters_required:
      - "url"

  - name: "knowledge"
    description: "What kind of knowledge you will need to accomplish this task? Call each time you have a new task or topic."
    parameters:
      - name: "im_going_to_do"
        type: "string"
        description: "Put your intent there: 'debug file1.cpp', 'install project1', 'gather info about MyClass'"
    parameters_required:
      - "im_going_to_do"

  - name: "relevant_files"
    description: "Get a list of files that are relevant to solve a particular task."
    parameters:
    parameters_required:
"####;

#[allow(dead_code)]
const NOT_READY_TOOLS: &str = r####"
  - name: "files_skeleton"
    description: "Collects limited files context with AST"
    parameters:
      - name: "paths"
        type: "string"
        description: "String that contains list of file names separated by commas. Use absolute file paths."
    parameters_required:
      - "paths"

  - name: "diff"
    description: "Perform a diff operation. Can be used to get git diff for a project (no arguments) or git diff for a specific file (file_path)"
    parameters:
      - name: "file_path"
        type: "string"
        description: "Path to the specific file to diff (optional)."
    parameters_required:
"####;


// - name: "save_knowledge"
// description: "Use it when you see something you'd want to remember about user, project or your experience for your future self."
// parameters:
//   - name: "memory_topic"
//     type: "string"
//     description: "one or two words that describe the memory"
//   - name: "memory_text"
//     type: "string"
//     description: "The text of memory you want to save"
//   - name: "memory_type"
//     type: "string"
//     description: "one of: `consequence` -- the set of actions that caused success / fail; `reflection` -- what can you do better next time; `familirity` -- what new did you get about the project; `relationship` -- what new did you get about the user."
// parameters_required:
//   - "memory_topic"
//   - "memory_text"
//   - "memory_type"

// - "op"
// - name: "op"
// type: "string"
// description: "Operation on a file: 'new', 'edit', 'remove'"
// - "lookup_definitions"
// - name: "lookup_definitions"
// type: "string"
// description: "Comma separated types that might be useful in making this change"
// - name: "remember_how_to_use_tools"
// description: Save a note to memory.
// parameters:
//   - name: "text"
//     type: "string"
//     description: "Write the exact format message here, starting with CORRECTION_POINTS"
// parameters_required:
//   - "text"

// - name: "memorize_if_user_asks"
// description: |
//     DO NOT CALL UNLESS USER EXPLICITLY ASKS. Use this format exactly:
//     when ... [describe situation when it's applicable] use ... tool call or method or plan
// parameters:
//   - name: "text"
//     type: "string"
//     description: "Follow the format in function description."
// parameters_required:
//   - "text"
//   - "shortdesc"


#[derive(Deserialize)]
pub struct DictDeserialize {
    pub tools: Vec<ToolDict>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolDict {
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

impl ToolDict {
    pub fn into_openai_style(self) -> Value {
        make_openai_tool_value(
            self.name,
            self.description,
            self.parameters_required,
            self.parameters,
        )
    }
}

pub fn tools_compiled_in(turned_on: &Vec<String>) -> Result<Vec<ToolDict>, String> {
    let at_dict: DictDeserialize = serde_yaml::from_str(TOOLS)
        .map_err(|e|format!("Failed to parse TOOLS: {}", e))?;
    Ok(at_dict.tools.iter().filter(|x|turned_on.contains(&x.name)).cloned().collect::<Vec<_>>())
}

pub async fn tools_from_customization(gcx: Arc<ARwLock<GlobalContext>>, turned_on: &Vec<String>) -> Vec<ToolCustDict> {
    return match crate::toolbox::toolbox_config::load_customization(gcx.clone()).await {
        Ok(tconfig) => tconfig.tools.iter().filter(|x|turned_on.contains(&x.name)).cloned().collect::<Vec<_>>(),
        Err(e) => {
            tracing::error!("Error loading toolbox config: {:?}", e);
            vec![]
        }
    }
}
