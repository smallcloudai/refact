use std::{sync::Arc, sync::Weak, time::SystemTime};
use async_process::Command;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tokio::time::Duration;
use tracing::{error, info, warn};

use crate::files_correction::get_project_dirs;
use crate::global_context::GlobalContext;
use crate::integrations::process_io_utils::last_n_lines;
use crate::integrations::sessions::get_session_hashmap_key;
use crate::{at_commands::at_commands::AtCommandsContext, integrations::sessions::IntegrationSession};
use crate::integrations::docker::docker_ssh_tunnel_utils::{ssh_tunnel_open, SshTunnel};
use crate::integrations::docker::integr_docker::{docker_tool_load, ToolDocker};

use super::docker_ssh_tunnel_utils::ssh_tunnel_check_status;

const DEFAULT_CONTAINER_LSP_PATH: &str = "/usr/local/bin/refact-lsp";

pub struct DockerContainerSession {
    container_id: String,
    connection: DockerContainerConnectionEnum,
    last_usage_ts: u64,
    session_timeout_after_inactivity: Duration,
    weak_gcx: Weak<ARwLock<GlobalContext>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Port {
    #[serde(rename = "local_port", deserialize_with = "string_or_number")]
    pub external: String,
    #[serde(rename = "container_port", deserialize_with = "string_or_number")]
    pub internal: String,
}

fn string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde::Deserialize::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        _ => return Err(serde::de::Error::custom("expected a string or an integer")),
    })
}

enum DockerContainerConnectionEnum {
    SshTunnel(SshTunnel),
    LocalPort(String),
}

impl Drop for DockerContainerSession {
    fn drop(&mut self) {
        if let Some(gcx) = self.weak_gcx.upgrade() {
            let container_id = self.container_id.clone();
            tokio::spawn(async move {
                if let Err(e) = docker_container_kill(gcx, &container_id).await {
                    error!("Failed to cleanup docker container session: {}", e);
                }
            });
        } else {
            info!("Detected program shutdown, quit.");
        }
    }
}

impl IntegrationSession for DockerContainerSession {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn is_expired(&self) -> bool {
        let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        self.last_usage_ts + self.session_timeout_after_inactivity.as_secs() < current_time
    }
}

