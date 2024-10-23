use std::{sync::Arc, time::SystemTime};
use serde_json::Value;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tokio::time::Duration;
use tracing::{info, warn};

use crate::global_context::GlobalContext;
use crate::integrations::sessions::get_session_hashmap_key;
use crate::{at_commands::at_commands::AtCommandsContext, integrations::sessions::IntegrationSession};
use crate::integrations::docker::docker_ssh_tunnel_utils::{ssh_tunnel_open, SshTunnel};
use crate::integrations::docker::integr_docker::ToolDocker;

use super::docker_ssh_tunnel_utils::ssh_tunnel_check_status;

const SESSION_TIMEOUT_AFTER_INACTIVITY: Duration = Duration::from_secs(60 * 60);

const DEFAULT_CONTAINER_LSP_PATH: &str = "/usr/local/bin/refact-lsp";

pub struct DockerContainerSession {
    pub container_id: String,
    connection: DockerContainerConnectionEnum,
    last_usage_ts: u64,
}

enum DockerContainerConnectionEnum {
    SshTunnel(SshTunnel),
    LocalPort(u16),
}

impl IntegrationSession for DockerContainerSession {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn is_expired(&self) -> bool {
        let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        self.last_usage_ts + SESSION_TIMEOUT_AFTER_INACTIVITY.as_secs() < current_time
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
            // let mut docker_container_session_locked = docker_container_session.lock().await;
            // let docker_container_session = docker_container_session_locked.as_any_mut().downcast_mut::<DockerContainerSession>()
            //     .ok_or_else(|| "Failed to downcast docker container session")?;

            // let ssh_config = {
            //     let docker_tool_locked = docker_tool.lock().await;
            //     let docker = docker_tool_locked.as_any().downcast_ref::<ToolDocker>().ok_or_else(|| "Failed to downcast docker tool")?;

            //     todo!()
            //     // docker_container_check_status(&docker, &docker_container_session.container_id, gcx.clone()).await?;

            //     // docker.integration_docker.ssh_config.clone()
            // };

            // match &mut docker_container_session.connection {
            //     DockerContainerConnectionEnum::SshTunnel(ssh_tunnel) => {
            //         match ssh_tunnel_check_status(ssh_tunnel).await {
            //             Ok(()) => {}
            //             Err(e) => {
            //                 warn!("SSH tunnel error: {}, restarting tunnel..", e);
            //                 let ssh_config = ssh_config.ok_or_else(|| "No ssh config for docker container".to_string())?;
            //                 ssh_tunnel = &mut ssh_tunnel_open(&ssh_tunnel.remote_port_or_socket, &ssh_config).await?;
            //             }
            //         }
            //     },
            //     DockerContainerConnectionEnum::LocalPort(_) => {}
            // }

            // docker_container_session.last_usage_ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
            // return Ok(());
            Ok(())
        }
        None => {
            let docker_tool_locked = docker_tool.lock().await;  
            let docker = docker_tool_locked.as_any().downcast_ref::<ToolDocker>().ok_or_else(|| "Failed to downcast docker tool")?;

            let internal_port: u16 = 8001;

            let container_id = docker_container_start(docker, &chat_id, &internal_port, gcx.clone()).await?;
            let host_port = docker_container_get_host_port(docker, &container_id, &internal_port, gcx.clone()).await?;

            let ssh_config_maybe = docker.integration_docker.ssh_config.clone();

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
    let docker_image_id = docker.integration_docker.docker_image_id.clone().ok_or_else(|| "No image ID to run container from, please specify one in integrations.yaml".to_string())?;
    let workspace_folder = docker.integration_docker.container_workspace_folder.clone().unwrap_or("/app".to_string());
    let host_lsp_path  = docker.integration_docker.host_lsp_path.clone();

    let api_key = gcx.read().await.cmdline.api_key.clone();

    let lsp_command = format!(
        "mkdir -p $HOME/.cache/refact/ && {DEFAULT_CONTAINER_LSP_PATH} --http-port {internal_port} --logs-stderr \
        --address-url Refact --api-key {api_key} --vecdb --reset-memory --ast --experimental \
        --inside-container --workspace-folder {workspace_folder}",
    );

    let run_command = format!(
        "run --detach --name=refact-{chat_id} --volume={host_lsp_path}:{DEFAULT_CONTAINER_LSP_PATH} \
        --publish=0:{internal_port} {docker_image_id} sh -c '{lsp_command}'",
    );

    info!("Executing docker command: {}", &run_command);
    let run_output = docker.command_execute(&run_command, gcx.clone()).await?;

    let container_id = run_output.trim();
    if container_id.len() < 12 {
        return Err("Docker run error: no container ID returned.".into());
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

    let inspect_data: Value = serde_json::from_str(&inspect_output)
        .map_err(|e| format!("Error parsing JSON output from docker inspect: {}", e))?;

    inspect_data[&format!("{}/tcp", internal_port)][0]["HostPort"].as_str()
        .ok_or_else(|| "Error getting host port from docker inspect output.".to_string())?
        .parse::<u16>()
        .map_err(|e| format!("Error parsing host port as u16: {}", e))
}
