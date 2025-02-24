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

    async fn match_against_confirm_deny(
        &self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>
    ) -> Result<MatchConfirmDeny, String> {
        let command_to_match = self.command_to_match_against_confirm_deny(&args).map_err(|e| {
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

    fn command_to_match_against_confirm_deny(
        &self,
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

    fn tool_description(&self) -> ToolDesc {
        unimplemented!();
    }
}

pub async fn tools_merged_and_filtered(
    gcx: Arc<ARwLock<GlobalContext>>,
    _supports_clicks: bool,  // XXX
) -> Result<IndexMap<String, Box<dyn Tool + Send>>, String> {
    let (ast_on, vecdb_on, allow_experimental) = {
        let gcx_locked = gcx.read().await;
        #[cfg(feature="vecdb")]
        let vecdb_on = gcx_locked.vec_db.lock().await.is_some();
        #[cfg(not(feature="vecdb"))]
        let vecdb_on = false;
        (gcx_locked.ast_service.is_some(), vecdb_on, gcx_locked.cmdline.experimental)
    };

    let mut tools_all = IndexMap::from([
        ("definition".to_string(), Box::new(crate::tools::tool_ast_definition::ToolAstDefinition{}) as Box<dyn Tool + Send>),
        ("references".to_string(), Box::new(crate::tools::tool_ast_reference::ToolAstReference{}) as Box<dyn Tool + Send>),
        ("tree".to_string(), Box::new(crate::tools::tool_tree::ToolTree{}) as Box<dyn Tool + Send>),
        ("create_textdoc".to_string(), Box::new(crate::tools::file_edit::tool_create_textdoc::ToolCreateTextDoc{}) as Box<dyn Tool + Send>),
        ("replace_textdoc".to_string(), Box::new(crate::tools::file_edit::tool_replace_textdoc::ToolReplaceTextDoc{}) as Box<dyn Tool + Send>),
        ("update_textdoc".to_string(), Box::new(crate::tools::file_edit::tool_update_textdoc::ToolUpdateTextDoc {}) as Box<dyn Tool + Send>),
        ("update_textdoc_regex".to_string(), Box::new(crate::tools::file_edit::tool_update_textdoc_regex::ToolUpdateTextDocRegex {}) as Box<dyn Tool + Send>),
        ("web".to_string(), Box::new(crate::tools::tool_web::ToolWeb{}) as Box<dyn Tool + Send>),
        ("cat".to_string(), Box::new(crate::tools::tool_cat::ToolCat{}) as Box<dyn Tool + Send>),
        ("rm".to_string(), Box::new(crate::tools::tool_rm::ToolRm{}) as Box<dyn Tool + Send>),
        ("mv".to_string(), Box::new(crate::tools::tool_mv::ToolMv{}) as Box<dyn Tool + Send>),
        ("think".to_string(), Box::new(crate::tools::tool_deep_thinking::ToolDeepThinking{}) as Box<dyn Tool + Send>),
        // ("locate".to_string(), Box::new(crate::tools::tool_locate::ToolLocate{}) as Box<dyn Tool + Send>))),
        // ("locate".to_string(), Box::new(crate::tools::tool_relevant_files::ToolRelevantFiles{}) as Box<dyn Tool + Send>))),
        #[cfg(feature="vecdb")]
        ("search".to_string(), Box::new(crate::tools::tool_search::ToolSearch{}) as Box<dyn Tool + Send>),
        #[cfg(feature="vecdb")]
        ("locate".to_string(), Box::new(crate::tools::tool_locate_search::ToolLocateSearch{}) as Box<dyn Tool + Send>),
    ]);

    #[cfg(feature="vecdb")]
    tools_all.insert("knowledge".to_string(), Box::new(crate::tools::tool_knowledge::ToolGetKnowledge{}) as Box<dyn Tool + Send>);

    let integrations = crate::integrations::running_integrations::load_integration_tools(
        gcx.clone(),
        allow_experimental,
    ).await;
    tools_all.extend(integrations);

    let mut filtered_tools = IndexMap::new();
    for (tool_name, tool) in tools_all {
        let dependencies = tool.tool_depends_on();
        if dependencies.contains(&"ast".to_string()) && !ast_on {
            continue;
        }
        if dependencies.contains(&"vecdb".to_string()) && !vecdb_on {
            continue;
        }
        filtered_tools.insert(tool_name, tool);
    }

    Ok(filtered_tools)
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

  - name: "cat"
    description: "Like cat in console, but better: it can read multiple files and skeletonize them. Give it AST symbols important for the goal (classes, functions, variables, etc) to see them in full. It can also read images just fine."
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

  - name: "rm"
    description: "Deletes a file or directory. Use recursive=true for directories. Set dry_run=true to preview without deletion."
    parameters:
      - name: "path"
        type: "string"
        description: "Absolute or relative path of the file or directory to delete."
      - name: "recursive"
        type: "boolean"
        description: "If true and target is a directory, delete recursively. Defaults to false."
      - name: "dry_run"
        type: "boolean"
        description: "If true, only report what would be done without deleting."
      - name: "max_depth"
        type: "number"
        description: "(Optional) Maximum depth (currently unused)."
    parameters_required:
      - "path"

  - name: "mv"
    description: "Moves or renames files and directories. If a simple rename fails due to a cross-device error and the source is a file, it falls back to copying and deleting. Use overwrite=true to replace an existing target."
    parameters:
      - name: "source"
        type: "string"
        description: "Path of the file or directory to move."
      - name: "destination"
        type: "string"
        description: "Target path where the file or directory should be placed."
      - name: "overwrite"
        type: "boolean"
        description: "If true and target exists, replace it. Defaults to false."
    parameters_required:
      - "source"
      - "destination"

  - name: "create_textdoc"
    agentic: false
    description: "Creates a new text document or code"
    parameters:
      - name: "path"
        type: "string"
        description: "Absolute path to new file."
      - name: "content"
        type: "string"
        description: "The initial text or code."
    parameters_required:
      - "path"
      - "content"
      
  - name: "update_textdoc"
    agentic: false
    description: "Updates an existing document by replacing specific text. Optimized for large files or small changes where simple string replacement is sufficient. Prefer this over replace_textdoc for large files."
    parameters:
      - name: "path"
        type: "string"
        description: "Absolute path to the file to change."
      - name: "old_str"
        type: "string"
        description: "The exact text that needs to be updated. Use update_textdoc_regex if you need pattern matching."        
      - name: "replacement"
        type: "string"
        description: "The new text that will replace the old text."        
      - name: "multiple"
        type: "boolean"
        description: "If true, applies the replacement to all occurrences; if false, only the first occurrence is replaced."        
    parameters_required:
      - "path"
      - "old_str"
      - "replacement"
      - "multiple"
      
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

  - name: "think"
    agentic: true
    description: "Think about a complex problem to make a plan."
    parameters:
      - name: "problem_statement"
        type: "string"
        description: "What's the topic and what kind of result do you want?"
    parameters_required:
      - "problem_statement"
      
  - name: "update_textdoc_regex"
    agentic: true
    description: "Updates an existing document using regex pattern matching. Ideal when changes can be expressed as a regular expression or when you need to match variable text patterns. May be slower than update_textdoc for large files."
    parameters:
      - name: "path"
        type: "string"
        description: "Absolute path to the file to change."
      - name: "pattern"
        type: "string"
        description: "A regex pattern to match the text that needs to be updated. Prefer simpler regexes for better performance."        
      - name: "replacement"
        type: "string"
        description: "The new text that will replace the matched pattern."        
      - name: "multiple"
        type: "boolean"
        description: "If true, applies the replacement to all occurrences; if false, only the first occurrence is replaced."        
    parameters_required:
      - "path"
      - "pattern"
      - "replacement"
      - "multiple"
      
  - name: "replace_textdoc"
    agentic: true
    description: "Completely replaces the content of an existing document. Use ONLY for small files, as it rewrites the entire file. For large files or small changes, use update_textdoc instead."
    parameters:
      - name: "path"
        type: "string"
        description: "Absolute path to existing file. File must be small."
      - name: "replacement"
        type: "string"
        description: "The complete replacement text or code that will overwrite the entire file."
    parameters_required:
      - "path"
      - "replacement"

  - name: "github"
    agentic: true
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

  - name: "postgres"
    agentic: true
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

  - name: "mysql"
    agentic: true
    description: "MySQL integration, can run a single query per call."
    parameters:
      - name: "query"
        type: "string"
        description: |
          Don't forget semicolon at the end, examples:
          SELECT * FROM table_name;
          CREATE INDEX my_index_users_email ON my_users (email);
    parameters_required:
      - "query"

  - name: "docker"
    agentic: true
    experimental: true
    description: "Access to docker cli, in a non-interactive way, don't open a shell."
    parameters:
      - name: "command"
        type: "string"
        description: "Examples: docker images"
    parameters_required:
      - "command"

  - name: "knowledge"
    agentic: true
    description: "Fetches successful trajectories to help you accomplish your task. Call each time you have a new task to increase your chances of success."
    parameters:
      - name: "im_going_to_use_tools"
        type: "string"
        description: "Which tools are you about to use? Comma-separated list, examples: hg, git, gitlab, rust debugger"
      - name: "im_going_to_apply_to"
        type: "string"
        description: "What your actions will be applied to? List all you can identify, starting with the project name. Comma-separated list, examples: project1, file1.cpp, MyClass, PRs, issues"
      - name: "goal"
        type: "string"
        description: "What is your goal here?"
      - name: "language_slash_framework"
        type: "string"
        description: "What programming language and framework is the current project using? Use lowercase, dashes and dots. Examples: python/django, typescript/node.js, rust/tokio, ruby/rails, php/laravel, c++/boost-asio"
    parameters_required:
      - "im_going_to_use_tools"
      - "im_going_to_apply_to"
      - "goal"
      - "language_slash_framework"
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
