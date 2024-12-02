// use std::path::PathBuf;
// use std::sync::Arc;
// use indexmap::IndexMap;
// use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};

// use crate::global_context::GlobalContext;
// use crate::tools::tools_description::Tool;
// use crate::yaml_configs::create_configs::{integrations_enabled_cfg, read_yaml_into_value};


pub mod integr_abstract;
// pub mod integr_github;
// pub mod integr_gitlab;
// pub mod integr_pdb;
// pub mod integr_chrome;
pub mod integr_postgres;
pub mod integr_cmdline;

pub mod process_io_utils;
pub mod docker;
pub mod sessions;
pub mod config_chat;
pub mod yaml_schema;
pub mod setting_up_integrations;
pub mod running_integrations;

use integr_abstract::IntegrationTrait;


pub fn integration_from_name(n: &str) -> Result<Box<dyn IntegrationTrait + Send + Sync>, String>
{
    match n {
        // "github" => Ok(Box::new(ToolGithub { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        // "gitlab" => Ok(Box::new(ToolGitlab { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        // "pdb" => Ok(Box::new(ToolPdb { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        "postgres" => Ok(Box::new(integr_postgres::ToolPostgres { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        // "chrome" => Ok(Box::new(ToolChrome { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        "docker" => Ok(Box::new(docker::integr_docker::ToolDocker {..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        cmdline if cmdline.starts_with("cmdline_") => {
            let tool_name = cmdline.strip_prefix("cmdline_").unwrap();
            tracing::info!("todo finish me tool_name={}", tool_name);
            Err("todo finish me".to_string())
        },
        service if service.starts_with("service_") => {
            let tool_name = service.strip_prefix("service_").unwrap();
            tracing::info!("todo finish me service tool_name={}", tool_name);
            Err("todo finish me".to_string())
        },
        _ => Err(format!("Unknown integration name: {}", n)),
    }
}

pub fn icon_from_name(n: &str) -> String
{
    // match n {
    //     // "github" => Box::new(ToolGithub { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>,
    //     // "gitlab" => Box::new(ToolGitlab { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>,
    //     // "pdb" => Box::new(ToolPdb { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>,
    //     "postgres" => Box::new(integr_postgres::ToolPostgres { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>,
    //     // "chrome" => Box::new(ToolChrome { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>,
    //     _ => panic!("Unknown integration name: {}", n),
    // }
    return "".to_string();
}

pub fn integrations_list() -> Vec<&'static str> {
    vec![
        // "github",
        // "gitlab",
        // "pdb",
        "postgres",
        // "chrome",
        "docker"
    ]
}

pub fn go_to_configuration_message(integration_name: &str) -> String {
    format!("🧩 for configuration go to SETTINGS:{integration_name}")
}



// pub fn get_integration_path(cache_dir: &PathBuf, name: &str) -> PathBuf {
//     cache_dir.join("integrations.d").join(format!("{}.yaml", name))
// }


// pub async fn validate_integration_value(name: &str, value: serde_yaml::Value) -> Result<serde_yaml::Value, String> {
//     let integrations = get_empty_integrations();
//     match integrations.get(name) {
//         Some(i) => {
//             let j_value: serde_json::Value = i.integr_yaml2json(&value)?;
//             let yaml_value: serde_yaml::Value = serde_yaml::to_value(&j_value).map_err(|e| e.to_string())?;
//             Ok(yaml_value)
//         },
//         None => Err(format!("Integration {} is not defined", name))
//     }
// }

// pub async fn load_integration_tools(
//     gcx: Arc<ARwLock<GlobalContext>>,
// ) -> IndexMap<String, Arc<AMutex<Box<dyn Tool + Send>>>> {
//     let paths = integrations_paths(gcx.clone()).await;
//     let integrations_yaml_value = {
//         let cache_dir = gcx.read().await.cache_dir.clone();
//         let yaml_path = cache_dir.join("integrations.yaml");
//         read_yaml_into_value(&yaml_path).await?
//     };
//     let cache_dir = gcx.read().await.cache_dir.clone();
//     // let enabled_path = cache_dir.join("integrations-enabled.yaml");
//     // let enabled = match integrations_enabled_cfg(&enabled_path).await {
//     //     serde_yaml::Value::Mapping(map) => map.into_iter().filter_map(|(k, v)| {
//     //         if let (serde_yaml::Value::String(key), serde_yaml::Value::Bool(value)) = (k, v) {
//     //             Some((key, value))
//     //         } else {
//     //             None
//     //         }
//     //     }).collect::<std::collections::HashMap<String, bool>>(),
//     //     _ => std::collections::HashMap::new(),
//     // };

