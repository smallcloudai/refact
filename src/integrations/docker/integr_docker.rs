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
use crate::integrations::integr_abstract::{IntegrationTrait, IntegrationCommon, IntegrationConfirmation};
use crate::tools::tools_description::Tool;
use crate::integrations::docker::docker_ssh_tunnel_utils::{SshConfig, forward_remote_docker_if_needed};
use crate::integrations::utils::{serialize_num_to_str, deserialize_str_to_num};

#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct SettingsDocker {
    pub label: String,
    pub docker_daemon_address: String,
    pub docker_cli_path: String,
    pub remote_docker: bool,
    pub ssh_host: String,
    pub ssh_user: String,
    #[serde(serialize_with = "serialize_num_to_str", deserialize_with = "deserialize_str_to_num")]
    pub ssh_port: u16,
    pub ssh_identity_file: String,
}

impl SettingsDocker {
    pub fn get_ssh_config(&self) -> Option<SshConfig> {
        if self.remote_docker {
            Some(SshConfig {
                host: self.ssh_host.clone(),
                user: self.ssh_user.clone(),
                port: self.ssh_port.clone(),
                identity_file: if !self.ssh_identity_file.is_empty()
                    { Some(self.ssh_identity_file.clone()) } else { None },
            })
        } else {
            None
        }
    }
}

#[derive(Clone, Default)]
pub struct ToolDocker {
    pub common:  IntegrationCommon,
    pub settings_docker: SettingsDocker,
    pub config_path: String,
}

#[async_trait]
impl IntegrationTrait for ToolDocker {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn integr_settings_apply(&mut self, value: &Value, config_path: String) -> Result<(), String> {
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
        match serde_json::from_value::<IntegrationCommon>(value.clone()) {
            Ok(x) => self.common = x,
            Err(e) => {
                tracing::error!("Failed to apply common settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        self.config_path = config_path;
        Ok(())
    }

    fn integr_settings_as_json(&self) -> Value {
        serde_json::to_value(&self.settings_docker).unwrap()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    fn integr_tools(&self, _integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        vec![Box::new(ToolDocker {
            common: self.common.clone(),
            settings_docker: self.settings_docker.clone(),
            config_path: self.config_path.clone(),
        })]
    }

    fn integr_schema(&self) -> &str
    {
        DOCKER_INTEGRATION_SCHEMA
    }
}

impl ToolDocker {
    pub async fn command_execute(&self, command: &str, gcx: Arc<ARwLock<GlobalContext>>, fail_if_stderr_is_not_empty: bool, verbose_error: bool) -> Result<(String, String), String>
    {
        let mut command_args = split_command(&command)?;

        if command_is_interactive_or_blocking(&command_args) {
            return Err("Docker commands that are interactive or blocking are not supported".to_string());
        }

        command_append_label_if_creates_resource(&mut command_args, &self.settings_docker.label);

        let docker_host = self.get_docker_host(gcx.clone()).await?;
        let mut command_process = Command::new(&self.settings_docker.docker_cli_path);
        if !docker_host.is_empty() {
            command_process.arg("-H").arg(&docker_host);
        }
        let output = command_process
            .args(&command_args)
            .stdin(std::process::Stdio::null())
            .output()
            .await
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if fail_if_stderr_is_not_empty && !stderr.is_empty() {
            let error_message = if verbose_error {
                format!("Command `{}` failed: {}", command, stderr)
            } else {
                stderr
            };
            return Err(error_message);
        }

        Ok((stdout, stderr))
    }

    pub async fn get_docker_host(&self, gcx: Arc<ARwLock<GlobalContext>>) -> Result<String, String>
    {
        match &self.settings_docker.get_ssh_config() {
            Some(ssh_config) => {
                let local_port = forward_remote_docker_if_needed(&self.settings_docker.docker_daemon_address, ssh_config, gcx.clone()).await?;
                Ok(format!("127.0.0.1:{}", local_port))
            },
            None => Ok(self.settings_docker.docker_daemon_address.clone()),
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

        let (stdout, _) = self.command_execute(&command, gcx.clone(), true, false).await?;

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

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(self.integr_common().confirmation)
    }

    fn has_config_path(&self) -> Option<String> {
        Some(self.config_path.clone())
    }
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
  docker_cli_path:
    f_type: string_long
    f_desc: "Path to the Docker CLI executable."
    f_default: "docker"
  label:
    f_type: string_short
    f_desc: "Label for the Docker container."
    f_default: "refact"
  docker_daemon_address:
    f_type: string_long
    f_desc: "The address to connect to the Docker daemon; specify only if not using the default."
    f_extra: true
  remote_docker:
    f_type: bool
    f_desc: "Use SSH to connect to remote Docker."
    f_extra: true
  ssh_host:
    f_type: string_long
    f_desc: "SSH host to connect to remote Docker."
    f_label: "SSH Host"
    f_extra: true
  ssh_user:
    f_type: string_short
    f_desc: "SSH user to connect to remote Docker."
    f_default: "root"
    f_label: "SSH User"
    f_extra: true
  ssh_port:
    f_type: string_short
    f_desc: "The SSH port to connect to remote Docker."
    f_default: "22"
    f_label: "SSH Port"
    f_extra: true
  ssh_identity_file:
    f_type: string_long
    f_desc: "Path to the SSH identity file to connect to remote Docker."
    f_label: "SSH Identity File"
    f_extra: true
available:
  on_your_laptop_possible: true
  when_isolated_possible: false
confirmation:
  ask_user_default: []
  deny_default: ["docker* rm *", "docker* rmi *", "docker* pause *", "docker* stop *", "docker* kill *"]
smartlinks:
  - sl_label: "Test"
    sl_chat:
      - role: "user"
        content: |
          🔧 The docker tool should be visible now. To test the tool, list the running containers, briefly describe the containers and express
          satisfaction and relief if it works, and change nothing. If it doesn't work or the tool isn't available, go through the usual plan in the system prompt.
    sl_enable_only_with_tool: true
"#;
