use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
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

#[derive(Serialize, Default, Debug, Clone)]
pub struct IntegrationRecord {
    pub project_path: String,
    pub integr_name: String,
    pub integr_config_path: String,
    pub integr_config_exists: bool,
    pub icon_path: String,
    pub on_your_laptop: bool,
    pub when_isolated: bool,
    pub ask_user: Vec<String>,
    pub deny: Vec<String>,
    #[serde(skip_serializing)]
    pub config_unparsed: serde_json::Value,
}

#[derive(Serialize, Default)]
pub struct IntegrationResult {
    pub integrations: Vec<IntegrationRecord>,
    pub error_log: Vec<YamlError>,
}

fn get_array_of_str_or_empty(val: &serde_json::Value, path: &str) -> Vec<String> {
    val.pointer(path)
        .and_then(|val| {
            val.as_array().map(|array| {
                array
                    .iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect::<Vec<String>>()
            })
        })
        .unwrap_or_default()
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
                let (_integr_name, project_path) = match split_path_into_project_and_integration(&entry.path()) {
                    Ok(x) => x,
                    Err(e) => {
                        tracing::error!("error deriving project path: {}", e);
                        continue;
                    }
                };
                tracing::info!("XXX file_name_str={:?}", file_name_str);
                if file_name_str.starts_with("cmdline_") || file_name_str.starts_with("service_") || file_name_str.starts_with("mcp_") {
                    tracing::info!("XXXX file_name_str={:?}", file_name_str);
                    files_to_read.push((entry.path().to_string_lossy().to_string(), file_name_str_no_yaml.to_string(), project_path));
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

    // 5. Fill confirmation in each record
    for rec in &mut result {
        if let Some(confirmation) = rec.config_unparsed.get("confirmation") {
            rec.ask_user = get_array_of_str_or_empty(&confirmation, "/ask_user");
            rec.deny = get_array_of_str_or_empty(&confirmation, "/deny");
        } else {
            let schema = match crate::integrations::integration_from_name(rec.integr_name.as_str()) {
                Ok(i) => {
                    serde_json::to_value(
                        serde_yaml::from_str::<serde_yaml::Value>(i.integr_schema()).expect("schema is invalid")
                    ).expect("schema is invalid")
                }
                Err(err) => {
                    tracing::warn!("failed to retrieve schema from {}: {err}", rec.integr_name);
                    continue;
                }
            };
            rec.ask_user = get_array_of_str_or_empty(&schema, "/confirmation/ask_user_default");
            rec.deny = get_array_of_str_or_empty(&schema, "/confirmation/deny_default");
        }
    }

    result
}

pub async fn get_vars_for_replacements(
    gcx: Arc<ARwLock<GlobalContext>>,
    error_log: &mut Vec<YamlError>,
) -> HashMap<String, String> {
    let (config_dir, variables_yaml) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.config_dir.clone(), gcx_locked.cmdline.variables_yaml.clone())
    };
    let secrets_yaml_path = config_dir.join("secrets.yaml");
    let variables_yaml_path = if variables_yaml.is_empty() {
        config_dir.join("variables.yaml")
    } else {
        crate::files_correction::to_pathbuf_normalize(&variables_yaml)
    };
    let mut variables = HashMap::new();

    // Helper function to read and parse a YAML file
    async fn read_and_parse_yaml(
        path: &PathBuf,
        error_log: &mut Vec<YamlError>,
    ) -> Result<HashMap<String, String>, ()> {
        if !path.exists() {
            return Ok(HashMap::new());
        }

        match fs::read_to_string(path) {
            Ok(content) => match serde_yaml::from_str::<HashMap<String, String>>(&content) {
                Ok(parsed_yaml) => Ok(parsed_yaml),
                Err(e) => {
                    tracing::warn!("Failed to parse {}: {}", path.display(), e);
                    error_log.push(YamlError {
                        integr_config_path: path.to_string_lossy().to_string(),
                        error_line: e.location().map(|loc| loc.line()).unwrap_or(0),
                        error_msg: format!("Failed to parse {}: {}", path.display(), e),
                    });
                    Err(())
                }
            },
            Err(e) => {
                tracing::info!("Failed to read {}: {}", path.display(), e);
                error_log.push(YamlError {
                    integr_config_path: path.to_string_lossy().to_string(),
                    error_line: 0,
                    error_msg: format!("Failed to read {}: {}", path.display(), e),
                });
                Err(())
            }
        }
    }

    // Read and parse secrets.yaml
    if let Ok(secrets_yaml) = read_and_parse_yaml(&secrets_yaml_path, error_log).await {
        variables.extend(secrets_yaml);
    }

    // Read and parse variables.yaml
    if let Ok(variables_yaml) = read_and_parse_yaml(&variables_yaml_path, error_log).await {
        variables.extend(variables_yaml);
    }

    variables
}

pub fn join_config_path(config_dir: &PathBuf, integr_name: &str) -> String
{
    config_dir.join("integrations.d").join(format!("{}.yaml", integr_name)).to_string_lossy().into_owned()
}

