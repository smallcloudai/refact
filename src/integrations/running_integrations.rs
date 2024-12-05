// use std::path::PathBuf;
use std::sync::Arc;
use indexmap::IndexMap;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;

use crate::tools::tools_description::Tool;
use crate::global_context::GlobalContext;


pub async fn load_integration_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
    _current_project: String,
    _allow_experimental: bool,
) -> IndexMap<String, Arc<AMutex<Box<dyn Tool + Send>>>> {
    // XXX filter _workspace_folders_arc that fit _current_project
    let (config_dirs, global_config_dir) = crate::integrations::setting_up_integrations::get_config_dirs(gcx.clone()).await;
    let integrations_yaml_path = crate::integrations::setting_up_integrations::get_integrations_yaml_path(gcx.clone()).await;

    let mut error_log: Vec<crate::integrations::setting_up_integrations::YamlError> = Vec::new();
    let lst: Vec<&str> = crate::integrations::integrations_list();
    let vars_for_replacements = crate::integrations::setting_up_integrations::get_vars_for_replacements(gcx.clone()).await;
    let records = crate::integrations::setting_up_integrations::read_integrations_d(&config_dirs, &global_config_dir, &integrations_yaml_path, &vars_for_replacements, &lst, &mut error_log);

    let mut tools = IndexMap::new();
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
            tracing::error!("failed to apply settings for integration {}: {:?}", rec.integr_name, should_be_fine.err());
        }
        tools.insert(rec.integr_name.clone(), Arc::new(AMutex::new(integr.integr_upgrade_to_tool(&rec.integr_name))));
    }

    for e in error_log {
        tracing::error!(
            "{}:{} {:?}",
            crate::nicer_logs::last_n_chars(&&e.integr_config_path, 30),
            e.error_line,
            e.error_msg,
        );
    }

    tools
}
