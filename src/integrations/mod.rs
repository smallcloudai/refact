// use std::path::PathBuf;
// use std::sync::Arc;
// use indexmap::IndexMap;
// use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};

// use crate::global_context::GlobalContext;
// use crate::tools::tools_description::Tool;
// use crate::yaml_configs::create_configs::{integrations_enabled_cfg, read_yaml_into_value};


pub mod integr_abstract;
pub mod integr_github;
pub mod integr_gitlab;
pub mod integr_pdb;
pub mod integr_chrome;
pub mod integr_postgres;
pub mod integr_mysql;
pub mod integr_cmdline;
pub mod integr_cmdline_service;
pub mod integr_shell;
//pub mod integr_mcp;

pub mod process_io_utils;
pub mod docker;
pub mod sessions;
pub mod config_chat;
pub mod project_summary_chat;
pub mod yaml_schema;
pub mod setting_up_integrations;
pub mod running_integrations;
pub mod utils;

use integr_abstract::IntegrationTrait;


pub fn integration_from_name(n: &str) -> Result<Box<dyn IntegrationTrait + Send + Sync>, String>
{
    match n {
        "github" => Ok(Box::new(integr_github::ToolGithub { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        "gitlab" => Ok(Box::new(integr_gitlab::ToolGitlab { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        "pdb" => Ok(Box::new(integr_pdb::ToolPdb { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        "chrome" => Ok(Box::new(integr_chrome::ToolChrome { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        "postgres" => Ok(Box::new(integr_postgres::ToolPostgres { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        "mysql" => Ok(Box::new(integr_mysql::ToolMysql { ..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        "docker" => Ok(Box::new(docker::integr_docker::ToolDocker {..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
        "shell" => Ok(Box::new(integr_shell::ToolShell {..Default::default() }) as Box<dyn IntegrationTrait + Send + Sync>),
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

pub fn integrations_list(allow_experimental: bool) -> Vec<&'static str> {
    let mut integrations = vec![
        "github",
        "gitlab",
        "pdb",
        "chrome",
        "postgres",
        "mysql",
        "cmdline_TEMPLATE",
        "service_TEMPLATE",
        "docker",
        "shell",
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
