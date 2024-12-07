// use std::path::PathBuf;
use std::sync::Arc;
use indexmap::IndexMap;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;

use crate::tools::tools_description::Tool;
use crate::global_context::GlobalContext;
use crate::integrations::integr_abstract::IntegrationTrait;

pub async fn load_integration_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
    _current_project: String,
    _allow_experimental: bool,
) -> IndexMap<String, Arc<AMutex<Box<dyn Tool + Send>>>> {
    let (integraions_map, _yaml_errors) = load_integrations(gcx.clone(), _current_project, _allow_experimental).await;
    let mut tools = IndexMap::new();
    for (name, integr) in integraions_map {
        if integr.can_upgrade_to_tool() {
            tools.insert(name.clone(), Arc::new(AMutex::new(integr.integr_upgrade_to_tool(&name))));
        }
    }
    tools
}

pub async fn load_integrations(
    gcx: Arc<ARwLock<GlobalContext>>,
    _current_project: String,
    _allow_experimental: bool,
) -> (IndexMap<String, Box<dyn IntegrationTrait + Send + Sync>>, Vec<crate::integrations::setting_up_integrations::YamlError>) {
    // XXX filter _workspace_folders_arc that fit _current_project
    let (config_dirs, global_config_dir) = crate::integrations::setting_up_integrations::get_config_dirs(gcx.clone()).await;
    let integrations_yaml_path = crate::integrations::setting_up_integrations::get_integrations_yaml_path(gcx.clone()).await;

    let mut error_log: Vec<crate::integrations::setting_up_integrations::YamlError> = Vec::new();
    let lst: Vec<&str> = crate::integrations::integrations_list();
    let vars_for_replacements = crate::integrations::setting_up_integrations::get_vars_for_replacements(gcx.clone()).await;
    let records = crate::integrations::setting_up_integrations::read_integrations_d(&config_dirs, &global_config_dir, &integrations_yaml_path, &vars_for_replacements, &lst, &mut error_log);

    let mut integrations_map = IndexMap::new();
    for rec in records {
        if !rec.on_your_laptop {
            continue;
        }
        if !rec.integr_config_exists {
            continue;
        }
        let mut integr = match crate::integrations::integration_from_name(&rec.integr_name) {
            Ok(x) => x,
            Err(e) => {
                tracing::error!("don't have integration {}: {}", rec.integr_name, e);
                continue;
            }
        };
        let should_be_fine = integr.integr_settings_apply(&rec.config_unparsed);
        if should_be_fine.is_err() {
            // tracing::warn!("failed to apply settings for integration {}: {:?}", rec.integr_name, should_be_fine.err());
            error_log.push(crate::integrations::setting_up_integrations::YamlError {
                integr_config_path: rec.integr_config_path.clone(),
                error_line: 0,
                error_msg: format!("failed to apply settings: {:?}", should_be_fine.err()),
            });
        }
        integrations_map.insert(rec.integr_name.clone(), integr);
    }

    for e in error_log.iter() {
        tracing::error!(
            "{}:{} {:?}",
            crate::nicer_logs::last_n_chars(&e.integr_config_path, 30),
            e.error_line,
            e.error_msg,
        );
    }

    (integrations_map, error_log)
}