pub async fn docker_container_check_status_or_start(ccx: Arc<AMutex<AtCommandsContext>>) -> Result<(), String>
{
    let (gcx, chat_id, docker_tool_maybe) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.global_context.clone(), ccx_locked.chat_id.clone(), ccx_locked.docker_tool.clone())
    };
    let docker = docker_tool_maybe.ok_or_else(|| "Docker tool not found".to_string())?;
    let docker_container_session_maybe = {
        let gcx_locked = gcx.read().await;
        gcx_locked.integration_sessions.get(&get_session_hashmap_key("docker", &chat_id)).cloned()
    };

    match docker_container_session_maybe {
        Some(docker_container_session) => {
            let mut docker_container_session_locked = docker_container_session.lock().await;
            let docker_container_session = docker_container_session_locked.as_any_mut().downcast_mut::<DockerContainerSession>()
                .ok_or_else(|| "Failed to downcast docker container session")?;

            match &mut docker_container_session.connection {
                DockerContainerConnectionEnum::SshTunnel(ssh_tunnel) => {
                    match ssh_tunnel_check_status(ssh_tunnel).await {
                        Ok(()) => {}
                        Err(e) => {
                            warn!("SSH tunnel error: {}, restarting tunnel..", e);
                            let ssh_config = docker.integration_docker.ssh_config.clone().ok_or_else(|| "No ssh config for docker container".to_string())?;
                            docker_container_session.connection = DockerContainerConnectionEnum::SshTunnel(
                                ssh_tunnel_open(&mut ssh_tunnel.forwarded_ports, &ssh_config).await?
                            );
                        }
                    }
                },
                DockerContainerConnectionEnum::LocalPort(_) => {}
            }

            docker_container_session.last_usage_ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
            Ok(())
        }
        None => {
            let ssh_config_maybe = docker.integration_docker.ssh_config.clone();
            
            const LSP_PORT: &str = "8001";
            let mut ports_to_forward = if ssh_config_maybe.is_some() {
                docker.integration_docker.ports.iter()
                    .map(|p| Port {external: "0".to_string(), internal: p.internal.clone()}).collect::<Vec<_>>()
            } else {
                docker.integration_docker.ports.clone()
            };
            ports_to_forward.insert(0, Port {external: "0".to_string(), internal: LSP_PORT.to_string()});

            let container_id = docker_container_start(&docker, &chat_id, &ports_to_forward, LSP_PORT, gcx.clone()).await?;
            docker_container_sync_yaml_configs(&docker, &container_id, gcx.clone()).await?;
            docker_container_sync_workspace(&docker, &container_id, gcx.clone()).await?;
            let exposed_ports = docker_container_get_exposed_ports(&docker, &container_id, &ports_to_forward, gcx.clone()).await?;
            let host_lsp_port = exposed_ports.iter().find(|p| p.internal == LSP_PORT)
                .ok_or_else(|| "No LSP port exposed".to_string())?.external.clone();

            let keep_containers_alive_for_x_minutes = docker.integration_docker.keep_containers_alive_for_x_minutes;

            let connection = match ssh_config_maybe {
                Some(ssh_config) => {
                    let mut ports_to_forward_through_ssh = exposed_ports.into_iter()
                        .map(|exposed_port| {
                            let matched_external_port = docker.integration_docker.ports.iter()
                                .find(|configured_port| configured_port.internal == exposed_port.internal)
                                .map_or_else(|| "0".to_string(), |forwarded_port| forwarded_port.external.clone());
                            Port {
                                external: matched_external_port,
                                internal: exposed_port.external,
                            }
                        }).collect::<Vec<_>>();
                    let ssh_tunnel = ssh_tunnel_open(&mut ports_to_forward_through_ssh, &ssh_config).await?;
                    DockerContainerConnectionEnum::SshTunnel(ssh_tunnel)
                },
                None => DockerContainerConnectionEnum::LocalPort(host_lsp_port),
            };

            let session: Arc<AMutex<Box<dyn IntegrationSession>>> = Arc::new(AMutex::new(Box::new(DockerContainerSession {
                container_id,
                connection,
                last_usage_ts: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
                session_timeout_after_inactivity: Duration::from_secs(60 * keep_containers_alive_for_x_minutes),
                weak_gcx: Arc::downgrade(&gcx),
            })));

            let mut gcx_locked = gcx.write().await;
            gcx_locked.integration_sessions.insert(
                get_session_hashmap_key("docker", &chat_id), session
            );
            Ok(())
        }
    }
}

pub async fn docker_container_get_host_lsp_port_to_connect(ccx: Arc<AMutex<AtCommandsContext>>) -> Result<String, String> {
    let (gcx, chat_id) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.global_context.clone(), ccx_locked.chat_id.clone())
    };

    let docker_container_session_maybe = {
        let gcx_locked = gcx.read().await;
        gcx_locked.integration_sessions.get(&get_session_hashmap_key("docker", &chat_id)).cloned()
    };

    match docker_container_session_maybe {
        Some(docker_container_session) => {
            let mut docker_container_session_locked = docker_container_session.lock().await;
            let docker_container_session = docker_container_session_locked.as_any_mut().downcast_mut::<DockerContainerSession>()
              .ok_or_else(|| "Failed to downcast docker container session")?;

            return match &docker_container_session.connection {
                DockerContainerConnectionEnum::SshTunnel(ssh_tunnel) => {
                    ssh_tunnel.get_first_external_port()
                },
                DockerContainerConnectionEnum::LocalPort(internal_port) => {
                    Ok(internal_port.to_string())
                },
            };
        },
        None => {
            return Err("Docker container session not found, cannot get host port".to_string());
        }
    }
}

