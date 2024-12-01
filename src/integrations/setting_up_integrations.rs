use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use regex::Regex;
use serde::Serialize;
use tokio::sync::RwLock as ARwLock;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;

use crate::global_context::GlobalContext;
// use crate::tools::tools_description::Tool;
// use crate::yaml_configs::create_configs::{integrations_enabled_cfg, read_yaml_into_value};


#[derive(Serialize, Default)]
pub struct YamlError {
    pub integr_config_path: String,
    pub error_line: usize,  // starts with 1, zero if invalid
    pub error_msg: String,
}

#[derive(Serialize, Default)]
pub struct IntegrationRecord {
    pub project_path: String,
    pub integr_name: String,
    pub integr_config_path: String,
    pub integr_config_exists: bool,
    pub on_your_laptop: bool,
    pub when_isolated: bool,
    #[serde(skip_serializing)]
    pub config_unparsed: serde_json::Value,
}

#[derive(Serialize, Default)]
pub struct IntegrationWithIconResult {
    pub integrations: Vec<IntegrationRecord>,
    pub error_log: Vec<YamlError>,
}

pub fn read_integrations_d(
    config_folders: &Vec<PathBuf>,
    integrations_yaml_path: &String,
    lst: &[&str],
    error_log: &mut Vec<YamlError>,
) -> Vec<IntegrationRecord> {
    let mut integrations = Vec::new();
    for config_dir in config_folders {
        for integr_name in lst.iter() {
            let path_str = join_config_path(config_dir, integr_name);
            let path = PathBuf::from(path_str.clone());
            let mut rec: IntegrationRecord = Default::default();
            let (_integr_name, project_path) = match split_path_into_project_and_integration(&path) {
                Ok(x) => x,
                Err(e) => {
                    tracing::error!("error deriving project path: {}", e);
                    continue;
                }
            };
            let short_pp = if project_path.is_empty() { format!("global") } else { crate::nicer_logs::last_n_chars(&project_path, 15) };
            rec.project_path = project_path.clone();
            rec.integr_name = integr_name.to_string();
            rec.integr_config_path = path_str.clone();
            rec.integr_config_exists = path.exists();
            if rec.integr_config_exists {
                match fs::read_to_string(&path) {
                    Ok(file_content) => match serde_yaml::from_str::<serde_yaml::Value>(&file_content) {
                        Ok(yaml_value) => {
                            rec.config_unparsed = serde_json::to_value(yaml_value.clone()).unwrap();
                        }
                        Err(e) => {
                            let location = e.location().map(|loc| format!(" at line {}, column {}", loc.line(), loc.column())).unwrap_or_default();
                            error_log.push(YamlError {
                                integr_config_path: path_str.to_string(),
                                error_line: e.location().map(|loc| loc.line()).unwrap_or(0),
                                error_msg: e.to_string(),
                            });
                            tracing::warn!("failed to parse {}{}: {}", path_str, location, e.to_string());
                        }
                    },
                    Err(e) => {
                        error_log.push(YamlError {
                            integr_config_path: path_str.to_string(),
                            error_line: 0,
                            error_msg: e.to_string(),
                        });
                        tracing::warn!("failed to read {}: {}", path_str, e.to_string());
                    }
                }
            } else {
                tracing::info!("{} no config file `{}`", short_pp, integr_name);
            }
            integrations.push(rec);
        }
    }

    let short_yaml = crate::nicer_logs::last_n_chars(integrations_yaml_path, 15);
    match fs::read_to_string(integrations_yaml_path) {
        Ok(content) => match serde_yaml::from_str::<serde_yaml::Value>(&content) {
            Ok(y) => {
                for integr_name in lst.iter() {
                    if let Some(config) = y.get(integr_name) {
                        let mut rec: IntegrationRecord = Default::default();
                        rec.integr_config_path = integrations_yaml_path.clone();
                        rec.integr_name = integr_name.to_string();
                        rec.integr_config_exists = true;
                        rec.config_unparsed = serde_json::to_value(config.clone()).unwrap();
                        integrations.push(rec);
                        tracing::info!("{} has `{}`", short_yaml, integr_name);
                    } else {
                        tracing::info!("{} no section `{}`", short_yaml, integr_name);
                    }
                }
            }
            Err(e) => {
                error_log.push(YamlError {
                    integr_config_path: integrations_yaml_path.clone(),
                    error_line: e.location().map(|loc| loc.line()).unwrap_or(0),
                    error_msg: e.to_string(),
                });
                tracing::warn!("failed to parse {}: {}", integrations_yaml_path, e);
            }
        },
        Err(e) => {
            error_log.push(YamlError {
                integr_config_path: integrations_yaml_path.clone(),
                error_line: 0,
                error_msg: e.to_string(),
            });
            tracing::warn!("failed to read {}: {}", integrations_yaml_path, e);
        }
    };

    for rec in &mut integrations {
        if !rec.integr_config_exists {
            continue;
        }
        if let Some(available) = rec.config_unparsed.get("available").and_then(|v| v.as_object()) {
            rec.on_your_laptop = available.get("on_your_laptop").and_then(|v| v.as_bool()).unwrap_or(false);
            rec.when_isolated = available.get("when_isolated").and_then(|v| v.as_bool()).unwrap_or(false);
        } else {
            let short_pp = if rec.project_path.is_empty() { format!("global") } else { crate::nicer_logs::last_n_chars(&rec.project_path, 15) };
            tracing::info!("{} no 'available' mapping in `{}`", short_pp, rec.integr_name);
        }
    }

    integrations
}


