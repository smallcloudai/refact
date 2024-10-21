use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde_json::{Value, json};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use tracing::warn;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatUsage, ContextEnum};
use crate::global_context::GlobalContext;
use crate::integrations::integr_github::ToolGithub;
use crate::integrations::integr_pdb::ToolPdb;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandsRequireConfimationConfig { // todo: fix typo
    pub commands_need_confirmation: Vec<String>,
    pub commands_deny: Vec<String>,
}

#[async_trait]
pub trait Tool: Send + Sync {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String>;

    fn command_to_match_against_confirm_deny(
        &self,
        _args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        Ok("".to_string())
    }

    fn tool_depends_on(&self) -> Vec<String> { vec![] }   // "ast", "vecdb"

    fn usage(&mut self) -> &mut Option<ChatUsage> {
        static mut DEFAULT_USAGE: Option<ChatUsage> = None;
        #[allow(static_mut_refs)]
        unsafe { &mut DEFAULT_USAGE }
    }
}

async fn read_integrations_value(cache_dir: &PathBuf) -> Result<serde_yaml::Value, String> {
    let yaml_path = cache_dir.join("integrations.yaml");

    let integrations_yaml = tokio::fs::read_to_string(&yaml_path).await.map_err(
        |e| format!("Failed to read integrations.yaml: {}", e)
    )?;

    serde_yaml::from_str::<serde_yaml::Value>(&integrations_yaml).map_err(
        |e| format!("Failed to parse integrations.yaml: {}", e)
    )
}

pub async fn tools_merged_and_filtered(gcx: Arc<ARwLock<GlobalContext>>) -> IndexMap<String, Arc<AMutex<Box<dyn Tool + Send>>>>
{
    let (ast_on, vecdb_on, allow_experimental) = {
        let gcx_locked = gcx.read().await;
        #[cfg(feature="vecdb")]
        let vecdb_on = gcx_locked.vec_db.lock().await.is_some();
        #[cfg(not(feature="vecdb"))]
        let vecdb_on = false;
        (gcx_locked.ast_service.is_some(), vecdb_on, gcx_locked.cmdline.experimental)
    };

    let cache_dir = gcx.read().await.cache_dir.clone();
    let integrations_value = match read_integrations_value(&cache_dir).await {
        Ok(value) => value,
        Err(e) => {
            warn!(e);
            serde_yaml::Value::default()
        }
    };

    let mut tools_all = IndexMap::from([
        ("definition".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::tool_ast_definition::ToolAstDefinition{}) as Box<dyn Tool + Send>))),
        ("references".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::tool_ast_reference::ToolAstReference{}) as Box<dyn Tool + Send>))),
        ("tree".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::tool_tree::ToolTree{}) as Box<dyn Tool + Send>))),
        ("patch".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::tool_patch::ToolPatch::new()) as Box<dyn Tool + Send>))),
        ("web".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::tool_web::ToolWeb{}) as Box<dyn Tool + Send>))),
        ("cat".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::tool_cat::ToolCat{}) as Box<dyn Tool + Send>))),
        // ("locate".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::tool_locate::ToolLocate{}) as Box<dyn Tool + Send>))),
        // ("locate".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::tool_relevant_files::ToolRelevantFiles{}) as Box<dyn Tool + Send>))),
        #[cfg(feature="vecdb")]
        ("search".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::tool_search::ToolSearch{}) as Box<dyn Tool + Send>))),
        #[cfg(feature="vecdb")]
        ("locate".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::tool_locate_search::ToolLocateSearch{}) as Box<dyn Tool + Send>))),
        // ("web_screenshot".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::tool_web_screenshot::ToolWebScreenshot{}) as Box<dyn Tool + Send>))),
    ]);

    if allow_experimental {
        // ("save_knowledge".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::att_knowledge::ToolSaveKnowledge{}) as Box<dyn Tool + Send>))),
        // ("memorize_if_user_asks".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::att_note_to_self::AtNoteToSelf{}) as Box<dyn AtTool + Send>))),
        if let Some(github_tool) = ToolGithub::new_if_configured(&integrations_value) {
            tools_all.insert("github".to_string(), Arc::new(AMutex::new(Box::new(github_tool) as Box<dyn Tool + Send>)));
        }
        if let Some(pdb_tool) = ToolPdb::new_if_configured(&integrations_value) {
            tools_all.insert("pdb".to_string(), Arc::new(AMutex::new(Box::new(pdb_tool) as Box<dyn Tool + Send>)));
        }
        if let Some(chrome_tool) = ToolChrome::new_if_configured(&integrations_value) {
            tools_all.insert("chrome".to_string(), Arc::new(AMutex::new(Box::new(chrome_tool) as Box<dyn Tool + Send>)));
        }
        #[cfg(feature="vecdb")]
        tools_all.insert("knowledge".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::tool_knowledge::ToolGetKnowledge{}) as Box<dyn Tool + Send>)));
    }

    let mut filtered_tools = IndexMap::new();
    for (tool_name, tool_arc) in tools_all {
        let tool_locked = tool_arc.lock().await;
        let dependencies = tool_locked.tool_depends_on();
        if dependencies.contains(&"ast".to_string()) && !ast_on {
            continue;
        }
        if dependencies.contains(&"vecdb".to_string()) && !vecdb_on {
            continue;
        }
        filtered_tools.insert(tool_name, tool_arc.clone());
    }

    filtered_tools
}

