use std::sync::Arc;
use std::path::PathBuf;
use serde::Deserialize;
use tokio::sync::RwLock as ARwLock;
use tokio::time::Duration;
use tokio::fs;
use tracing::{warn, error};
use std::time::SystemTime;
use std::collections::HashMap;
use crate::global_context::GlobalContext;
use crate::privacy::any_glob_matches_path;


pub const DEFAULT_BLOCKLIST_DIRS: &[&str] = &[
    "*\\.*", "*/.*",
    "*target*", "*node_modules*", "*vendor*",
    "*build*", "*dist*", "*bin*", "*pkg*", "*lib*",
    "*obj*", "*out*", "*venv*", "*env*", "*tmp*", "*temp*",
    "*logs*", "*coverage*", "*backup*", "*__pycache__*",
    "*_trajectories*", "*.gradle*",
];

#[derive(Debug, Clone, Deserialize)]
pub struct IndexingSettings {
    pub blocklist: Vec<String>,
    pub additional_indexing_dirs: Vec<String>,
}

impl Default for IndexingSettings {
    fn default() -> Self {
        IndexingSettings {
            blocklist: DEFAULT_BLOCKLIST_DIRS.iter().map(|s| s.to_string()).collect(),
            additional_indexing_dirs: vec![],
        }
    }
}

pub struct GlobalIndexingSettings {
    pub global_indexing_settings: IndexingSettings,
    pub vcs_indexing_settings_map: HashMap<String, IndexingSettings>,
    pub loaded_ts: u64,
}

impl Default for GlobalIndexingSettings {
    fn default() -> Self {
        GlobalIndexingSettings {
            global_indexing_settings: IndexingSettings::default(),
            vcs_indexing_settings_map: HashMap::new(),
            loaded_ts: 0,
        }
    }
}

impl GlobalIndexingSettings {
    // NOTE: path argument should be absolute
    pub fn get_indexing_settings(&self, path: PathBuf) -> IndexingSettings {
        let mut best_workspace: Option<PathBuf> = None;

        for (workspace, _) in &self.vcs_indexing_settings_map {
            let workspace_path = PathBuf::from(workspace);
            if path.starts_with(&workspace_path) {
                if best_workspace.is_none() || workspace_path.components().count() > best_workspace.clone().unwrap().components().count() {
                    best_workspace = Some(workspace_path);
                }
            }
        }

        if let Some(workspace) = best_workspace {
            self.vcs_indexing_settings_map.get(&workspace.to_str().unwrap().to_string()).cloned().unwrap_or_default()
        } else {
            self.global_indexing_settings.clone()
        }
    }
}


fn load_indexing_yaml_str(
    indexing_yaml_str: &str,
    indexing_root: Option<&PathBuf>,
) -> Result<IndexingSettings, String> {
    match serde_yaml::from_str::<IndexingSettings>(&indexing_yaml_str) {
        Ok(indexing_settings) => {
            let mut additional_indexing_dirs = vec![];
            for indexing_dir in indexing_settings.additional_indexing_dirs.iter() {
                if indexing_dir.is_empty() {
                    continue;
                }
                let indexing_dir_path = PathBuf::from(indexing_dir);
                if indexing_dir_path.is_absolute() {
                    // TODO: complicated case
                    additional_indexing_dirs.push(indexing_dir.clone());
                } else {
                    if let Some(root) = indexing_root {
                        additional_indexing_dirs.push(root.join(indexing_dir).to_str().unwrap().to_string());
                    } else {
                        warn!("skip relative additional indexing dir {} from global indexing.yaml", indexing_dir)
                    }
                }
            }
            return Ok(IndexingSettings{blocklist: indexing_settings.blocklist, additional_indexing_dirs})
        }
        Err(e) => {
            return Err(format!("indexing.yaml parsing failed\n{}", e));
        }
    }
}

pub async fn load_indexing_yaml(
    indexing_path: &PathBuf,
    indexing_root: Option<&PathBuf>,
) -> Result<IndexingSettings, String> {
    match fs::read_to_string(&indexing_path.as_path()).await.map_err(|e| e.to_string()) {
        Ok(content) => {
            match load_indexing_yaml_str(&content.as_str(), indexing_root) {
                Ok(indexing_settings) => {
                    return Ok(indexing_settings)
                }
                Err(e) => {
                    return Err(format!("load {} failed\n{}", indexing_path.display(), e));
                }
            }
        }
        Err(e) => {
            return Err(format!("load {} failed\n{}", indexing_path.display(), e));
        }
    }
}

