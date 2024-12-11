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
pub mod integr_chrome;
pub mod integr_postgres;
pub mod integr_cmdline;
pub mod integr_cmdline_service;

pub mod process_io_utils;
pub mod docker;
pub mod sessions;
pub mod config_chat;
pub mod project_summary_chat;
pub mod yaml_schema;
pub mod setting_up_integrations;
pub mod running_integrations;
pub mod utils;

use integr_abstract::{IntegrationTrait, IntegrationCommon};


pub fn integration_from_name(n: &str) -> Result<Box<dyn IntegrationTrait + Send + Sync>, String>
{
    match n {
        // "github" => Ok(Box::new(ToolGithub { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        // "gitlab" => Ok(Box::new(ToolGitlab { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        // "pdb" => Ok(Box::new(ToolPdb { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        "postgres" => Ok(Box::new(integr_postgres::ToolPostgres { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        "chrome" => Ok(Box::new(integr_chrome::ToolChrome { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        "docker" => Ok(Box::new(docker::integr_docker::ToolDocker {..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        cmdline if cmdline.starts_with("cmdline_") => {
            // let tool_name = cmdline.strip_prefix("cmdline_").unwrap();
            Ok(Box::new(integr_cmdline::ToolCmdline {..Default::default()}) as Box<dyn IntegrationTrait + Send + Sync>)
        },
        service if service.starts_with("service_") => {
            // let tool_name = service.strip_prefix("service_").unwrap();
            Ok(Box::new(integr_cmdline_service::ToolService {..Default::default()}) as Box<dyn IntegrationTrait + Send + Sync>)
        },
        "isolation" => Ok(Box::new(docker::integr_isolation::IntegrationIsolation {..Default::default()}) as Box<dyn IntegrationTrait + Send + Sync>),
        _ => Err(format!("Unknown integration name: {}", n)),
    }
}

#[allow(dead_code)]
pub fn icon_from_name(_n: &str) -> String
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

pub fn integrations_list(allow_experimental: bool) -> Vec<&'static str> {
    let mut integrations = vec![
        // "github",
        // "gitlab",
        // "pdb",
        "postgres",
        "chrome",
        "cmdline_TEMPLATE",
        "service_TEMPLATE",
        // "chrome",
        "docker",
    ];
    if allow_experimental {
        integrations.extend(vec![
            "isolation",
        ]);
    }
    integrations
}

pub fn go_to_configuration_message(integration_name: &str) -> String {
    format!("🧩 for configuration go to SETTINGS:{integration_name}")
}


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
