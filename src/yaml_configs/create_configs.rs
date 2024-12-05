use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock as ARwLock;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use sha2::{Sha256, Digest};
use serde_yaml;
use std::path::{Path, PathBuf};
use crate::global_context::GlobalContext;
// use crate::integrations::{get_empty_integrations, get_integration_path};


const DEFAULT_CHECKSUM_FILE: &str = "default-checksums.yaml";


pub async fn yaml_configs_try_create_all(gcx: Arc<ARwLock<GlobalContext>>) -> String {
    let mut results = Vec::new();
    let cache_dir = gcx.read().await.cache_dir.clone();

    let files = vec![
        ("bring-your-own-key.yaml", crate::caps::BRING_YOUR_OWN_KEY_SAMPLE),
        ("customization.yaml", crate::yaml_configs::customization_compiled_in::COMPILED_IN_INITIAL_USER_YAML),
        ("privacy.yaml", crate::privacy_compiled_in::COMPILED_IN_INITIAL_PRIVACY_YAML),
        ("integrations.yaml", crate::integrations::INTEGRATIONS_DEFAULT_YAML),
    ];

    for (file_name, content) in files {
        let file_path = cache_dir.join(file_name);
        if let Err(e) = _yaml_file_exists_or_create(gcx.clone(), &file_path, content).await {
            tracing::warn!("{}", e);
            results.push(format!("Error processing {:?}: {}", file_path, e));
        } else {
            results.push(file_path.to_string_lossy().to_string());
        }
    }

    let integrations_d = cache_dir.join("integrations.d");
    if let Err(e) = tokio::fs::create_dir_all(&integrations_d).await {
        tracing::warn!("Failed to create directory {:?}: {}", integrations_d, e);
        results.push(format!("Error creating directory {:?}: {}", integrations_d, e));
    }

    // let integrations_enabled = cache_dir.join("integrations-enabled.yaml");
    // let integrations = get_empty_integrations();

    // for (file_name, content) in integrations.iter().map(|(k, v)| (k.clone(), v.integr_settings_default())) {
    //     let file_path = get_integration_path(&cache_dir, &file_name);
    //     if let Err(e) = _yaml_file_exists_or_create(gcx.clone(), &file_path, &content).await {
    //         tracing::warn!("{}", e);
    //         results.push(format!("Error processing {:?}: {}", file_path, e));
    //     } else {
    //         results.push(file_path.to_string_lossy().to_string());
    //     }
    //     let integr_name = file_path.file_stem().unwrap().to_string_lossy().to_string();
    //     let mut enabled_cfg = integrations_enabled_cfg(&integrations_enabled).await;
    //     if let None = enabled_cfg.get(&integr_name) {
    //         if let serde_yaml::Value::Mapping(ref mut map) = enabled_cfg {
    //             map.insert(serde_yaml::Value::String(integr_name), serde_yaml::Value::Bool(false));
    //         }
    //         if let Err(e) = write_yaml_value(&integrations_enabled, &enabled_cfg).await {
    //             error!("Failed to write {}: {}", integrations_enabled.display(), e);
    //             panic!("{}", e);
    //         }
    //     }
    // }

    results.get(0).cloned().unwrap_or_default()
}