//     let integrations = get_integrations(gcx.clone()).await?;

//     let mut tools = IndexMap::new();
//     for (i_name, i) in integrations.iter() {
//         // if !enabled.get(i_name).unwrap_or(&false) {
//         //     info!("Integration {} is disabled", i_name);
//         //     continue;
//         // }
//         let tool = i.integr_upgrade_to_tool();
//         tools.insert(i_name.clone(), Arc::new(AMutex::new(tool)));
//     }
//     Ok(tools)
// }

// pub async fn json_for_integration(
//     yaml_path: &PathBuf,
//     value_from_integrations: Option<&serde_yaml::Value>,
//     integration: &Box<dyn IntegrationTrait + Send + Sync>,
// ) -> Result<serde_json::Value, String> {
//     let tool_name = integration.integr_name().clone();

//     let value = if yaml_path.exists() {
//         match read_yaml_into_value(yaml_path).await {
//             Ok(value) => integration.integr_yaml2json(&value).unwrap_or_else(|e| {
//                 let e = format!("Problem converting integration to JSON: {}", e);
//                 json!({"detail": e.to_string()})
//             }),
//             Err(e) => {
//                 let e = format!("Problem reading YAML from {}: {}", yaml_path.display(), e);
//                 json!({"detail": e.to_string()})
//             }
//         }
//     } else {
//         json!({"detail": format!("Cannot read {}. Probably, file does not exist", yaml_path.display())})
//     };

//     let value_from_integrations = value_from_integrations.map_or(json!({"detail": format!("tool {tool_name} is not defined in integrations.yaml")}), |value| {
//         integration.integr_yaml2json(value).unwrap_or_else(|e| {
//             let e = format!("Problem converting integration to JSON: {}", e);
//             json!({"detail": e.to_string()})
//         })
//     });

//     match (value.get("detail"), value_from_integrations.get("detail")) {
//         (None, None) => {
//             Err(format!("Tool {tool_name} exists in both {tool_name}.yaml and integrations.yaml. Consider removing one of them."))
//         },
//         (Some(_), None) => {
//             Ok(value_from_integrations)
//         },
//         (None, Some(_)) => {
//             Ok(value)
//         }
//         (Some(_), Some(_)) => {
//             Ok(value)
//         }
//     }

//     Ok(())
// }

// async fn load_tool_from_yaml<T: Tool + IntegrationTrait + Send + 'static>(
//     yaml_path: Option<&PathBuf>,
//     tool_constructor: fn(&serde_yaml::Value) -> Result<T, String>,
//     value_from_integrations: Option<&serde_yaml::Value>,
//     enabled: Option<&bool>,
//     integrations: &mut IndexMap<String, Arc<AMutex<Box<dyn Tool + Send>>>>,
// ) -> Result<(), String> {
//     let yaml_path = yaml_path.as_ref().expect("No yaml path");
//     let tool_name = yaml_path.file_stem().expect("No file name").to_str().expect("No file name").to_string();
//     if !enabled.unwrap_or(&false) {
//         tracing::info!("Integration {} is disabled", tool_name);
//         return Ok(());
//     }
//     let tool = if yaml_path.exists() {
//         match read_yaml_into_value(yaml_path).await {
//             Ok(value) => {
//                 match tool_constructor(&value) {
//                     Ok(tool) => {
//                         // integrations.insert(tool_name, Arc::new(AMutex::new(Box::new(tool) as Box<dyn Tool + Send>)));
//                         Some(tool)
//                     }
//                     Err(e) => {
//                         tracing::warn!("Problem in {}: {}", yaml_path.display(), e);
//                         None
//                     }
//                 }
//             }
//             Err(e) => {
//                 tracing::warn!("Problem reading {:?}: {}", yaml_path, e);
//                 None
//             }
//         }
//     } else {
//         None
//     };

//     let tool_from_integrations = value_from_integrations
//         .and_then(|value| match tool_constructor(&value) {
//             Ok(tool) => Some(tool),
//             Err(_) => None
//         });