pub async fn get_integrations_yaml_path(gcx: Arc<ARwLock<GlobalContext>>) -> String {
    let gcx_locked = gcx.read().await;
    let r = gcx_locked.cmdline.integrations_yaml.clone();
    if r.is_empty() {
        let config_dir = gcx_locked.config_dir.join("integrations.yaml");
        return config_dir.to_string_lossy().to_string();
    }
    r
}

pub fn join_config_path(config_dir: &PathBuf, integr_name: &str) -> String {
    config_dir.join("integrations.d").join(format!("{}.yaml", integr_name)).to_string_lossy().into_owned()
}

pub async fn config_dirs(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Vec<PathBuf> {
    let (global_dir, workspace_folders_arc, integrations_yaml) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.config_dir.clone(), gcx_locked.documents_state.workspace_folders.clone(), gcx_locked.cmdline.integrations_yaml.clone())
    };
    let mut config_folders = workspace_folders_arc.lock().unwrap().clone();
    config_folders = config_folders.iter().map(|folder| folder.join(".refact")).collect();
    if integrations_yaml.is_empty() {
        config_folders.push(global_dir);
    }
    config_folders
}

pub fn split_path_into_project_and_integration(cfg_path: &PathBuf) -> Result<(String, String), String> {
    let path_str = cfg_path.to_string_lossy();
    let re_per_project = Regex::new(r"^(.*)[\\/]\.refact[\\/](integrations\.d)[\\/](.+)\.yaml$").unwrap();
    let re_global = Regex::new(r"^(.*)[\\/]\.config[\\/](refact[\\/](integrations\.d)[\\/](.+)\.yaml$)").unwrap();

    if let Some(caps) = re_per_project.captures(&path_str) {
        let project_path = caps.get(1).map_or(String::new(), |m| m.as_str().to_string());
        let integr_name = caps.get(3).map_or(String::new(), |m| m.as_str().to_string());
        Ok((integr_name, project_path))
    } else if let Some(caps) = re_global.captures(&path_str) {
        let integr_name = caps.get(4).map_or(String::new(), |m| m.as_str().to_string());
        Ok((integr_name, String::new()))
    } else {
        Err(format!("invalid path: {}", cfg_path.display()))
    }
}

pub async fn integrations_all_with_icons(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> IntegrationWithIconResult {
    let config_folders = config_dirs(gcx.clone()).await;
    let lst: Vec<&str> = crate::integrations::integrations_list();
    let mut error_log: Vec<YamlError> = Vec::new();
    let integrations_yaml_path = get_integrations_yaml_path(gcx.clone()).await;
    let integrations = read_integrations_d(&config_folders, &integrations_yaml_path, &lst, &mut error_log);
    // rec.integr_icon = crate::integrations::icon_from_name(integr_name);
    IntegrationWithIconResult {
        integrations,
        error_log,
    }
}

#[derive(Serialize, Default)]
pub struct IntegrationGetResult {
    pub project_path: String,
    pub integr_name: String,
    pub integr_config_path: String,
    pub integr_schema: serde_json::Value,
    pub integr_values: serde_json::Value,
    pub error_log: Vec<YamlError>,
}

pub async fn integration_config_get(
    integr_config_path: String,
) -> Result<IntegrationGetResult, String> {
    let sanitized_path = crate::files_correction::canonical_path(&integr_config_path);
    let integr_name = sanitized_path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
    if integr_name.is_empty() {
        return Err(format!("can't derive integration name from file name"));
    }

    let (integr_name, project_path) = split_path_into_project_and_integration(&sanitized_path)?;
    let mut result = IntegrationGetResult {
        project_path: project_path.clone(),
        integr_name: integr_name.clone(),
        integr_config_path: integr_config_path.clone(),
        integr_schema: serde_json::Value::Null,
        integr_values: serde_json::Value::Null,
        error_log: Vec::new(),
    };

    let mut integration_box = crate::integrations::integration_from_name(integr_name.as_str())?;
    result.integr_schema = {
        let y: serde_yaml::Value = serde_yaml::from_str(integration_box.integr_schema()).unwrap();
        let j = serde_json::to_value(y).unwrap();
        j
    };

    let mut available = serde_json::json!({
        "on_your_laptop": false,
        "when_isolated": false
    });
    if sanitized_path.exists() {
        match fs::read_to_string(&sanitized_path) {
            Ok(content) => {
                match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                    Ok(y) => {
                        let j = serde_json::to_value(y).unwrap();
                        available["on_your_laptop"] = j.get("available").and_then(|v| v.get("on_your_laptop")).and_then(|v| v.as_bool()).unwrap_or(false).into();
                        available["when_isolated"] = j.get("available").and_then(|v| v.get("when_isolated")).and_then(|v| v.as_bool()).unwrap_or(false).into();
                        let did_it_work = integration_box.integr_settings_apply(&j);
                        if let Err(e) = did_it_work {
                            tracing::error!("oops: {}", e);
                        }
                    }
                    Err(e) => {
                        return Err(format!("failed to parse: {}", e.to_string()));
                    }
                };
            }
            Err(e) => {
                return Err(format!("failed to read configuration file: {}", e.to_string()));
            }
        };
    }

    result.integr_values = integration_box.integr_settings_as_json();
    result.integr_values["available"] = available;
    Ok(result)
}

