use std::sync::Arc;
use std::collections::HashMap;
use tokio::process::Command;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use async_trait::async_trait;
use tracing::error;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ContextEnum, ChatMessage};
use crate::global_context::GlobalContext;
use crate::tools::tools_description::Tool;
use crate::integrations::docker::docker_ssh_tunnel_utils::{SshConfig, forward_remote_docker_if_needed};

const COMMON_LABEL: &str = "humberto-refact";

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct IntegrationDocker {
    pub connect_to_daemon_at: String,   // 127.0.0.1:1337
    pub docker_cli_path: Option<String>,
    pub ssh_config: Option<SshConfig>,
}

pub struct ToolDocker {
    integration_docker: IntegrationDocker,
}

impl ToolDocker {
    pub async fn new_if_configured(integrations_value: &serde_yaml::Value, gcx: Arc<ARwLock<GlobalContext>>) -> Result<Self, String> {
        let integration_docker_value = integrations_value.get("docker")
            .ok_or_else(|| "Docker integration is not configured").cloned()?;
    
        let integration_docker = serde_yaml::from_value::<IntegrationDocker>(integration_docker_value)
            .map_err(|e| e.to_string())?;

        if let Some(ssh_config) = &integration_docker.ssh_config 
        {
            forward_remote_docker_if_needed(&integration_docker.connect_to_daemon_at, ssh_config, gcx.clone()).await?;
        }
    
        Ok(Self { integration_docker })
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
        let command_args = parse_command_args(args)?;

        if command_is_interactive_or_blocking(&command_args) {
            return Err("Docker commands that are interactive or blocking are not supported".to_string());
        }

        let docker_cli_command = self.integration_docker.docker_cli_path.as_deref().unwrap_or("docker");
        
        let mut docker_host = self.integration_docker.connect_to_daemon_at.clone();
        let gcx = {
            ccx.lock().await.global_context.clone()
        };
        let ssh_tunnel_arc = {
            gcx.read().await.docker_ssh_tunnel.clone()
        };
        {
            let ssh_tunnel_locked = ssh_tunnel_arc.lock().await;
            if let Some(ssh_tunnel) = &*ssh_tunnel_locked {
                docker_host = format!("tcp://localhost:{}", ssh_tunnel.local_port);
            }
        };
        
        let output = Command::new(docker_cli_command)
            .arg("-H")
            .arg(&docker_host)
            .args(&command_args)
            .output()
            .await
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !stderr.is_empty() {
            error!("Error: {:?}", stderr);
            return Err(stderr);
        }

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: stdout,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok((false, results))
    }

    fn command_to_match_against_confirm_deny(
        &self,
        args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        let mut command_args = parse_command_args(args)?;
        command_args.insert(0, "docker".to_string());
        Ok(command_args.join(" "))
    }
}

fn parse_command_args(args: &HashMap<String, Value>) -> Result<Vec<String>, String> {
    let command = match args.get("command") {
        Some(Value::String(s)) => s,
        Some(v) => return Err(format!("argument `command` is not a string: {:?}", v)),
        None => return Err("Missing argument `command`".to_string())
    };

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

    let subcommand = if command_args.len() >= 2 && command_args[0] == "container" { 
        command_args[1].as_str()
    } else { 
        command_args[0].as_str()
    };

    if COMMANDS_THAT_CAN_BE_INTERACTIVE.contains(&subcommand) && 
        command_contains_flag(command_args, "i", "interactive") 
    {
        return true;
    }

    if subcommand == "logs" && command_contains_flag(command_args, "f", "follow") { 
        return true;
    }

    if subcommand == "stats" && !command_contains_flag(command_args, "", "no-stream") {
        return true;
    }

    COMMANDS_ALWAYS_BLOCKING.contains(&subcommand)
}