//     match (tool, tool_from_integrations) {
//         (Some(_), Some(_)) => {
//             return Err(format!("Tool {tool_name} exists in both {tool_name}.yaml and integrations.yaml. Consider removing one of them."));
//         },
//         (Some(tool), None) | (None, Some(tool)) => {
//             integrations.insert(tool_name.clone(), Arc::new(AMutex::new(Box::new(tool) as Box<dyn Tool + Send>)));
//         },
//         _ => {}
//     }

//     Ok(())
// }

pub const INTEGRATIONS_DEFAULT_YAML: &str = r#"# This file is used to configure integrations in Refact Agent.
# If there is a syntax error in this file, no integrations will work.
#
# Here you can set up which commands require confirmation or must be denied. If both apply, the command is denied.
# Rules use glob patterns for wildcard matching (https://en.wikipedia.org/wiki/Glob_(programming))
#

commands_need_confirmation:
  - "gh * delete*"
  - "glab * delete*"
  - "psql*[!SELECT]*"
commands_deny:
  - "docker* rm *"
  - "docker* remove *"
  - "docker* rmi *"
  - "docker* pause *"
  - "docker* stop *"
  - "docker* kill *"
  - "gh auth token*"
  - "glab auth token*"


# Command line: things you can call and immediately get an answer
#cmdline:
#  run_make:
#    command: "make"
#    command_workdir: "%project_path%"
#    timeout: 600
#    description: "Run `make` inside a C/C++ project, or a similar project with a Makefile."
#    parameters:    # this is what the model needs to produce, you can use %parameter% in command and workdir
#      - name: "project_path"
#        description: "absolute path to the project"
#    output_filter:                   # output filter is optional, can help if the output is very long to reduce it, preserving valuable information
#      limit_lines: 50
#      limit_chars: 10000
#      valuable_top_or_bottom: "top"  # the useful infomation more likely to be at the top or bottom? (default "top")
#      grep: "(?i)error|warning"      # in contrast to regular grep this doesn't remove other lines from output, just prefers matching when approaching limit_lines or limit_chars (default "(?i)error")
#      grep_context_lines: 5          # leave that many lines around a grep match (default 5)
#      remove_from_output: "process didn't exit"    # some lines are very long and unwanted, this is also a regular expression (default "")

#cmdline_services:
#  manage_py_runserver:
#    command: "python manage.py runserver"
#    command_workdir: "%project_path%"
#    description: "Start or stop `python manage.py runserver` running in the background"
#    parameters:
#      - name: "project_path"
#        description: "absolute path to the project"
#    startup_wait: 10
#    startup_wait_port: 8000


# --- Docker integration ---
docker:
  docker_daemon_address: "unix:///var/run/docker.sock"  # Path to the Docker daemon. For remote Docker, the path to the daemon on the remote server.
  # docker_cli_path: "/usr/local/bin/docker"  # Uncomment to set a custom path for the docker cli, defaults to "docker"

  # Uncomment the following to connect to a remote Docker daemon
  # Docker and necessary ports will be forwarded for container communication. No additional commands will be executed over SSH.
  # ssh_config:
  #   host: "<your_server_domain_or_ip_here>"
  #   user: "root"
  #   port: 22
  #   identity_file: "~/.ssh/id_rsa"

  run_chat_threads_inside_container: false

  # The folder inside the container where the workspace is mounted, refact-lsp will start there, defaults to "/app"
  # container_workspace_folder: "/app"

  # Image ID for running containers, which can later be selected in the UI before starting a chat thread.
  # docker_image_id: "079b939b3ea1"

  # Map container ports to local ports
  # ports:
  #   - local_port: 4000
  #     container_port: 3000

  # Path to the LSP binary on the host machine, to be bound into the containers.
  host_lsp_path: "/opt/refact/bin/refact-lsp"

  # Will be added as a label to containers, images, and other resources created by Refact Agent, defaults to "refact"
  label: "refact"

  # Uncomment to execute a command inside the container when the thread starts. Regardless, refact-lsp will run independently of this setting.
  # command: "npm run dev"

  # The time in minutes that the containers will be kept alive while not interacting with the chat thread, defaults to 60.
  keep_containers_alive_for_x_minutes: 60
"#;