pub async fn get_config_dirs(
    gcx: Arc<ARwLock<GlobalContext>>,
    current_project_path: &Option<PathBuf>
) -> (Vec<PathBuf>, PathBuf) {
    let (global_config_dir, workspace_folders_arc, workspace_vcs_roots_arc, _integrations_yaml) = {
        let gcx_locked = gcx.read().await;
        (
            gcx_locked.config_dir.clone(),
            gcx_locked.documents_state.workspace_folders.clone(),
            gcx_locked.documents_state.workspace_vcs_roots.clone(),
            gcx_locked.cmdline.integrations_yaml.clone(),
        )
    };

    let mut workspace_folders = workspace_folders_arc.lock().unwrap().clone();
    if let Some(current_project_path) = current_project_path {
        workspace_folders = workspace_folders.into_iter()
            .filter(|folder| current_project_path.starts_with(&folder)).collect::<Vec<_>>();
    }
    let workspace_vcs_roots = workspace_vcs_roots_arc.lock().unwrap().clone();

    let mut config_dirs = Vec::new();

    for folder in workspace_folders {
        let vcs_roots: Vec<PathBuf> = workspace_vcs_roots
            .iter()
            .filter(|root| root.starts_with(&folder))
            .cloned()
            .collect();

        if !vcs_roots.is_empty() {
            // it has any workspace_vcs_roots => take them as projects
            for root in vcs_roots {
                config_dirs.push(root.join(".refact"));
            }
        } else {
            // it doesn't => use workspace_folder itself
            // probably we see this because it's a new project that doesn't have version control yet, but added to the workspace already
            config_dirs.push(folder.join(".refact"));
        }
    }

    config_dirs.sort();
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
    let (config_dirs, global_config_dir) = get_config_dirs(gcx.clone(), &None).await;
    let (allow_experimental, integrations_yaml_path) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.cmdline.experimental, gcx_locked.cmdline.integrations_yaml.clone())
    };
    let lst: Vec<&str> = crate::integrations::integrations_list(allow_experimental);
    let mut error_log: Vec<YamlError> = Vec::new();
    let vars_for_replacements = get_vars_for_replacements(gcx.clone(), &mut error_log).await;
    let integrations = read_integrations_d(&config_dirs, &global_config_dir, &integrations_yaml_path, &vars_for_replacements, &lst, &mut error_log);
    IntegrationResult { integrations, error_log }
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

    let (integr_name, project_path) = split_path_into_project_and_integration(&sanitized_path)?;
    let better_integr_config_path = sanitized_path.to_string_lossy().to_string();
    let mut result = IntegrationGetResult {
        project_path: project_path.clone(),
        integr_name: integr_name.clone(),
        integr_config_path: better_integr_config_path.clone(),
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

    if exists {
        match fs::read_to_string(&sanitized_path) {
            Ok(content) => {
                match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                    Ok(y) => {
                        let j = serde_json::to_value(y).unwrap();
                        match integration_box.integr_settings_apply(&j, better_integr_config_path.clone()) {
                            Ok(_) => {
                            }
                            Err(err) => {
                                result.error_log.push(YamlError {
                                    integr_config_path: better_integr_config_path.clone(),
                                    error_line: 0,
                                    error_msg: err.to_string(),
                                });
                                tracing::warn!("cannot deserialize some fields in the integration cfg {better_integr_config_path}: {err}");
                            }
                        }
                        let common_settings = integration_box.integr_common();
                        result.integr_values = integration_box.integr_settings_as_json();
                        result.integr_values["available"]["on_your_laptop"] = common_settings.available.on_your_laptop.into();
                        result.integr_values["available"]["when_isolated"] = common_settings.available.when_isolated.into();
                        result.integr_values["confirmation"]["ask_user"] = common_settings.confirmation.ask_user.into();
                        result.integr_values["confirmation"]["deny"] = common_settings.confirmation.deny.into();
                    }
                    Err(err) => {
                        result.error_log.push(YamlError {
                            integr_config_path: better_integr_config_path.clone(),
                            error_line: err.location().map(|loc| loc.line()).unwrap_or(0),
                            error_msg: err.to_string(),
                        });
                        tracing::warn!("cannot parse {better_integr_config_path}: {err}");
                        return Ok(result);
                    }
                };
            }
            Err(e) => {
                return Err(format!("failed to read configuration file: {}", e.to_string()));
            }
        };
    }
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

    integration_box.integr_settings_apply(integr_values, integr_config_path.clone())?;  // this will produce "no field XXX" errors
    let mut sanitized_json: serde_json::Value = integration_box.integr_settings_as_json();
    let common_settings = integration_box.integr_common();
    if let (Value::Object(sanitized_json_m), Value::Object(common_settings_m)) = (&mut sanitized_json, json!(common_settings)) {
        sanitized_json_m.extend(common_settings_m);
    }

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
        let integrations = crate::integrations::integrations_list(true);
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