async fn docker_container_start(
    docker: &ToolDocker,
    chat_id: &str,
    ports_to_forward: &Vec<Port>,
    lsp_port: &str,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<String, String> {
    let docker_image_id = docker.integration_docker.docker_image_id.clone();
    if docker_image_id.is_empty() {
        return Err("No image ID to run container from, please specify one.".to_string());
    }
    let workspace_folder = docker.integration_docker.container_workspace_folder.clone();
    let host_lsp_path  = docker.integration_docker.host_lsp_path.clone();

    let (address_url, api_key) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.cmdline.address_url.clone(), gcx_locked.cmdline.api_key.clone())
    };

    let lsp_command = format!(
        "mkdir -p $HOME/.cache/refact/ && {DEFAULT_CONTAINER_LSP_PATH} --http-port {lsp_port} --logs-stderr \
        --address-url {address_url} --api-key {api_key} --vecdb --reset-memory --ast --experimental \
        --inside-container --workspace-folder {workspace_folder}",
    );
    
    let ports_to_forward_as_arg_list = ports_to_forward.iter()
        .map(|p| format!("--publish={}:{}", p.external, p.internal)).collect::<Vec<_>>().join(" ");
    let run_command = format!(
        "run --detach --name=refact-{chat_id} --volume={host_lsp_path}:{DEFAULT_CONTAINER_LSP_PATH} \
        {ports_to_forward_as_arg_list} --entrypoint sh {docker_image_id} -c '{lsp_command}'",
    );

    info!("Executing docker command: {}", &run_command);
    let (run_output, _) = docker.command_execute(&run_command, gcx.clone(), true).await?;

    let container_id = run_output.trim();
    if container_id.len() < 12 {
        return Err("Docker run error: no container ID returned.".into());
    }

    // If docker container is not running, print last lines of logs.
    let inspect_command = "inspect --format '{{json .State.Running}}' ".to_string() + &container_id;
    let (inspect_output, _) = docker.command_execute(&inspect_command, gcx.clone(), true).await?;
    if inspect_output.trim() != "true" {
        let (logs_output, _) = docker.command_execute(&format!("container logs --tail 10 {container_id}"), gcx.clone(), true).await?;
        return Err(format!("Docker container is not running: \n{logs_output}"));
    }

    if !docker.integration_docker.command.is_empty() {
        let cmd_to_execute = format!("exec --detach {} {}", container_id, docker.integration_docker.command);
        match docker.command_execute(&cmd_to_execute, gcx.clone(), false).await {
            Ok((cmd_stdout, cmd_stderr)) => { info!("Command executed: {cmd_stdout}\n{cmd_stderr}") },
            Err(e) => { error!("Command execution failed: {}", e) },
        };
    }

    Ok(container_id[..12].to_string())
}

