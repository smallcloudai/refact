use std::sync::Arc;
use std::collections::HashMap;
use tokio::process::Command;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ContextEnum};
use crate::global_context::GlobalContext;
use crate::tools::tools_description::{read_integrations_yaml, Tool};
use crate::integrations::docker::docker_ssh_tunnel_utils::{SshConfig, forward_remote_docker_if_needed};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct IntegrationDocker {
    #[serde(default = "default_connect_to_daemon_at")]
    pub connect_to_daemon_at: String,
    #[serde(default = "default_docker_cli_path")]
    pub docker_cli_path: String,
    pub ssh_config: Option<SshConfig>,
    #[serde(default = "default_container_workspace_folder")]
    pub container_workspace_folder: String,
    #[serde(default)]
    pub docker_image_id: String,
    #[serde(default = "default_host_lsp_path")]
    pub host_lsp_path: String,
    #[serde(default)]
    pub run_chat_threads_inside_container: bool,
    #[serde(default = "default_label")]
    pub label: String,
    #[serde(default)]
    pub command: String,
    #[serde(default = "default_keep_containers_alive_for_x_minutes")]
    pub keep_containers_alive_for_x_minutes: u64,
}
fn default_connect_to_daemon_at() -> String { "unix:///var/run/docker.sock".to_string() }
fn default_docker_cli_path() -> String { "docker".to_string() }
fn default_container_workspace_folder() -> String { "/app".to_string() }
fn default_host_lsp_path() -> String { "/opt/refact/bin/refact-lsp".to_string() }
fn default_label() -> String { "refact".to_string() }
fn default_keep_containers_alive_for_x_minutes() -> u64 { 60 }

pub struct ToolDocker {
    pub integration_docker: IntegrationDocker,
}

impl ToolDocker {
    pub fn new_from_yaml(docker_config: &serde_yaml::Value) -> Result<Self, String> {
        let integration_docker = serde_yaml::from_value::<IntegrationDocker>(docker_config.clone())
            .map_err(|e| {
                let location = e.location().map(|loc| format!(" at line {}, column {}", loc.line(), loc.column())).unwrap_or_default();
                format!("{}{}", e.to_string(), location)
            })?;
        Ok(Self { integration_docker })
    }

    pub async fn command_execute(&self, command: &str, gcx: Arc<ARwLock<GlobalContext>>) -> Result<String, String> 
    {
        let mut command_args = split_command(&command)?;

        if command_is_interactive_or_blocking(&command_args) {
            return Err("Docker commands that are interactive or blocking are not supported".to_string());
        }

        command_append_label_if_creates_resource(&mut command_args, &self.integration_docker.label);

        let docker_host = if let Some(ssh_config) = &self.integration_docker.ssh_config {
            let local_port = forward_remote_docker_if_needed(&self.integration_docker.connect_to_daemon_at, ssh_config, gcx.clone()).await?;
            format!("127.0.0.1:{}", local_port)
        } else {
            self.integration_docker.connect_to_daemon_at.clone()
        };

        let output = Command::new(&self.integration_docker.docker_cli_path)
            .arg("-H")
            .arg(&docker_host)
            .args(&command_args)
            .output()
            .await
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !stderr.is_empty() {
            return Err(format!("Error running command '{command}': {stderr}"));
        }

        Ok(stdout)
    }
}

#[async_trait]
impl Tool for ToolDocker {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {

        let command = parse_command(args)?;

        let gcx = {
            let ccx_locked = ccx.lock().await;
            ccx_locked.global_context.clone()
        };
        
        let stdout = self.command_execute(&command, gcx.clone()).await?;

        Ok((false, vec![
            ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText(stdout),
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                ..Default::default()
            }),
        ]))
    }

    fn command_to_match_against_confirm_deny(
        &self,
        args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        let command = parse_command(args)?;
        let mut command_args = split_command(&command)?;
        command_args.insert(0, "docker".to_string());
        Ok(command_args.join(" "))
    }
}

pub async fn docker_tool_load(gcx: Arc<ARwLock<GlobalContext>>) -> Result<ToolDocker, String> {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let integrations_yaml = read_integrations_yaml(&cache_dir).await?;
    let docker_config = integrations_yaml.get("docker")
        .ok_or_else(|| "No docker integration found in integrations.yaml".to_string())?;
    Ok(ToolDocker::new_from_yaml(docker_config)?)
}

fn parse_command(args: &HashMap<String, Value>) -> Result<String, String>{
    return match args.get("command") {
        Some(Value::String(s)) => Ok(s.to_string()),
        Some(v) => Err(format!("argument `command` is not a string: {:?}", v)),
        None => Err("Missing argument `command`".to_string())
    };
}

fn split_command(command: &str) -> Result<Vec<String>, String> {
    let mut parsed_args = shell_words::split(&command).map_err(|e| e.to_string())?;
    if parsed_args.is_empty() {
        return Err("Parsed command is empty".to_string());
    }
    if parsed_args[0] == "docker" {
        parsed_args.remove(0);
    }
    Ok(parsed_args)
}

fn command_is_interactive_or_blocking(command_args: &Vec<String>) -> bool 
{
    const COMMANDS_THAT_CAN_BE_INTERACTIVE: &[&str] = &["run", "exec"];
    const COMMANDS_ALWAYS_BLOCKING: &[&str] = &["attach", "events", "wait"];

    fn command_contains_flag(command_args: &Vec<String>, short_flag: &str, long_flag: &str) -> bool
    {
        for arg in command_args {
            if !short_flag.is_empty() && arg.starts_with("-") && !arg.starts_with("--") && arg.contains(short_flag) {
                return true;
            }
            if !long_flag.is_empty() && arg == format!("--{}", long_flag).as_str() {
                return true;
            }
        }
        false
    }

    let mut command_args_iter = command_args.iter().filter(|arg| !arg.starts_with('-'));
    let subcommand_generic = command_args_iter.next().map(|arg| arg.as_str()).unwrap_or("");

    let subcommand_specific = if subcommand_generic == "container" {
        command_args_iter.next().map(|arg| arg.as_str()).unwrap_or("")
    } else {
        subcommand_generic
    };

    if COMMANDS_THAT_CAN_BE_INTERACTIVE.contains(&subcommand_specific) && 
        command_contains_flag(command_args, "i", "interactive") 
    {
        return true;
    }

    if subcommand_specific == "logs" && command_contains_flag(command_args, "f", "follow") { 
        return true;
    }

    if subcommand_specific == "stats" && !command_contains_flag(command_args, "", "no-stream") {
        return true;
    }

    COMMANDS_ALWAYS_BLOCKING.contains(&subcommand_specific)
}

fn command_append_label_if_creates_resource(command_args: &mut Vec<String>, label: &str) -> () {
    const COMMANDS_FOR_RESOURCE_CREATION: &[&[&str]] = &[
        &["build"], 
        &["buildx", "build"], 
        &["image", "build"], 
        &["builder", "build"], 
        &["buildx", "b"], 
        &["create"],
        &["container", "create"],
        &["network", "create"], 
        &["volume", "create"],
        &["run"],
        &["container", "run"],
    ];

    for prefix in COMMANDS_FOR_RESOURCE_CREATION {
        let prefix_vec: Vec<String> = prefix.iter().map(|s| s.to_string()).collect();
        if command_args.starts_with( &prefix_vec) {
            let insert_pos = prefix.len();
            command_args.insert(insert_pos, format!("--label={}", label));
            break;
        }
    }
}