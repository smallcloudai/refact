use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde_json::{Value, json};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatUsage, ContextEnum};
use crate::global_context::GlobalContext;
use crate::integrations::integr_github::ToolGithub;
use crate::integrations::integr_gitlab::ToolGitlab;
use crate::integrations::integr_pdb::ToolPdb;
use crate::integrations::integr_chrome::ToolChrome;
use crate::integrations::integr_postgres::ToolPostgres;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandsRequireConfirmationConfig { // todo: fix typo
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

    fn tool_description(&self) -> ToolDesc {
        unimplemented!();
    }
}

async fn read_integrations_yaml(cache_dir: &PathBuf) -> Result<serde_yaml::Value, String> {
    let yaml_path = cache_dir.join("integrations.yaml");

    let file = std::fs::File::open(&yaml_path).map_err(
        |e| format!("Failed to open {}: {}", yaml_path.display(), e)
    )?;

    let reader = std::io::BufReader::new(file);
    serde_yaml::from_reader(reader).map_err(
        |e| {
            let location = e.location().map(|loc| format!(" at line {}, column {}", loc.line(), loc.column())).unwrap_or_default();
            format!("Failed to parse {}{}: {}", yaml_path.display(), location, e)
        }
    )
}

pub async fn tools_merged_and_filtered(gcx: Arc<ARwLock<GlobalContext>>) -> Result<IndexMap<String, Arc<AMutex<Box<dyn Tool + Send>>>>, String>
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
    let integrations_value = match read_integrations_yaml(&cache_dir).await {
        Ok(value) => value,
        Err(e) => return Err(format!("Problem in integrations.yaml: {}", e)),
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
    ]);

    if allow_experimental {
        // The approach here: if it exists, it shouldn't have syntax errors, note the "?"
        if let Some(gh_config) = integrations_value.get("github") {
            tools_all.insert("github".to_string(), Arc::new(AMutex::new(Box::new(ToolGithub::new_from_yaml(gh_config)?) as Box<dyn Tool + Send>)));
        }
        if let Some(gl_config) = integrations_value.get("gitlab") {
            tools_all.insert("gitlab".to_string(), Arc::new(AMutex::new(Box::new(ToolGitlab::new_from_yaml(gl_config)?) as Box<dyn Tool + Send>)));
        }
        if let Some(pdb_config) = integrations_value.get("pdb") {
            tools_all.insert("pdb".to_string(), Arc::new(AMutex::new(Box::new(ToolPdb::new_from_yaml(pdb_config)?) as Box<dyn Tool + Send>)));
        }
        if let Some(chrome_config) = integrations_value.get("chrome") {
            tools_all.insert("chrome".to_string(), Arc::new(AMutex::new(Box::new(ToolChrome::new_from_yaml(chrome_config)?) as Box<dyn Tool + Send>)));
        }
        if let Some(postgres_config) = integrations_value.get("postgres") {
            tools_all.insert("postgres".to_string(), Arc::new(AMutex::new(Box::new(ToolPostgres::new_from_yaml(postgres_config)?) as Box<dyn Tool + Send>)));
        }
        #[cfg(feature="vecdb")]
        tools_all.insert("knowledge".to_string(), Arc::new(AMutex::new(Box::new(crate::tools::tool_knowledge::ToolGetKnowledge{}) as Box<dyn Tool + Send>)));
    }

    if let Some(cmdline) = integrations_value.get("cmdline") {
        let blocking_cmdline_tools = crate::tools::tool_cmdline::cmdline_tool_from_yaml_value(cmdline)?;
        tools_all.extend(blocking_cmdline_tools);
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

    Ok(filtered_tools)
}

pub async fn commands_require_confirmation_rules_from_integrations_yaml(gcx: Arc<ARwLock<GlobalContext>>) -> Result<CommandsRequireConfirmationConfig, String>
{
    let cache_dir = gcx.read().await.cache_dir.clone();
    let integrations_value = read_integrations_yaml(&cache_dir).await?;

    serde_yaml::from_value::<CommandsRequireConfirmationConfig>(integrations_value)
        .map_err(|e| format!("Failed to parse CommandsRequireConfirmationConfig: {}", e))
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
    description: "Like cat in console, but better: it can read multiple files (texts, images, etc.) and skeletonize them. Give it AST symbols important for the goal (classes, functions, variables, etc) to see them in full."
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
    experimental: true
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

  - name: "gitlab"
    agentic: true
    experimental: true
    description: "Access to glab command line command, to fetch issues, review PRs."
    parameters:
      - name: "project_dir"
        type: "string"
        description: "Look at system prompt for location of version control (.git folder) of the active file."
      - name: "command"
        type: "string"
        description: 'Examples:\nglab issue create --description "hello world" --title "Testing glab integration"\nglab issue list --author @me\n'
    parameters_required:
      - "project_dir"
      - "command"

  - name: "chrome"
    agentic: true
    experimental: true
    description: "A real web browser with graphical interface."
    parameters:
      - name: "command"
        type: "string"
        description: |
          Supports these commands:
          navigate_to <uri> <tab_id>
          screenshot <tab_id>
          html <tab_id>
          reload <tab_id>
    parameters_required:
      - "command"

  - name: "postgres"
    agentic: true
    experimental: true
    description: "PostgreSQL integration, can run a single query per call."
    parameters:
      - name: "query"
        type: "string"
        description: |
          Don't forget semicolon at the end, examples:
          SELECT * FROM table_name;
          CREATE INDEX my_index_users_email ON my_users (email);
    parameters_required:
      - "query"

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
    pub name: String,
    #[serde(rename = "type", default = "default_param_type")]
    pub param_type: String,
    pub description: String,
}

fn default_param_type() -> String {
    "string".to_string()
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
}

#[derive(Deserialize)]
pub struct ToolDictDeserialize {
    pub tools: Vec<ToolDesc>,
}

pub async fn tool_description_list_from_yaml(
    tools: indexmap::IndexMap<String, Arc<AMutex<Box<dyn Tool + Send>>>>,
    turned_on: &Vec<String>,
    allow_experimental: bool,
) -> Result<Vec<ToolDesc>, String> {
    let tool_desc_deser: ToolDictDeserialize = serde_yaml::from_str(BUILT_IN_TOOLS)
        .map_err(|e|format!("Failed to parse BUILT_IN_TOOLS: {}", e))?;

    let mut tool_desc_vec = vec![];
    tool_desc_vec.extend(tool_desc_deser.tools.iter().cloned());

    for (tool_name, tool_arc) in tools {
        if !tool_desc_vec.iter().any(|desc| desc.name == tool_name) {
            let tool_desc = {
                let tool_locked = tool_arc.lock().await;
                tool_locked.tool_description()
            };
            tool_desc_vec.push(tool_desc);
        }
    }

    Ok(tool_desc_vec.iter()
        .filter(|x| turned_on.contains(&x.name) && (allow_experimental || !x.experimental))
        .cloned()
        .collect::<Vec<_>>())
}