async fn _yaml_file_exists_or_create(
    gcx: Arc<ARwLock<GlobalContext>>,
    config_path: &PathBuf,
    the_default: &str
) -> Result<String, String>
{
    let cache_dir = gcx.read().await.cache_dir.clone();
    let config_path_str = config_path.to_string_lossy().to_string();
    let config_name = config_path.file_name().ok_or_else(|| format!("{} is not a file", config_path.display()))?.to_string_lossy().to_string();

    let checksums_dict = read_checksums(&cache_dir).await?;

    if config_path.exists() {
        let existing_content = tokio::fs::read_to_string(&config_path).await
            .map_err(|e| format!("failed to read {}: {}", config_name, e))?;
        if existing_content == the_default {
            // normal exit, content == default
            return Ok(config_path_str);
        }
        let existing_checksum = calculate_checksum(&existing_content);
        if existing_checksum == checksums_dict.get(&config_name).map(|s| s.as_str()).unwrap_or("") {
            tracing::info!("\n * * * detected that {} is a default config from a previous version of this binary, no changes made by human, overwrite * * *\n", config_path.display());
        } else {
            // normal exit, config changed by user
            return Ok(config_path_str);
        }
    }

    let mut f = File::create(&config_path).await
        .map_err(|e| format!("failed to create {}: {}", config_name, e))?;
    f.write_all(the_default.as_bytes()).await
        .map_err(|e| format!("failed to write into {}: {}", config_name, e))?;
    tracing::info!("created {}", config_path.display());

    let new_checksum = calculate_checksum(the_default);
    update_checksum(&cache_dir, config_name.to_string(), &new_checksum).await?;

    Ok(config_path_str)
}

fn calculate_checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

async fn read_checksums(cache_dir: &Path) -> Result<HashMap<String, String>, String> {
    let checksum_path = cache_dir.join(DEFAULT_CHECKSUM_FILE);
    if checksum_path.exists() {
        let content = tokio::fs::read_to_string(&checksum_path).await
            .map_err(|e| format!("failed to read {}: {}", DEFAULT_CHECKSUM_FILE, e))?;
        let checksums: HashMap<String, String> = serde_yaml::from_str(&content)
            .map_err(|e| format!("failed to parse {}: {}", DEFAULT_CHECKSUM_FILE, e))?;
        Ok(checksums)
    } else {
        Ok(HashMap::new())
    }
}

async fn update_checksum(cache_dir: &Path, config_name: String, checksum: &str) -> Result<(), String> {
    let checksum_path = cache_dir.join(DEFAULT_CHECKSUM_FILE);
    let mut checksums = read_checksums(&cache_dir).await?;
    checksums.insert(config_name.to_string(), checksum.to_string());
    let content = format!(
        "# This file allows to determine whether a config file still has the default text, so we can upgrade it.\n#\n{}",
        serde_yaml::to_string(&checksums).unwrap()
    );
    tokio::fs::write(&checksum_path, content).await
        .map_err(|e| format!("failed to write {}: {}", DEFAULT_CHECKSUM_FILE, e))?;
    Ok(())
}

// pub async fn integrations_enabled_cfg(
//     integrations_enabled_path: &PathBuf,
// ) -> serde_yaml::Value {
//     read_yaml_into_value(integrations_enabled_path).await.unwrap_or_else(|_| serde_yaml::Value::Mapping(Default::default()))
// }

// pub async fn read_yaml_into_value(yaml_path: &PathBuf) -> Result<serde_yaml::Value, String> {
//     let file = std::fs::File::open(&yaml_path).map_err(
//         |e| format!("Failed to open {}: {}", yaml_path.display(), e)
//     )?;

//     let reader = std::io::BufReader::new(file);
//     serde_yaml::from_reader(reader).map_err(
//         |e| {
//             let location = e.location().map(|loc| format!(" at line {}, column {}", loc.line(), loc.column())).unwrap_or_default();
//             format!("Failed to parse {}{}: {}", yaml_path.display(), location, e)
//         }
//     )
// }

// pub async fn write_yaml_value(path: &Path, value: &serde_yaml::Value) -> Result<(), String> {
//     let content = serde_yaml::to_string(value).map_err(|e| format!("Failed to serialize YAML: {}", e))?;

//     let mut file = tokio::fs::OpenOptions::new()
//         .write(true)
//         .truncate(true)
//         .create(true)
//         .open(path)
//         .await
//         .map_err(|e| format!("Failed to open file {}: {}", path.display(), e))?;

//     AsyncWriteExt::write_all(&mut file, content.as_bytes()).await
//         .map_err(|e| format!("Failed to write to file {}: {}", path.display(), e))
// }
