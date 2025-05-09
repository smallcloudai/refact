use std::sync::Arc;

use indexmap::IndexMap;
use tokio::sync::RwLock as ARwLock;

use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::integrations::running_integrations::load_integration_tools;

use super::tools_description::{Tool, ToolGroup, ToolGroupCategory};


pub async fn get_builtin_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Vec<ToolGroup> {
    let config_dir = gcx.read().await.config_dir.clone();
    let config_path = config_dir.join("builtin_tools.yaml").to_string_lossy().to_string();

    let mut tools = vec![
        Box::new(crate::tools::tool_ast_definition::ToolAstDefinition{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::tool_ast_reference::ToolAstReference{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::tool_tree::ToolTree{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::file_edit::tool_create_textdoc::ToolCreateTextDoc{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::file_edit::tool_update_textdoc::ToolUpdateTextDoc {config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::file_edit::tool_update_textdoc_regex::ToolUpdateTextDocRegex {config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::tool_web::ToolWeb{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::tool_cat::ToolCat{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::tool_rm::ToolRm{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::tool_mv::ToolMv{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::tool_strategic_planning::ToolStrategicPlanning{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::tool_regex_search::ToolRegexSearch{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::tool_knowledge::ToolGetKnowledge{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::tool_create_knowledge::ToolCreateKnowledge{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::tool_create_memory_bank::ToolCreateMemoryBank{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::tool_search::ToolSearch{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
        Box::new(crate::tools::tool_locate_search::ToolLocateSearch{config_path: config_path.clone()}) as Box<dyn Tool + Send>,
    ];

    vec![
        ToolGroup {
            name: "builtin".to_string(),
            description: "Builtin tools".to_string(),
            category: ToolGroupCategory::Builtin,
            tools,
        },
    ]
}

pub async fn get_available_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
    _supports_clicks: bool,  // XXX
) -> Result<IndexMap<String, Box<dyn Tool + Send>>, String> {
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

    let mut tools_all = get_builtin_tools();
    tools_all.extend(
        load_integration_tools(gcx, allow_experimental).await
    );

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
        if tool.tool_description().experimental && !allow_experimental {
            continue;
        }
        filtered_tools.insert(tool_name, tool);
    }

    Ok(filtered_tools)
}