pub async fn commands_require_confirmation_rules_from_integrations_yaml(gcx: Arc<ARwLock<GlobalContext>>) -> Result<CommandsRequireConfimationConfig, String>
{
    let cache_dir = gcx.read().await.cache_dir.clone();
    let integrations_value = read_integrations_value(&cache_dir).await?;

    serde_yaml::from_value::<CommandsRequireConfimationConfig>(integrations_value)
        .map_err(|e| format!("Failed to parse CommandsRequireConfimationConfig: {}", e))
}

const BUILT_IN_TOOLS: &str = r####"
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

  - name: "definition"
    description: "Read definition of a symbol in the project using AST"
    parameters:
      - name: "symbol"
        type: "string"
        description: "The exact name of a function, method, class, type alias. No spaces allowed."
      - name: "skeleton"
        type: "boolean"
        description: "Skeletonize ouput. Set true to explore, set false when as much context as possible is needed."
    parameters_required:
      - "symbol"

  - name: "references"
    description: "Find usages of a symbol within a project using AST"
    parameters:
      - name: "symbol"
        type: "string"
        description: "The exact name of a function, method, class, type alias. No spaces allowed."
      - name: "skeleton"
        type: "boolean"
        description: "Skeletonize ouput. Set true to explore, set false when as much context as possible is needed."
    parameters_required:
      - "symbol"

  - name: "tree"
    description: "Get a files tree with symbols for the project. Use it to get familiar with the project, file names and symbols"
    parameters:
      - name: "path"
        type: "string"
        description: "An absolute path to get files tree for. Do not pass it if you need a full project tree."
      - name: "use_ast"
        type: "boolean"
        description: "If true, for each file an array of AST symbols will appear as well as its filename"
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
    experimental: true
    parameters:
      - name: "im_going_to_use_tools"
        type: "string"
        description: "Which tools are you about to use? Comma-separated list, examples: hg, git, github, gitlab, rust debugger, patch"
      - name: "im_going_to_apply_to"
        type: "string"
        description: "What your future actions will be applied to? List all you can identify, starting from the project name. Comma-separated list, examples: project1, file1.cpp, MyClass, PRs, issues"
    parameters_required:
      - "im_going_to_use_tools"
      - "im_going_to_apply_to"

  - name: "cat"
    description: "Like cat in console, but better: it can read multiple files and skeletonize them. Give it AST symbols important for the goal (classes, functions, variables, etc) to see them in full."
    parameters:
      - name: "paths"
        type: "string"
        description: "Comma separated file names or directories: dir1/file1.ext, dir2/file2.ext, dir3/dir4"
      - name: "symbols"
        type: "string"
        description: "Comma separated AST symbols: MyClass, MyClass::method, my_function"
      - name: "skeleton"
        type: "boolean"
        description: "if true, files will be skeletonized - mostly only AST symbols will be visible"
    parameters_required:
      - "paths"

  # -- agentic tools below --

  - name: "locate"
    agentic: true
    description: "Get a list of files that are relevant to solve a particular task."
    parameters:
      - name: "problem_statement"
        type: "string"
        description: "Copy word-for-word the problem statement as provided by the user, if available. Otherwise, tell what you need to do in your own words."
    parameters_required:
      - "problem_statement"

  - name: "patch"
    agentic: true
    description: |
      Collect context first, then write the necessary changes using the 📍-notation before code blocks, then call this function to apply the changes.
      To make this call correctly, you only need the tickets.
      If you wrote changes for multiple files, call this tool in parallel for each file.
      If you have several attempts to change a single thing, for example following a correction from the user, pass only the ticket for the latest one.
      Multiple tickets is allowed only for PARTIAL_EDIT, otherwise only one ticket must be provided.
    parameters:
      - name: "path"
        type: "string"
        description: "Path to the file to change."
      - name: "tickets"
        type: "string"
        description: "Use 3-digit tickets comma separated to refer to the changes within ONE file. No need to copy anything else. Additionaly, you can put DELETE here to delete the file."
    parameters_required:
      - "tickets"
      - "path"

  - name: "github"
    agentic: true
    experimental: true
    description: "Access to gh command line command, to fetch issues, review PRs."
    parameters:
      - name: "project_dir"
        type: "string"
        description: "Look at system prompt for location of version control (.git folder) of the active file."
      - name: "command"
        type: "string"
        description: 'Examples:\ngh issue create --body "hello world" --title "Testing gh integration"\ngh issue list --author @me --json number,title,updatedAt,url\n'
    parameters_required:
      - "project_dir"
      - "command"

  - name: "pdb"
    agentic: true
    experimental: true
    description: "Python debugger for exploring code and inspecting variables. It can open debugging sessions on python files and interact with them in the chat."
    parameters:
      - name: "command"
        type: "string"
        description: "Examples: 'python -m pdb script.py', 'break module_name.function_name', 'break 10', 'continue', 'print(variable_name)', 'list', 'quit'"
    parameters_required:
      - "command"

  - name: "web_screenshot"
    description: "Add screenshot of a web page to the chat."
    parameters:
      - name: "url"
        type: "string"
        description: "URL of the web page to make screenshot."
      - name: "html"
        type: "boolean"
        description: "In addition to the screenshot, return inner HTML of the page."
    parameters_required:
      - "url"

  - name: "chrome"
    agentic: true
    experimental: true
    description: "Web browser. Opens a tab and operates in it."
    parameters:
      - name: "command"
        type: "string"
        description: "Chrome has this commands: navigate_to url, screenshot, html, reload."
    parameters_required:
      - "command"
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
pub struct ToolDictDeserialize {
    pub tools: Vec<ToolDict>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolDict {
    pub name: String,
    #[serde(default)]
    pub agentic: bool,
    #[serde(default)]
    pub experimental: bool,
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
    agentic: bool,
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
                "agentic": agentic,
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
            self.agentic,
            self.description,
            self.parameters_required,
            self.parameters,
        )
    }
}

pub fn tool_description_list_from_yaml(
    turned_on: &Vec<String>,
    allow_experimental: bool,
) -> Result<Vec<ToolDict>, String> {
    let at_dict: ToolDictDeserialize = serde_yaml::from_str(BUILT_IN_TOOLS)
        .map_err(|e|format!("Failed to parse BUILT_IN_TOOLS: {}", e))?;
    Ok(at_dict.tools.iter()
        .filter(|x| turned_on.contains(&x.name) && (allow_experimental || !x.experimental))
        .cloned()
        .collect::<Vec<_>>())
}
