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
use crate::integrations::integr_abstract::IntegrationTrait;
use crate::integrations::running_integrations::load_integration_tools;
use crate::tools::tools_description::Tool;
use crate::integrations::docker::docker_ssh_tunnel_utils::{SshConfig, forward_remote_docker_if_needed};
use crate::integrations::docker::docker_container_manager::Port;

#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct SettingsDocker {
    pub connect_to_daemon_at: String,
    pub docker_cli_path: String,
    pub ssh_config: Option<SshConfig>,
    pub container_workspace_folder: String,
    pub docker_image_id: String,
    pub host_lsp_path: String,
    pub run_chat_threads_inside_container: bool,
    pub label: String,
    pub command: String,
    pub keep_containers_alive_for_x_minutes: u64,
    pub ports: Vec<Port>,
}

#[derive(Clone, Default, Debug)]
pub struct ToolDocker {
    pub settings_docker: SettingsDocker,
}

impl IntegrationTrait for ToolDocker {
    fn integr_settings_apply(&mut self, value: &Value) -> Result<(), String> {
        match serde_json::from_value::<SettingsDocker>(value.clone()) {
            Ok(settings_docker) => {
                tracing::info!("Docker settings applied: {:?}", settings_docker);
                self.settings_docker = settings_docker
            },
            Err(e) => {
                tracing::error!("Failed to apply settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        Ok(())
    }

    fn integr_settings_as_json(&self) -> Value {
        serde_json::to_value(&self.settings_docker).unwrap()
    }

    fn integr_upgrade_to_tool(&self) -> Box<dyn Tool + Send> {
        Box::new(ToolDocker {
            settings_docker: self.settings_docker.clone()
        }) as Box<dyn Tool + Send>
    }

    fn integr_schema(&self) -> &str
    {
        DOCKER_INTEGRATION_SCHEMA
    }
}

impl ToolDocker {
    pub async fn command_execute(&self, command: &str, gcx: Arc<ARwLock<GlobalContext>>, fail_if_stderr_is_not_empty: bool) -> Result<(String, String), String> 
    {
        let mut command_args = split_command(&command)?;

        if command_is_interactive_or_blocking(&command_args) {
            return Err("Docker commands that are interactive or blocking are not supported".to_string());
        }

        command_append_label_if_creates_resource(&mut command_args, &self.settings_docker.label);

        let docker_host = self.get_docker_host(gcx.clone()).await?;
        let output = Command::new(&self.settings_docker.docker_cli_path)
            .arg("-H")
            .arg(&docker_host)
            .args(&command_args)
            .output()
            .await
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if fail_if_stderr_is_not_empty && !stderr.is_empty() {
            return Err(format!("Error executing command {command}: \n{stderr}"));
        }

        Ok((stdout, stderr))
    }

    pub async fn get_docker_host(&self, gcx: Arc<ARwLock<GlobalContext>>) -> Result<String, String>
    {
        match &self.settings_docker.ssh_config {
            Some(ssh_config) => {
                let local_port = forward_remote_docker_if_needed(&self.settings_docker.connect_to_daemon_at, ssh_config, gcx.clone()).await?;
                Ok(format!("127.0.0.1:{}", local_port))
            },
            None => Ok(self.settings_docker.connect_to_daemon_at.clone()),
        }
    }
}

#[async_trait]
impl Tool for ToolDocker {
    fn as_any(&self) -> &dyn std::any::Any { self }

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
        
        let (stdout, _) = self.command_execute(&command, gcx.clone(), true).await?;

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
    let tools = load_integration_tools(gcx.clone(), "".to_string(), true).await;
    let docker_tool = tools.get("docker").cloned().ok_or("Docker integration not found")?
        .lock().await.as_any().downcast_ref::<ToolDocker>().cloned().unwrap();
    Ok(docker_tool)
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

pub const DOCKER_INTEGRATION_SCHEMA: &str = r#"
fields:
  connect_to_daemon_at:
    f_type: string_long
    f_desc: "The address to connect to the Docker daemon."
    f_default: "unix:///var/run/docker.sock"
  docker_cli_path:
    f_type: string_long
    f_desc: "Path to the Docker CLI executable."
    f_default: "docker"
  ssh_config:
    f_type: object
    f_desc: "SSH configuration for connecting to remote Docker daemons."
    f_fields:
      host:
        f_type: string_long
        f_desc: "The SSH host."
      user:
        f_type: string_short
        f_desc: "The SSH user."
        f_default: "root"
      port:
        f_type: string_short
        f_desc: "The SSH port."
        f_default: "22"
      identity_file:
        f_type: string_short
        f_desc: "Path to the SSH identity file."
  container_workspace_folder:
    f_type: string_long
    f_desc: "The workspace folder inside the container."
    f_default: "/app"
  docker_image_id:
    f_type: string_long
    f_desc: "The Docker image ID to use."
  host_lsp_path:
    f_type: string_long
    f_desc: "Path to the LSP on the host."
    f_default: "/opt/refact/bin/refact-lsp"
  run_chat_threads_inside_container:
    f_type: bool
    f_desc: "Whether to run chat threads inside the container."
    f_default: "false"
  label:
    f_type: string_short
    f_desc: "Label for the Docker container."
    f_default: "refact"
  command:
    f_type: string_long
    f_desc: "Command to run inside the Docker container."
  keep_containers_alive_for_x_minutes:
    f_type: string_short
    f_desc: "How long to keep containers alive in minutes."
    f_default: "60"
  ports:
    f_type: array
    f_desc: "Ports to expose."
available:
  on_your_laptop_possible: true
  when_isolated_possible: false
smartlinks:
  - sl_label: "Test"
    sl_chat:
      - role: "user"
        content: |
          🔧 The docker tool should be visible now. To test the tool, list the running containers, briefly describe the containers and express
          satisfaction and relief if it works, and change nothing. If it doesn't work or the tool isn't available, go through the usual plan in the system prompt.
          The current config file is %CURRENT_CONFIG%.
"#;