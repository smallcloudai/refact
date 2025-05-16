use std::sync::Arc;

use tokio::sync::RwLock as ARwLock;

use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::integrations::running_integrations::load_integrations;

use super::tools_description::{Tool, ToolGroup, ToolGroupCategory};

fn tool_available(
    tool: &Box<dyn Tool + Send>,
    ast_on: bool,
    vecdb_on: bool,
    is_there_a_thinking_model: bool,
    allow_knowledge: bool,
    allow_experimental: bool,
) -> bool {
    let dependencies = tool.tool_depends_on();
    if dependencies.contains(&"ast".to_string()) && !ast_on {
        return false;
    }
    if dependencies.contains(&"vecdb".to_string()) && !vecdb_on {
        return false;
    }
    if dependencies.contains(&"thinking".to_string()) && !is_there_a_thinking_model {
        return false;
    }
    if dependencies.contains(&"knowledge".to_string()) && !allow_knowledge {
        return false;
    }
    if tool.tool_description().experimental && !allow_experimental {
        return false;
    }
    true
}

async fn tool_available_from_gcx(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> impl Fn(&Box<dyn Tool + Send>) -> bool {
    let (ast_on, vecdb_on, allow_experimental) = {
        let gcx_locked = gcx.read().await;
        let vecdb_on = gcx_locked.vec_db.lock().await.is_some();
        (gcx_locked.ast_service.is_some(), vecdb_on, gcx_locked.cmdline.experimental)
    };

    let (is_there_a_thinking_model, allow_knowledge) = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => {
            (caps.chat_models.get(&caps.defaults.chat_thinking_model).is_some(),
             caps.metadata.features.contains(&"knowledge".to_string()))
        },
        Err(_) => (false, false),
    };

    move |tool: &Box<dyn Tool + Send>| {
        tool_available(
            tool,
            ast_on,
            vecdb_on,
            is_there_a_thinking_model,
            allow_knowledge,
            allow_experimental,
        )
    }
}

impl ToolGroup {
    pub async fn retain_available_tools(
        &mut self,
        gcx: Arc<ARwLock<GlobalContext>>,
    ) {
        let tool_available = tool_available_from_gcx(gcx.clone()).await;
        self.tools.retain(|tool| tool_available(tool));
    }
}

async fn get_builtin_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Vec<ToolGroup> {
    let config_dir = gcx.read().await.config_dir.clone();
    let config_path = config_dir.join("builtin_tools.yaml").to_string_lossy().to_string();

    let mut codebase_search_tools: Vec<Box<dyn Tool + Send>> = vec![
        Box::new(crate::tools::tool_ast_definition::ToolAstDefinition{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_ast_reference::ToolAstReference{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_tree::ToolTree{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_cat::ToolCat{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_regex_search::ToolRegexSearch{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_search::ToolSearch{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_locate_search::ToolLocateSearch{config_path: config_path.clone()}),
    ];

    let codebase_change_tools: Vec<Box<dyn Tool + Send>> = vec![
        Box::new(crate::tools::file_edit::tool_create_textdoc::ToolCreateTextDoc{config_path: config_path.clone()}),
        Box::new(crate::tools::file_edit::tool_update_textdoc::ToolUpdateTextDoc{config_path: config_path.clone()}),
        Box::new(crate::tools::file_edit::tool_update_textdoc_regex::ToolUpdateTextDocRegex{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_rm::ToolRm{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_mv::ToolMv{config_path: config_path.clone()}),
    ];

    let web_tools: Vec<Box<dyn Tool + Send>> = vec![
        Box::new(crate::tools::tool_web::ToolWeb{config_path: config_path.clone()}),
    ];

    let deep_analysis_tools: Vec<Box<dyn Tool + Send>> = vec![
        Box::new(crate::tools::tool_strategic_planning::ToolStrategicPlanning{config_path: config_path.clone()}),
    ];

    let knowledge_tools: Vec<Box<dyn Tool + Send>> = vec![
        Box::new(crate::tools::tool_knowledge::ToolGetKnowledge{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_create_knowledge::ToolCreateKnowledge{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_create_memory_bank::ToolCreateMemoryBank{config_path: config_path.clone()}),
    ];

    let mut tool_groups = vec![
        ToolGroup {
            name: "codebase_search".to_string(),
            description: "Codebase search tools".to_string(),
            category: ToolGroupCategory::Builtin,
            tools: codebase_search_tools,
            allow_per_tool_toggle: false
        },
        ToolGroup {
            name: "codebase_change".to_string(),
            description: "Codebase modification tools".to_string(),
            category: ToolGroupCategory::Builtin,
            tools: codebase_change_tools,
            allow_per_tool_toggle: false
        },
        ToolGroup {
            name: "web".to_string(),
            description: "Web tools".to_string(),
            category: ToolGroupCategory::Builtin,
            tools: web_tools,
            allow_per_tool_toggle: false
        },
        ToolGroup {
            name: "strategic_planning".to_string(),
            description: "Strategic planning tools".to_string(),
            category: ToolGroupCategory::Builtin,
            tools: deep_analysis_tools,
            allow_per_tool_toggle: false
        },
        ToolGroup {
            name: "knowledge".to_string(),
            description: "Knowledge tools".to_string(),
            category: ToolGroupCategory::Builtin,
            tools: knowledge_tools,
            allow_per_tool_toggle: false
        },
    ];

    for tool_group in tool_groups.iter_mut() {
        tool_group.retain_available_tools(gcx.clone()).await;
    }

    tool_groups
}

async fn get_integration_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Vec<ToolGroup> {
    let mut tool_groups = vec![
        ToolGroup {
            name: "integrations".to_string(),
            description: "Integration tools".to_string(),
            category: ToolGroupCategory::Integration,
            tools: vec![],
            allow_per_tool_toggle: true
        },
        ToolGroup { 
            name: "mcp".to_string(), 
            description: "MCP tools".to_string(), 
            category: ToolGroupCategory::MCP, 
            tools: vec![],
            allow_per_tool_toggle: true
        },
    ];
    let (integrations_map, _yaml_errors) = load_integrations(gcx.clone(), &["**/*".to_string()]).await;
    for (name, integr) in integrations_map {
        for tool in integr.integr_tools(&name).await {
            if tool.tool_description().name.starts_with("mcp") {
                tool_groups[1].tools.push(tool);
            } else {
                tool_groups[0].tools.push(tool);
            }
        }
    }
    for tool_group in tool_groups.iter_mut() {
        tool_group.retain_available_tools(gcx.clone()).await;
    }

    tool_groups
}

pub async fn get_available_tool_groups(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Vec<ToolGroup> {
    let mut tools_all = get_builtin_tools(gcx.clone()).await;
    tools_all.extend(
        get_integration_tools(gcx).await
    );

    tools_all
}

pub async fn get_available_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Vec<Box<dyn Tool + Send>> {
    get_available_tool_groups(gcx).await.into_iter().flat_map(|g| g.tools).collect()
}
