use std::sync::Arc;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use tokio::sync::RwLock as ARwLock;
use tokio::time::Duration;
use tokio::fs;
use std::time::SystemTime;
use std::collections::HashMap;
use crate::files_correction::canonical_path;
use crate::global_context::GlobalContext;
use crate::privacy::any_glob_matches_path;


// TODO:
// remove debug prints
// react on .git appearing / disappearing => reindex all
// react on indexing.yaml additional_indexing_dirs change => reindex all
// make sure "git ls" lists unstaged files

// Testing:
// ignored file initial indexing doesn't happen
// ignored file add / remove file events don't do anything
// a file in an ignored dir, same tests
// changes in indexing.yaml loaded (almost) immediately


const INDEXING_TOO_OLD: Duration = Duration::from_secs(3);

pub const DEFAULT_BLOCKLIST_DIRS: &[&str] = &[
    "*/.*",     // hidden files start with dot
    "*/target/*",
    "*/node_modules/*",
    "*/vendor/*",
    "*/build/*",
    "*/dist/*",
    "*/bin/*",
    "*/pkg/*",
    "*/lib/*",
    "*/obj/*",
    "*/out/*",
    "*/venv/*",
    "*/env/*",
    "*/tmp/*",
    "*/temp/*",
    "*/logs/*",
    "*/coverage/*",
    "*/backup/*",
    "*/__pycache__/*",
    "*/_trajectories/*",
    "*/.gradle/*",
];


#[derive(Debug, Clone, Deserialize)]
pub struct IndexingSettings {
    #[serde(default)]
    pub blocklist: Vec<String>,
    #[serde(default)]
    pub additional_indexing_dirs: Vec<String>,
}

impl Default for IndexingSettings {
    fn default() -> Self {
        IndexingSettings {
            blocklist: vec![],
            additional_indexing_dirs: vec![],
        }
    }
}

pub struct IndexingEverywhere {
    pub global: IndexingSettings,
    pub vcs_indexing_settings_map: HashMap<String, IndexingSettings>,
    pub loaded_ts: u64,
}

impl Default for IndexingEverywhere {
    fn default() -> Self {
        IndexingEverywhere {
            global: IndexingSettings::default(),
            vcs_indexing_settings_map: HashMap::new(),
            loaded_ts: 0,
        }
    }
}

impl IndexingEverywhere {
    pub fn indexing_for_path(&self, path: &Path) -> IndexingSettings {
        assert!(path.is_absolute());
        let mut result: IndexingSettings = self.global.clone();
        result.blocklist.extend(DEFAULT_BLOCKLIST_DIRS.iter().map(|s| s.to_string()));

        let mut best_vcs: Option<IndexingSettings> = None;
        let mut best_pathbuf: Option<PathBuf> = None;
        for (vcs, vcs_settings) in &self.vcs_indexing_settings_map {
            let vcs_pathbuf = PathBuf::from(vcs);
            if path.starts_with(&vcs) {
                if best_vcs.is_none() || vcs_pathbuf.components().count() > best_pathbuf.clone().unwrap().components().count() {
                    best_vcs = Some(vcs_settings.clone());
                    best_pathbuf = Some(vcs_pathbuf);
                }
            }
        }

        if let Some(t) = best_vcs {
            result.blocklist.extend(t.blocklist);
            result.additional_indexing_dirs.extend(t.additional_indexing_dirs);
        }

        result
    }
}

pub async fn load_indexing_yaml(
    indexing_yaml_path: &PathBuf,
    relative_path_base: Option<&PathBuf>,
) -> Result<IndexingSettings, String> {
    match fs::read_to_string(&indexing_yaml_path.as_path()).await.map_err(|e| e.to_string()) {
        Ok(content) => {
            match _load_indexing_yaml_str(&content.as_str(), relative_path_base) {
                Ok(indexing_settings) => {
                    return Ok(indexing_settings)
                }
                Err(e) => {
                    return Err(format!("load {} failed\n{}", indexing_yaml_path.display(), e));
                }
            }
        }
        Err(e) => {
            return Err(format!("load {} failed\n{}", indexing_yaml_path.display(), e));
        }
    }
}

pub async fn reload_global_indexing_only(gcx: Arc<ARwLock<GlobalContext>>) -> IndexingEverywhere
{
    let (config_dir, indexing_yaml) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.config_dir.clone(), gcx_locked.cmdline.indexing_yaml.clone())
    };
    let global_indexing_path = if indexing_yaml.is_empty() {
        config_dir.join("indexing.yaml")
    } else {
        canonical_path(indexing_yaml)
    };
    IndexingEverywhere {
        global: load_indexing_yaml(&global_indexing_path, None).await.unwrap_or_default(),
        vcs_indexing_settings_map: HashMap::new(),
        loaded_ts: 0,
    }
}

