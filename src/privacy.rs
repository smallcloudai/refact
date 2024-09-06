use std::sync::Arc;
use std::path::Path;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;
use tokio::time::Duration;
use tokio::fs;
use tracing::{error, info};
use glob::Pattern;
use std::time::SystemTime;
use tokio::io::AsyncWriteExt;

use crate::global_context::GlobalContext;
use crate::privacy_compiled_in::COMPILED_IN_INITIAL_PRIVACY_YAML;

#[derive(Debug, PartialEq, PartialOrd)]
pub enum FilePrivacyLevel {
    Blocked = 0,
    OnlySendToServersIControl = 1,
    AllowToSendEverywhere = 2,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PrivacySettings {
    pub file_privacy: FilePrivacySettings,
    #[serde(default = "default_expiry_time", skip)]
    pub expiry_time: u64,
}

const PRIVACY_RELOAD_EACH_N_SECONDS: Duration = Duration::from_secs(10);
fn default_expiry_time() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() + PRIVACY_RELOAD_EACH_N_SECONDS.as_secs()
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FilePrivacySettings {
    #[serde(default)]
    pub only_send_to_servers_I_control: Vec<String>,
    #[serde(default)]
    pub blocked: Vec<String>,
}

// TODO: Move to other yaml files handling once that part is finished
async fn read_privacy_yaml(path: &Path) -> PrivacySettings {
    match fs::read_to_string(&path).await {
        Ok(content) => {
            match serde_yaml::from_str(&content) {
                Ok(privacy_settings) => {
                    info!("privacy settings loaded from {}", path.display());
                    privacy_settings
                }
                Err(e) => {
                    error!("failed to deserialize YAML from {}: {}, no privacy settings will be used", path.display(), e);
                    PrivacySettings::default()
                }
            }
        }
        Err(e) => {
            error!("failed to read content from {}: {}, no privacy settings will be used", path.display(), e);
            PrivacySettings::default()
        }
    }
}

// TODO: Move to other yaml files handling once that part is finished
pub async fn load_privacy_if_needed(gcx: Arc<ARwLock<GlobalContext>>) -> Arc<PrivacySettings> {
    let path = {
        let gcx_locked = gcx.read().await;
        let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let should_reload = gcx_locked.privacy_settings.expiry_time <= current_time;
        if !should_reload {
            return gcx_locked.privacy_settings.clone();
        }
        gcx_locked.cache_dir.join("privacy.yaml")
    };

    if !path.exists() {
        match fs::File::create(&path).await {
            Ok(mut file) => {
                if let Err(e) = file.write_all(COMPILED_IN_INITIAL_PRIVACY_YAML.as_bytes()).await {
                    error!("Failed to write to file: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to create file: {}", e);
            }
        }
    }

    let new_privacy_settings = read_privacy_yaml(&path).await;

    {
        let mut gcx_locked = gcx.write().await;
        gcx_locked.privacy_settings = Arc::new(new_privacy_settings);
        gcx_locked.privacy_settings.clone()
    }
}

fn any_glob_matches_path(globs: &Vec<String>, path: &Path) -> bool {
    globs.iter().any(|glob| {
        let pattern = Pattern::new(glob).unwrap();
        pattern.matches_path(path)
    })
}

fn get_file_privacy_level(privacy_settings: Arc<PrivacySettings>, path: &Path) -> FilePrivacyLevel
{
    if any_glob_matches_path(&privacy_settings.file_privacy.blocked, path) {
        FilePrivacyLevel::Blocked
    } else if any_glob_matches_path(&privacy_settings.file_privacy.only_send_to_servers_I_control, path) {
        FilePrivacyLevel::OnlySendToServersIControl
    } else {
        FilePrivacyLevel::AllowToSendEverywhere
    }
}

pub fn check_file_privacy(privacy_settings: Arc<PrivacySettings>, path: &Path, min_allowed_privacy_level: &FilePrivacyLevel) -> Result<(), String>
{
    let file_privacy_level = get_file_privacy_level(privacy_settings.clone(), path);
    if file_privacy_level < *min_allowed_privacy_level {
        return Err(format!("access to {} is restricted level={:?}", path.display(), file_privacy_level));
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, sync::Arc};

    #[test]
    fn test_get_file_privacy_level() {
        // Arrange
        let privacy_settings = Arc::new(PrivacySettings {
            file_privacy: FilePrivacySettings {
                only_send_to_servers_I_control: vec!["*.cat.txt".to_string(), "*.md".to_string(), "*/.venv/*".to_string(), "**/tests_dir/**/*".to_string()],
                blocked: vec!["*/make.png".to_string(), "*.txt".to_string()],
            },
            expiry_time: default_expiry_time(),
        });

        let current_dir = std::env::current_dir().unwrap();

        // Cases to test
        let cases: Vec<(PathBuf, FilePrivacyLevel)> = vec![
            (current_dir.join("test.txt"), FilePrivacyLevel::Blocked),
            (current_dir.join("test.cat.txt"), FilePrivacyLevel::Blocked),
            (current_dir.join("car/steps/make.png"), FilePrivacyLevel::Blocked),
            (current_dir.join("build/rename.md"), FilePrivacyLevel::OnlySendToServersIControl),
            (current_dir.join(".venv/bin/activate"), FilePrivacyLevel::OnlySendToServersIControl),
            (PathBuf::from("/home/user/.venv/bin/activate"), FilePrivacyLevel::OnlySendToServersIControl),
            (PathBuf::from("/home/user/venv/bin/activate"), FilePrivacyLevel::AllowToSendEverywhere),
            (current_dir.join("car/steps/load.make.png"), FilePrivacyLevel::AllowToSendEverywhere),
            (current_dir.join("test.cat.txt.zip"), FilePrivacyLevel::AllowToSendEverywhere),
            (current_dir.join("car/steps/make.pngs"), FilePrivacyLevel::AllowToSendEverywhere),
            (current_dir.join("car/tests_dir/cars.rs"), FilePrivacyLevel::OnlySendToServersIControl),
            (current_dir.join("car/tests_dir/bul/tar/gip.rs"), FilePrivacyLevel::OnlySendToServersIControl),
            (current_dir.join("tests_dir/.hidden"), FilePrivacyLevel::OnlySendToServersIControl),
            (current_dir.join("/tests_dir/.hidden"), FilePrivacyLevel::OnlySendToServersIControl),
        ];

        // Act and assert
        for (path, expected_privacy_level) in cases {
            let actual_privacy_level = get_file_privacy_level(privacy_settings.clone(), &path);
            assert_eq!(
                actual_privacy_level,
                expected_privacy_level,
                "Testing get_file_privacy_level with path {} and expected privacy level {:?}, got {:?}",
                path.display(),
                expected_privacy_level,
                actual_privacy_level,
            );
        }
    }

    #[test]
    fn test_check_file_privacy() {
        // Arrange
        let privacy_settings = Arc::new(PrivacySettings {
            file_privacy: FilePrivacySettings {
                only_send_to_servers_I_control: vec!["*.cat.txt".to_string(), "*.md".to_string(), "*/.venv/*".to_string(), "**/tests_dir/**/*".to_string()],
                blocked: vec!["*/make.png".to_string(), "*.txt".to_string()],
            },
            expiry_time: default_expiry_time(),
        });

        let current_dir = std::env::current_dir().unwrap();

        // Cases to test
        let cases: Vec<(PathBuf, FilePrivacyLevel, bool)> = vec![
            (current_dir.join("test.zip"), FilePrivacyLevel::AllowToSendEverywhere, true),
            (current_dir.join("test.md"), FilePrivacyLevel::AllowToSendEverywhere, false),
            (current_dir.join("test.md"), FilePrivacyLevel::OnlySendToServersIControl, true),
            (current_dir.join("test.cat.txt"), FilePrivacyLevel::OnlySendToServersIControl, false),
        ];

        // Act and assert: check_file_privacy
        for (path, expected_privacy_level, expected_result) in &cases {
            let result = check_file_privacy(privacy_settings.clone(), path, expected_privacy_level);
            if *expected_result {
                assert!(
                    result.is_ok(),
                    "Testing check_file_privacy with path {} and expected privacy level {:?}, got {:?} and it should have been ok",
                    path.display(),
                    expected_privacy_level,
                    result.unwrap_err(),
                );
            } else {
                assert!(
                    result.is_err(),
                    "Testing check_file_privacy with path {} and expected privacy level {:?}, got {:?} and it should have been err",
                    path.display(),
                    expected_privacy_level,
                    result.unwrap(),
                );
            }
        }
    }
}








