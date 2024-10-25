use std::{sync::Arc, sync::Weak, time::SystemTime};
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde_json::Value;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tokio::time::Duration;
use tracing::{error, info, warn};

use crate::global_context::GlobalContext;
use crate::integrations::sessions::get_session_hashmap_key;
use crate::tools::tools_description::read_integrations_value;
use crate::{at_commands::at_commands::AtCommandsContext, integrations::sessions::IntegrationSession};
use crate::integrations::docker::docker_ssh_tunnel_utils::{ssh_tunnel_open, SshTunnel};
use crate::integrations::docker::integr_docker::ToolDocker;

use super::docker_ssh_tunnel_utils::ssh_tunnel_check_status;

const DEFAULT_CONTAINER_LSP_PATH: &str = "/usr/local/bin/refact-lsp";

pub struct DockerContainerSession {
    container_id: String,
    connection: DockerContainerConnectionEnum,
    last_usage_ts: u64,
    session_timeout_after_inactivity: Duration,
    weak_gcx: Weak<ARwLock<GlobalContext>>,
}

enum DockerContainerConnectionEnum {
    SshTunnel(SshTunnel),
    LocalPort(u16),
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
        (ccx_locked.global_context.clone(), ccx_locked.chat_id.clone(), ccx_locked.at_tools.get("docker").cloned())
    };

    let docker_tool = match docker_tool_maybe {
        Some(docker_tool) => docker_tool,
        None => return Err(format!("docker tool not found, cannot check status or start docker container")),
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

            let ssh_config = {
                let docker_tool_locked = docker_tool.lock().await;
                let docker = docker_tool_locked.as_any().downcast_ref::<ToolDocker>().ok_or_else(|| "Failed to downcast docker tool")?;
                docker.integration_docker.ssh_config.clone()
            };

            match &mut docker_container_session.connection {
                DockerContainerConnectionEnum::SshTunnel(ssh_tunnel) => {
                    match ssh_tunnel_check_status(ssh_tunnel).await {
                        Ok(()) => {}
                        Err(e) => {
                            warn!("SSH tunnel error: {}, restarting tunnel..", e);
                            let ssh_config = ssh_config.ok_or_else(|| "No ssh config for docker container".to_string())?;
                            docker_container_session.connection = DockerContainerConnectionEnum::SshTunnel(
                                ssh_tunnel_open(&ssh_tunnel.remote_port_or_socket, &ssh_config).await?
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
            let docker_tool_locked = docker_tool.lock().await;
            let docker = docker_tool_locked.as_any().downcast_ref::<ToolDocker>().ok_or_else(|| "Failed to downcast docker tool")?;

            let internal_port: u16 = 8001;

            let container_id = docker_container_start(docker, &chat_id, &internal_port, gcx.clone()).await?;
            let host_port = docker_container_get_host_port(docker, &container_id, &internal_port, gcx.clone()).await?;

            let ssh_config_maybe = docker.integration_docker.ssh_config.clone();
            let keep_containers_alive_for_x_minutes = docker.integration_docker.keep_containers_alive_for_x_minutes;

            drop(docker_tool_locked);

            let connection = match ssh_config_maybe {
                Some(ssh_config) => {
                    let ssh_tunnel = ssh_tunnel_open(&format!("127.0.0.1:{}", host_port.to_string()), &ssh_config).await?;
                    DockerContainerConnectionEnum::SshTunnel(ssh_tunnel)
                },
                None => DockerContainerConnectionEnum::LocalPort(host_port),
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

pub async fn docker_container_get_host_port_to_connect(ccx: Arc<AMutex<AtCommandsContext>>) -> Result<u16, String> {
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
                    Ok(ssh_tunnel.local_port)
                },
                DockerContainerConnectionEnum::LocalPort(internal_port) => {
                    Ok(*internal_port)
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
    internal_port: &u16,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<String, String> {
    let docker_image_id = docker.integration_docker.docker_image_id.clone();
    if docker_image_id.is_empty() {
        return Err("No image ID to run container from, please specify one.".to_string());
    }
    let workspace_folder = docker.integration_docker.container_workspace_folder.clone();
    let host_lsp_path  = docker.integration_docker.host_lsp_path.clone();

    let api_key = gcx.read().await.cmdline.api_key.clone();

    // XXX hardcoded Refact, api_key is insufficient
    let lsp_command = format!(
        "mkdir -p $HOME/.cache/refact/ && {DEFAULT_CONTAINER_LSP_PATH} --http-port {internal_port} --logs-stderr \
        --address-url Refact --api-key {api_key} --vecdb --reset-memory --ast --experimental \
        --inside-container --workspace-folder {workspace_folder}",
    );
    let random_str = rand::thread_rng().sample_iter(&Alphanumeric).take(9).map(char::from).collect::<String>();

    // XXX look again, chat_id should be enough, why random_str?
    let run_command = format!(
        "run --detach --name=refact-{chat_id}-{random_str} --volume={host_lsp_path}:{DEFAULT_CONTAINER_LSP_PATH} \
        --publish=0:{internal_port} --entrypoint sh {docker_image_id} -c '{lsp_command}'",
    );

    info!("Executing docker command: {}", &run_command);
    let run_output = docker.command_execute(&run_command, gcx.clone()).await?;
    // XXX docker output might be:
    // /usr/local/bin/refact-lsp: error while loading shared libraries: libssl.so.1.1: cannot open shared object file: No such file or directory
    info!("run output: {}", &run_output);
    
    let container_id = run_output.trim();
    if container_id.len() < 12 {
        return Err("Docker run error: no container ID returned.".into());
    }

    if !docker.integration_docker.command.is_empty() {
        let cmd_to_execute = format!("exec --detach {} {}", container_id, docker.integration_docker.command);
        match docker.command_execute(&cmd_to_execute, gcx.clone()).await {
            Ok(cmd_result) => { info!("Command executed: {}", cmd_result) },
            Err(e) => { error!("Command execution failed: {}", e) },
        };
    }

    Ok(container_id[..12].to_string())
}

async fn docker_container_get_host_port(
    docker: &ToolDocker,
    container_id: &str,
    internal_port: &u16,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<u16, String> {
    let inspect_command = "inspect --format '{{json .NetworkSettings.Ports}}' ".to_string() + &container_id;
    let inspect_output = docker.command_execute(&inspect_command, gcx.clone()).await?;
    tracing::info!("{}:\n{}", inspect_command, inspect_output);

    let inspect_data: Value = serde_json::from_str(&inspect_output)
        .map_err(|e| format!("Error parsing JSON output from docker inspect: {}", e))?;

    inspect_data[&format!("{}/tcp", internal_port)][0]["HostPort"].as_str()
        .ok_or_else(|| "Error getting host port from docker inspect output.".to_string())?
        .parse::<u16>()
        .map_err(|e| format!("Error parsing host port as u16: {}", e))
}

async fn docker_container_kill(
    gcx: Arc<ARwLock<GlobalContext>>,
    container_id: &str,
) -> Result<(), String> {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let integrations_value = read_integrations_value(&cache_dir).await?;
    let docker = ToolDocker::new_if_configured(&integrations_value)
        .ok_or_else(|| "No docker integration configured".to_string())?;

    docker.command_execute(&format!("container stop {container_id}"), gcx.clone()).await?;
    info!("Stopped docker container {container_id}.");
    docker.command_execute(&format!("container remove {container_id}"), gcx.clone()).await?;
    info!("Removed docker container {container_id}.");
    Ok(())
}