pub async fn reload_indexing_everywhere_if_needed(gcx: Arc<ARwLock<GlobalContext>>) -> Arc<IndexingEverywhere>
{
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    // Initially this is loaded in _ls_files_under_version_control_recursive()
    let (config_dir, indexing_yaml, workspace_vcs_roots) = {
        let gcx_locked = gcx.read().await;
        if gcx_locked.indexing_everywhere.loaded_ts + INDEXING_TOO_OLD.as_secs() > now {
            return gcx_locked.indexing_everywhere.clone();
        }
        (gcx_locked.config_dir.clone(), gcx_locked.cmdline.indexing_yaml.clone(), gcx_locked.documents_state.workspace_vcs_roots.clone())
    };

    let indexing_everywhere = {
        let global = {
            let global_indexing_path = if indexing_yaml.is_empty() {
                config_dir.join("indexing.yaml")
            } else {
                canonical_path(indexing_yaml)
            };
            load_indexing_yaml(&global_indexing_path, None).await.unwrap_or_else(|e| {
                tracing::error!("cannot load {:?}: {}, fallback to defaults", config_dir, e);
                IndexingSettings::default()
            })
        };

        let vcs_dirs: Vec<PathBuf> = workspace_vcs_roots.lock().unwrap().iter().cloned().collect();
        let mut vcs_indexing_settings_map: HashMap<String, IndexingSettings> = HashMap::new();
        for indexing_root in vcs_dirs {
            let indexing_path = indexing_root.join(".refact").join("indexing.yaml");
            if indexing_path.exists() {
                match load_indexing_yaml(&indexing_path, Some(&indexing_root)).await {
                    Ok(indexing_settings) => {
                        vcs_indexing_settings_map.insert(indexing_root.to_str().unwrap().to_string(), indexing_settings);
                    },
                    Err(e) => {
                        tracing::error!("{}, skip", e);
                    }
                }
            }
        }
        IndexingEverywhere {
            global,
            vcs_indexing_settings_map,
            loaded_ts: now,
        }
    };

    {
        let mut gcx_locked = gcx.write().await;
        gcx_locked.indexing_everywhere = Arc::new(indexing_everywhere);
        gcx_locked.indexing_everywhere.clone()
    }
}

// pub fn is_this_inside_blocklisted_dir(indexing_settings: &IndexingSettings, path: &PathBuf) -> bool {
//     is_blocklisted(&indexing_settings, &path)
// }

pub fn is_blocklisted(indexing_settings: &IndexingSettings, path: &PathBuf) -> bool {
    let block = any_glob_matches_path(&indexing_settings.blocklist, &path);
    // tracing::info!("is_blocklisted {:?} {:?} block={}", indexing_settings, path, block);
    block
}

fn _load_indexing_yaml_str(
    indexing_yaml_str: &str,
    relative_path_base: Option<&PathBuf>,
) -> Result<IndexingSettings, String> {
    match serde_yaml::from_str::<IndexingSettings>(&indexing_yaml_str) {
        Ok(indexing_settings) => {
            let mut additional_indexing_dirs = vec![];
            for indexing_dir in indexing_settings.additional_indexing_dirs.iter() {
                if indexing_dir.is_empty() {
                    continue;
                }
                let expanded_dir = if indexing_dir.starts_with("~") {
                    if let Some(without_tilde) = indexing_dir.strip_prefix("~") {
                        let home_dir = PathBuf::from(&home::home_dir().ok_or(()).expect("failed to find home dir").to_string_lossy().to_string());
                        home_dir.join(without_tilde.trim_start_matches('/')).to_string_lossy().into_owned()
                    } else {
                        indexing_dir.clone()
                    }
                } else {
                    indexing_dir.clone()
                };
                let indexing_dir_path = PathBuf::from(&expanded_dir);
                if indexing_dir_path.is_absolute() {
                    let normalized = crate::files_correction::canonical_path(&expanded_dir)
                        .to_string_lossy()
                        .into_owned();
                    additional_indexing_dirs.push(normalized);
                } else {
                    if let Some(b) = relative_path_base {
                        let joined_path = b.join(&expanded_dir).to_str().unwrap().to_string();
                        let normalized = crate::files_correction::canonical_path(&joined_path)
                            .to_string_lossy()
                            .into_owned();
                        additional_indexing_dirs.push(normalized);
                    } else {
                        tracing::error!("can't have relative path {} in the global indexing.yaml", indexing_dir)
                    }
                }
            }
            return Ok(IndexingSettings {
                blocklist: indexing_settings.blocklist,
                additional_indexing_dirs
            })
        }
        Err(e) => {
            return Err(format!("{}", e));
        }
    }
}
