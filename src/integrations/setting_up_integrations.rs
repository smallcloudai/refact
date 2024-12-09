use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
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

#[derive(Serialize, Default, Debug)]
pub struct IntegrationRecord {
    pub project_path: String,
    pub integr_name: String,
    pub integr_config_path: String,
    pub integr_config_exists: bool,
    pub icon_path: String,
    pub on_your_laptop: bool,
    pub when_isolated: bool,
    #[serde(skip_serializing)]
    pub config_unparsed: serde_json::Value,
}

#[derive(Serialize, Default)]
pub struct IntegrationResult {
    pub integrations: Vec<IntegrationRecord>,
    pub error_log: Vec<YamlError>,
}

pub fn read_integrations_d(
    config_dirs: &Vec<PathBuf>,
    global_config_dir: &PathBuf,
    integrations_yaml_path: &String,
    vars_for_replacements: &HashMap<String, String>,
    lst: &[&str],
    error_log: &mut Vec<YamlError>,
) -> Vec<IntegrationRecord> {
    let mut result = Vec::new();

    // 1. Read each of config_dirs
    let mut files_to_read = Vec::new();
    let mut project_config_dirs = config_dirs.iter().map(|dir| dir.to_string_lossy().to_string()).collect::<Vec<String>>();
    if integrations_yaml_path.is_empty() {
        project_config_dirs.push("".to_string());  // global
    }

    for project_config_dir in project_config_dirs {
        // Read config_folder/integr_name.yaml and make a record, even if the file doesn't exist
        let config_dir = if project_config_dir == "" { global_config_dir.clone() } else { PathBuf::from(project_config_dir.clone()) };
        for integr_name in lst.iter() {
            let path_str = join_config_path(&config_dir, integr_name);
            let path = PathBuf::from(path_str.clone());
            let (_integr_name, project_path) = match split_path_into_project_and_integration(&path) {
                Ok(x) => x,
                Err(e) => {
                    tracing::error!("error deriving project path: {}", e);
                    continue;
                }
            };
            files_to_read.push((path_str, integr_name.to_string(), project_path));
        }
        // Find special files that start with cmdline_* and service_*
        if let Ok(entries) = fs::read_dir(config_dir.join("integrations.d")) {
            let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();
                if !file_name_str.ends_with(".yaml") {
                    continue;
                }
                let file_name_str_no_yaml = file_name_str.trim_end_matches(".yaml").to_string();
                if file_name_str.starts_with("cmdline_") || file_name_str.starts_with("service_") {
                    files_to_read.push((entry.path().to_string_lossy().to_string(), file_name_str_no_yaml.to_string(), project_config_dir.clone()));
                }
            }
        }
    }

    for (path_str, integr_name, project_path) in files_to_read {
        let path = PathBuf::from(&path_str);
        // let short_pp = if project_path.is_empty() { format!("global") } else { crate::nicer_logs::last_n_chars(&project_path, 15) };
        let mut rec: IntegrationRecord = Default::default();
        rec.project_path = project_path.clone();
        rec.integr_name = integr_name.clone();
        rec.icon_path = format!("/integration-icon/{integr_name}.png");
        rec.integr_config_path = path_str.clone();
        rec.integr_config_exists = path.exists();
        if rec.integr_config_exists {
            match fs::read_to_string(&path) {
                Ok(file_content) => match serde_yaml::from_str::<serde_yaml::Value>(&file_content) {
                    Ok(yaml_value) => {
                        // tracing::info!("{} has {}", short_pp, integr_name);
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
            // tracing::info!("{} no config file for {}", short_pp, integr_name);
        }
        result.push(rec);
    }

    // 2. Read single file integrations_yaml_path, sections in yaml become integrations
    // The --integrations-yaml flag disables the global config folder in (1)
    if !integrations_yaml_path.is_empty() {
        let short_yaml = crate::nicer_logs::last_n_chars(integrations_yaml_path, 15);
        match fs::read_to_string(integrations_yaml_path) {
            Ok(content) => match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                Ok(y) => {
                    if let Some(mapping) = y.as_mapping() {
                        for (key, value) in mapping {
                            if let Some(key_str) = key.as_str() {
                                if key_str.starts_with("cmdline_") || key_str.starts_with("service_") {
                                    let mut rec: IntegrationRecord = Default::default();
                                    rec.integr_config_path = integrations_yaml_path.clone();
                                    rec.integr_name = key_str.to_string();
                                    rec.icon_path = format!("/integration-icon/{key_str}.png");
                                    rec.integr_config_exists = true;
                                    rec.config_unparsed = serde_json::to_value(value.clone()).unwrap();
                                    result.push(rec);
                                    tracing::info!("{} detected prefix `{}`", short_yaml, key_str);
                                } else if lst.contains(&key_str) {
                                    let mut rec: IntegrationRecord = Default::default();
                                    rec.integr_config_path = integrations_yaml_path.clone();
                                    rec.integr_name = key_str.to_string();
                                    rec.icon_path = format!("/integration-icon/{key_str}.png");
                                    rec.integr_config_exists = true;
                                    rec.config_unparsed = serde_json::to_value(value.clone()).unwrap();
                                    result.push(rec);
                                    tracing::info!("{} has `{}`", short_yaml, key_str);
                                } else {
                                    tracing::warn!("{} unrecognized section `{}`", short_yaml, key_str);
                                }
                            }
                        }
                    } else {
                        tracing::warn!("{} is not a mapping", short_yaml);
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
    }

    // 3. Replace vars in config_unparsed
    for rec in &mut result {
        if let serde_json::Value::Object(map) = &mut rec.config_unparsed {
            for (_key, value) in map.iter_mut() {
                if let Some(str_value) = value.as_str() {
                    let replaced_value = vars_for_replacements.iter().fold(str_value.to_string(), |acc, (var, replacement)| {
                        acc.replace(&format!("${}", var), replacement)
                    });
                    *value = serde_json::Value::String(replaced_value);
                }
            }
        }
    }

    // 4. Fill on_your_laptop/when_isolated in each record
    for rec in &mut result {
        if !rec.integr_config_exists {
            continue;
        }
        if let Some(available) = rec.config_unparsed.get("available").and_then(|v| v.as_object()) {
            rec.on_your_laptop = available.get("on_your_laptop").and_then(|v| v.as_bool()).unwrap_or(false);
            rec.when_isolated = available.get("when_isolated").and_then(|v| v.as_bool()).unwrap_or(false);
        } else {
            // let short_pp = if rec.project_path.is_empty() { format!("global") } else { crate::nicer_logs::last_n_chars(&rec.project_path, 15) };
            // tracing::info!("{} no 'available' mapping in `{}` will default to true", short_pp, rec.integr_name);
            rec.on_your_laptop = true;
            rec.when_isolated = true;
        }
    }

    result
}


pub async fn get_integrations_yaml_path(gcx: Arc<ARwLock<GlobalContext>>) -> String {
    let gcx_locked = gcx.read().await;
    let r = gcx_locked.cmdline.integrations_yaml.clone();
    // if r.is_empty() {
    //     let config_dir = gcx_locked.config_dir.join("integrations.yaml");
    //     return config_dir.to_string_lossy().to_string();
    // }
    r
}

pub async fn get_vars_for_replacements(gcx: Arc<ARwLock<GlobalContext>>) -> HashMap<String, String>
{
    let gcx_locked = gcx.read().await;
    let secrets_yaml_path = gcx_locked.config_dir.join("secrets.yaml");
    let variables_yaml_path = gcx_locked.config_dir.join("variables.yaml");
    let mut variables = HashMap::new();
    if let Ok(secrets_content) = fs::read_to_string(&secrets_yaml_path) {
        if let Ok(secrets_yaml) = serde_yaml::from_str::<HashMap<String, String>>(&secrets_content) {
            variables.extend(secrets_yaml);
        } else {
            tracing::warn!("cannot parse secrets.yaml");
        }
    } else {
        tracing::info!("cannot read secrets.yaml");
    }
    if let Ok(variables_content) = fs::read_to_string(&variables_yaml_path) {
        if let Ok(variables_yaml) = serde_yaml::from_str::<HashMap<String, String>>(&variables_content) {
            variables.extend(variables_yaml);
        } else {
            tracing::warn!("cannot parse variables.yaml");
        }
    } else {
        tracing::info!("cannot read variables.yaml");
    }
    variables
}

pub fn join_config_path(config_dir: &PathBuf, integr_name: &str) -> String
{
    config_dir.join("integrations.d").join(format!("{}.yaml", integr_name)).to_string_lossy().into_owned()
}

pub async fn get_config_dirs(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> (Vec<PathBuf>, PathBuf) {
    let (global_config_dir, workspace_folders_arc, _integrations_yaml) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.config_dir.clone(), gcx_locked.documents_state.workspace_folders.clone(), gcx_locked.cmdline.integrations_yaml.clone())
    };
    let mut config_dirs = workspace_folders_arc.lock().unwrap().clone();
    config_dirs = config_dirs.iter().map(|dir| dir.join(".refact")).collect();
    (config_dirs, global_config_dir)
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

pub async fn integrations_all(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> IntegrationResult {
    let (config_dirs, global_config_dir) = get_config_dirs(gcx.clone()).await;
    let lst: Vec<&str> = crate::integrations::integrations_list();
    let mut error_log: Vec<YamlError> = Vec::new();
    let integrations_yaml_path = get_integrations_yaml_path(gcx.clone()).await;
    let vars_for_replacements = get_vars_for_replacements(gcx.clone()).await;
    let integrations = read_integrations_d(&config_dirs, &global_config_dir, &integrations_yaml_path, &vars_for_replacements, &lst, &mut error_log);
    IntegrationResult {
        integrations,
        error_log,
    }
}

#[derive(Serialize, Default)]
pub struct IntegrationGetResult {
    pub project_path: String,
    pub integr_name: String,
    pub integr_config_path: String,
    pub integr_config_exists: bool,
    pub integr_schema: serde_json::Value,
    pub integr_values: serde_json::Value,
    pub error_log: Vec<YamlError>,
}

pub async fn integration_config_get(
    integr_config_path: String,
) -> Result<IntegrationGetResult, String> {
    let sanitized_path = crate::files_correction::canonical_path(&integr_config_path);
    let exists = sanitized_path.exists();
    let integr_name = sanitized_path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
    if integr_name.is_empty() {
        return Err(format!("can't derive integration name from file name"));
    }

    let (integr_name, project_path) = split_path_into_project_and_integration(&sanitized_path)?;
    let mut result = IntegrationGetResult {
        project_path: project_path.clone(),
        integr_name: integr_name.clone(),
        integr_config_path: integr_config_path.clone(),
        integr_config_exists: exists,
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
    if exists {
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
    use crate::integrations::yaml_schema::ISchema;
    use serde_yaml;
    use std::fs::File;
    use std::io::Write;

    #[tokio::test]
    async fn test_integration_schemas() {
        let integrations = crate::integrations::integrations_list();
        for name in integrations {
            let integration_box = crate::integrations::integration_from_name(name).unwrap();
            let schema_json = {
                let y: serde_yaml::Value = serde_yaml::from_str(integration_box.integr_schema()).unwrap();
                let j = serde_json::to_value(y).unwrap();
                j
            };
            let schema_yaml: serde_yaml::Value = serde_json::from_value(schema_json.clone()).unwrap();
            let compare_me1 = serde_yaml::to_string(&schema_yaml).unwrap();
            eprintln!("schema_json {:?}", schema_json);
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
