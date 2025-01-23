use std::sync::Arc;
use std::path::PathBuf;
use serde::Deserialize;
use tokio::sync::RwLock as ARwLock;
use tokio::time::Duration;
use tokio::fs;
use tracing::error;
use std::time::SystemTime;
use std::collections::HashMap;
use crate::global_context::GlobalContext;


pub const DEFAULT_BLOCKLIST_DIRS: &[&str] = &[
    "target", "node_modules", "vendor", "build", "dist",
    "bin", "pkg", "lib", "lib64", "obj",
    "out", "venv", "env", "tmp", "temp", "logs",
    "coverage", "backup", "__pycache__",
    "_trajectories", ".gradle",
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
    pub indexing_settings_map: HashMap<String, IndexingSettings>,
    pub loaded_ts: u64,
}

impl Default for GlobalIndexingSettings {
    fn default() -> Self {
        GlobalIndexingSettings {
            indexing_settings_map: HashMap::new(),
            loaded_ts: 0,
        }
    }
}

impl GlobalIndexingSettings {
    // NOTE: path argument should be absolute
    pub fn get_indexing_settings(&self, path: PathBuf) -> IndexingSettings {
        let mut best_workspace: Option<PathBuf> = None;

        for (workspace, _) in &self.indexing_settings_map {
            let workspace_path = PathBuf::from(workspace);
            if path.starts_with(&workspace_path) {
                if best_workspace.is_none() || workspace_path.components().count() > best_workspace.clone().unwrap().components().count() {
                    best_workspace = Some(workspace_path);
                }
            }
        }

        if let Some(workspace) = best_workspace {
            self.indexing_settings_map.get(&workspace.to_str().unwrap().to_string()).cloned().unwrap_or_default()
        } else {
            IndexingSettings::default()
        }
    }
}

pub async fn load_indexing_yaml(vcs_root: &PathBuf) -> IndexingSettings {
    let indexing_path = vcs_root.join(".refact").join("indexing.yaml");
    match fs::read_to_string(&indexing_path.as_path()).await.map_err(|e| e.to_string()) {
        Ok(content) => {
            match serde_yaml::from_str::<IndexingSettings>(&content) {
                Ok(indexing_settings) => {
                    let default_indexing_settings = IndexingSettings::default();
                    let blocklist = default_indexing_settings.blocklist.iter().chain(indexing_settings.blocklist.iter()).cloned().collect();
                    let mut additional_indexing_dirs = vec![];
                    for indexing_dir in default_indexing_settings.additional_indexing_dirs.iter().chain(indexing_settings.additional_indexing_dirs.iter()) {
                        if indexing_dir.is_empty() {
                            continue;
                        }
                        let indexing_dir_path = PathBuf::from(indexing_dir);
                        if indexing_dir_path.is_absolute() {
                            // TODO: complicated case
                            additional_indexing_dirs.push(indexing_dir.clone());
                        } else {
                            additional_indexing_dirs.push(vcs_root.join(indexing_dir).to_str().unwrap().to_string());
                        }
                    }
                    return IndexingSettings{blocklist, additional_indexing_dirs}
                }
                Err(e) => {
                    error!("parsing {} failed\n{}", indexing_path.display(), e);
                    IndexingSettings::default()
                }
            }
        }
        Err(e) => {
            error!("parsing {} failed\n{}", indexing_path.display(), e);
            IndexingSettings::default()
        }
    }
}

async fn get_vcs_dirs(gcx: Arc<ARwLock<GlobalContext>>) -> Vec<PathBuf> {
    let mut vcs_dirs = vec![];

    let workspace_vcs_roots = {
        let gcx_locked = gcx.read().await;
        gcx_locked.documents_state.workspace_vcs_roots.clone()
    };

    let vcs_roots_locked = workspace_vcs_roots.lock().unwrap();
    for project_path in vcs_roots_locked.iter() {
        if project_path.join(".refact").exists() {
            vcs_dirs.push(project_path.clone());
        }
    }

    vcs_dirs
}

async fn load_global_indexing_settings(gcx: Arc<ARwLock<GlobalContext>>) -> GlobalIndexingSettings {
    let vcs_dirs = get_vcs_dirs(gcx.clone()).await;
    let mut indexing_settings_map: HashMap<String, IndexingSettings> = HashMap::new();
    for project_path in vcs_dirs {
        indexing_settings_map.insert(
            project_path.to_str().unwrap().to_string(),
            load_indexing_yaml(&project_path.to_path_buf()).await,
        );
    }

    let loaded_ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    GlobalIndexingSettings{
        indexing_settings_map,
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
    let mut path = path.clone();
    while path.parent().is_some() {
        path = path.parent().unwrap().to_path_buf();
        if is_blocklisted(&indexing_settings, &path) {
            return true;
        }
    }
    false
}

pub fn is_blocklisted(indexing_settings: &IndexingSettings, path: &PathBuf) -> bool {
    if let Some(file_name) = path.file_name() {
        if indexing_settings.blocklist.contains(&file_name.to_str().unwrap_or_default().to_string()) {
            return true;
        }
        if let Some(file_name_str) = file_name.to_str() {
            if file_name_str.starts_with(".") {
                return true;
            }
        }
    }
    false
}