pub async fn integration_config_save(
    integr_config_path: &String,
    integr_values: &serde_json::Value,
) -> Result<(), String> {
    let config_path = crate::files_correction::canonical_path(integr_config_path);
    let (integr_name, _project_path) = crate::integrations::setting_up_integrations::split_path_into_project_and_integration(&config_path)
        .map_err(|e| format!("Failed to split path: {}", e))?;
    let mut integration_box = crate::integrations::integration_from_name(integr_name.as_str())
        .map_err(|e| format!("Failed to load integrations: {}", e))?;

    integration_box.integr_settings_apply(integr_values)?;  // this will produce "no field XXX" errors

    let mut sanitized_json: serde_json::Value = integration_box.integr_settings_as_json();
    tracing::info!("posted values:\n{}", serde_json::to_string_pretty(integr_values).unwrap());
    if !sanitized_json.as_object_mut().unwrap().contains_key("available") {
        sanitized_json["available"] = serde_json::Value::Object(serde_json::Map::new());
    }
    sanitized_json["available"]["on_your_laptop"] = integr_values.pointer("/available/on_your_laptop").cloned().unwrap_or(serde_json::Value::Bool(false));
    sanitized_json["available"]["when_isolated"] = integr_values.pointer("/available/when_isolated").cloned().unwrap_or(serde_json::Value::Bool(false));
    tracing::info!("writing to {}:\n{}", config_path.display(), serde_json::to_string_pretty(&sanitized_json).unwrap());
    let sanitized_yaml = serde_yaml::to_value(sanitized_json).unwrap();

    let config_dir = config_path.parent().ok_or_else(|| {
        "Failed to get parent directory".to_string()
    })?;
    async_fs::create_dir_all(config_dir).await.map_err(|e| {
        format!("Failed to create {}: {}", config_dir.display(), e)
    })?;

    let mut file = async_fs::File::create(&config_path).await.map_err(|e| {
        format!("Failed to create {}: {}", config_path.display(), e)
    })?;
    let sanitized_yaml_string = serde_yaml::to_string(&sanitized_yaml).unwrap();
    file.write_all(sanitized_yaml_string.as_bytes()).await.map_err(|e| {
        format!("Failed to write to {}: {}", config_path.display(), e)
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    // use super::*;
    use crate::integrations::integr_abstract::IntegrationTrait;
    use crate::integrations::yaml_schema::ISchema;
    use serde_yaml;
    use indexmap::IndexMap;
    use std::fs::File;
    use std::io::Write;

    #[tokio::test]
    async fn test_integration_schemas() {
        let integrations = crate::integrations::integrations_list();
        for name in integrations {
            let mut integration_box = crate::integrations::integration_from_name(name).unwrap();
            let schema_json = {
                let y: serde_yaml::Value = serde_yaml::from_str(integration_box.integr_schema()).unwrap();
                let j = serde_json::to_value(y).unwrap();
                j
            };
            let schema_yaml: serde_yaml::Value = serde_json::from_value(schema_json.clone()).unwrap();
            let compare_me1 = serde_yaml::to_string(&schema_yaml).unwrap();
            let schema_struct: ISchema = serde_json::from_value(schema_json).unwrap();
            let schema_struct_yaml = serde_json::to_value(&schema_struct).unwrap();
            let compare_me2 = serde_yaml::to_string(&schema_struct_yaml).unwrap();
            if compare_me1 != compare_me2 {
                eprintln!("schema mismatch for integration `{}`:\nOriginal:\n{}\nSerialized:\n{}", name, compare_me1, compare_me2);
                let original_file_path = format!("/tmp/original_schema_{}.yaml", name);
                let serialized_file_path = format!("/tmp/serialized_schema_{}.yaml", name);
                let mut original_file = File::create(&original_file_path).unwrap();
                let mut serialized_file = File::create(&serialized_file_path).unwrap();
                original_file.write_all(compare_me1.as_bytes()).unwrap();
                serialized_file.write_all(compare_me2.as_bytes()).unwrap();
                eprintln!("cat {}", original_file_path);
                eprintln!("cat {}", serialized_file_path);
                eprintln!("diff {} {}", original_file_path, serialized_file_path);
                panic!("oops");
            }
        }
    }
}