pub async fn load_global_indexing_yaml(gcx: Arc<ARwLock<GlobalContext>>) -> IndexingSettings {
    let config_dir = {
        let gcx_locked = gcx.read().await;
        gcx_locked.config_dir.clone()
    };
    let global_indexing_path = PathBuf::from(config_dir).join("indexing.yaml");
    load_indexing_yaml(&global_indexing_path, None).await.unwrap_or_else(|e| {
        error!("{}, fallback to defaults", e);
        IndexingSettings::default()
    })
}

async fn get_vcs_dirs(gcx: Arc<ARwLock<GlobalContext>>) -> Vec<PathBuf> {
    let mut vcs_dirs = vec![];

    let workspace_vcs_roots = {
        let gcx_locked = gcx.read().await;
        gcx_locked.documents_state.workspace_vcs_roots.clone()
    };

    let vcs_roots_locked = workspace_vcs_roots.lock().unwrap();
    for project_path in vcs_roots_locked.iter() {
        vcs_dirs.push(project_path.clone());
    }

    vcs_dirs
}

async fn load_global_indexing_settings(gcx: Arc<ARwLock<GlobalContext>>) -> GlobalIndexingSettings {
    let global_indexing_settings = load_global_indexing_yaml(gcx.clone()).await;
    let vcs_dirs = get_vcs_dirs(gcx.clone()).await;
    let mut vcs_indexing_settings_map: HashMap<String, IndexingSettings> = HashMap::new();
    for indexing_root in vcs_dirs {
        let indexing_path = indexing_root.join(".refact").join("indexing.yaml");
        match load_indexing_yaml(&indexing_path, Some(&indexing_root)).await {
            Ok(indexing_settings) => {
                vcs_indexing_settings_map.insert(
                    indexing_root.to_str().unwrap().to_string(),
                    IndexingSettings {
                        blocklist: global_indexing_settings.blocklist.iter().chain(indexing_settings.blocklist.iter()).cloned().collect(),
                        additional_indexing_dirs: global_indexing_settings.additional_indexing_dirs.iter().chain(indexing_settings.additional_indexing_dirs.iter()).cloned().collect(),
                    },
                );
            },
            Err(e) => {
                error!("{}, skip", e)
            }
        }
    }

    let loaded_ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    GlobalIndexingSettings{
        global_indexing_settings,
        vcs_indexing_settings_map,
        loaded_ts,
    }
}

const INDEXING_TOO_OLD: Duration = Duration::from_secs(3);

pub async fn load_global_indexing_settings_if_needed(gcx: Arc<ARwLock<GlobalContext>>) -> Arc<GlobalIndexingSettings>
{
    {
        let gcx_locked = gcx.read().await;
        let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        if gcx_locked.global_indexing_settings.loaded_ts + INDEXING_TOO_OLD.as_secs() > current_time {
            return gcx_locked.global_indexing_settings.clone();
        }
    }

    let global_indexing_settings = load_global_indexing_settings(gcx.clone()).await;
    {
        let mut gcx_locked = gcx.write().await;
        gcx_locked.global_indexing_settings = Arc::new(global_indexing_settings);
        gcx_locked.global_indexing_settings.clone()
    }
}

fn is_path_in_additional_indexing_dirs(indexing_settings: &IndexingSettings, path: &str) -> bool {
    for dir in indexing_settings.additional_indexing_dirs.iter() {
        if !dir.is_empty() && path.starts_with(dir.as_str()) {
            return true;
        }
    }
    false
}

pub fn is_this_inside_blocklisted_dir(indexing_settings: &IndexingSettings, path: &PathBuf) -> bool {
    if is_path_in_additional_indexing_dirs(indexing_settings, path.to_str().unwrap()) {
        return false;
    }
    is_blocklisted(&indexing_settings, &path)
}

pub fn is_blocklisted(indexing_settings: &IndexingSettings, path: &PathBuf) -> bool {
    return any_glob_matches_path(&indexing_settings.blocklist, &path)
}
