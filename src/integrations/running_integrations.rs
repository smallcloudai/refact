use std::path::PathBuf;
use std::sync::Arc;
use indexmap::IndexMap;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;

use crate::tools::tools_description::Tool;
use crate::global_context::GlobalContext;


pub async fn load_integration_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
    _current_project: String,
    allow_experimental: bool,
) -> IndexMap<String, Arc<AMutex<Box<dyn Tool + Send>>>> {
    let (global_dir, _workspace_folders_arc) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.config_dir.clone(), gcx_locked.documents_state.workspace_folders.clone())
    };
    let mut config_folders: Vec<PathBuf> = Vec::new();
    // XXX filter _workspace_folders_arc that fit _current_project
    config_folders.push(global_dir);

    let mut error_log: Vec<crate::integrations::setting_up_integrations::YamlError> = Vec::new();
    let lst: Vec<&str> = crate::integrations::integrations_list();
    let records = crate::integrations::setting_up_integrations::read_integrations_d(&config_folders, &lst, &mut error_log);

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
                tracing::error!("Failed to load integration {}: {}", rec.integr_name, e);
                continue;
            }
        };
        integr.integr_settings_apply(&rec.config_unparsed);
        tools.insert(rec.integr_name.clone(), Arc::new(AMutex::new(integr.integr_upgrade_to_tool())));
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