async fn docker_container_sync_yaml_configs(
    docker: &ToolDocker,
    container_id: &str,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<(), String> {
    let cache_dir = gcx.read().await.cache_dir.clone();

    let home_cmd = format!("container exec {container_id} sh -c 'echo $HOME'");
    let (stdout, _) = docker.command_execute(&home_cmd, gcx.clone(), true).await?;
    let container_home_dir = stdout.trim();

    let config_files_to_sync = ["privacy.yaml", "integrations.yaml"];
    for file in &config_files_to_sync {
        let local_path = cache_dir.join(file).to_string_lossy().to_string();
        let container_path = format!("{container_id}:{container_home_dir}/.cache/refact/{file}");
        docker.command_execute(&format!("container cp {local_path} {container_path}"), gcx.clone(), true).await?;
    }

    Ok(())
}

async fn docker_container_sync_workspace(
    docker: &ToolDocker,
    container_id: &str,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<(), String> {
    let _ = Command::new("rsync").output().await.map_err(|e| format!("Error copying workspace folder, rsync not found locally: {}", e))?;
    
    // TODO: There could be more than one workspace folder, to decide which one we'll sync we should 
    // get the active one, probably from the IDE.
    let mut workspace_folder = get_project_dirs(gcx.clone()).await.into_iter().next().ok_or_else(|| "No workspace folders found".to_string())?;
    workspace_folder.push("");
    let container_workspace_folder = &docker.integration_docker.container_workspace_folder;

    const COMMAND_TO_CHECK_IF_RSYNC_IS_INSTALLED: &str = r#"rsync --version > /dev/null 2>&1 && echo "rsync is installed""#;
    let (is_rsync_installed_output, _) = docker.command_execute(&format!("container exec {container_id} {COMMAND_TO_CHECK_IF_RSYNC_IS_INSTALLED}"), gcx.clone(), true).await?;
    info!("Is rsync installed output: {is_rsync_installed_output}");

    if is_rsync_installed_output.trim() != "rsync is installed" {
        const INSTALL_RSYNC_SHELL_COMMAND: &str = r#"sh -c 'sudo apt install -y rsync || \
            sudo yum install -y rsync || \
            sudo dnf install -y rsync || \
            sudo zypper install -y rsync || \
            sudo pacman -S --noconfirm rsync || \
            sudo apk add --no-cache rsync || \
            sudo microdnf install -y rsync || \
            sudo apt-get install -y rsync || \
            apt install -y rsync || \
            yum install -y rsync || \
            dnf install -y rsync || \
            zypper install -y rsync || \
            pacman -S --noconfirm rsync || \
            apk add --no-cache rsync || \
            microdnf install -y rsync || \
            apt-get install -y rsync && echo "_success_"'"#;
        
        let rsync_install_command = format!("container exec {container_id} {INSTALL_RSYNC_SHELL_COMMAND}");
            
        match docker.command_execute(&rsync_install_command, gcx.clone(), false).await {
            Ok((stdout, _)) if stdout.trim().ends_with("_success_") => {
                info!("Rsync has been installed");
            }
            Ok((_, stderr)) => {
                return Err(format!("Error installing rsync, please install it in the image: {stderr}"));
            }
            Err(e) => {
                return Err(format!("Error installing rsync, please install it in the image: {e}"));
            }
        }
    }

    let docker_cli_command = &docker.integration_docker.docker_cli_path;
    let docker_host = docker.get_docker_host(gcx.clone()).await?;
    let docker_exec_command = format!("{docker_cli_command} -H={docker_host} exec -i");

    let copy_from = workspace_folder.to_string_lossy().to_string();
    let copy_to = format!("{container_id}:{container_workspace_folder}");
    let args = ["--rsh", &docker_exec_command, "--checksum", "--recursive", "--links", "--perms", 
        "--executability", "--itemize-changes", "--filter", ":- .gitignore", &copy_from, &copy_to];
    // TODO: Copying .git folder can take a lot of time, we should try to not to copy it fully later.

    let rsync_output = Command::new("rsync").args(args).output().await
        .map_err(|e| format!("Error executing rsync: {e}"))?;

    let stderr = String::from_utf8_lossy(&rsync_output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&rsync_output.stdout).to_string();

    if !stderr.is_empty() {
        return Err(format!("Error syncing workspace: {stderr}"));
    }

    info!("Synced workspace: {}", last_n_lines(&stdout, 10));
    Ok(())
}

async fn docker_container_get_exposed_ports(
    docker: &ToolDocker,
    container_id: &str,
    ports_to_forward: &Vec<Port>,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<Vec<Port>, String> {
    let inspect_command = "inspect --format '{{json .NetworkSettings.Ports}}' ".to_string() + &container_id;
    let (inspect_output, _) = docker.command_execute(&inspect_command, gcx.clone(), true).await?;
    tracing::info!("{}:\n{}", inspect_command, inspect_output);

    let inspect_data: Value = serde_json::from_str(&inspect_output)
        .map_err(|e| format!("Error parsing JSON output from docker inspect: {}", e))?;

    let mut exposed_ports = Vec::new();
    for port in ports_to_forward {
        let host_port = inspect_data[&format!("{}/tcp", port.internal)][0]["HostPort"]
            .as_str()
            .ok_or_else(|| "Error getting host port from docker inspect output.".to_string())?;
        exposed_ports.push(Port { external: host_port.to_string(), internal: port.internal.to_string() });
    }
    Ok(exposed_ports)
}

async fn docker_container_kill(
    gcx: Arc<ARwLock<GlobalContext>>,
    container_id: &str,
) -> Result<(), String> {
    let docker = docker_tool_load(gcx.clone()).await?;

    docker.command_execute(&format!("container stop {container_id}"), gcx.clone(), true).await?;
    info!("Stopped docker container {container_id}.");
    docker.command_execute(&format!("container remove {container_id}"), gcx.clone(), true).await?;
    info!("Removed docker container {container_id}.");
    Ok(())
}