use std::{sync::Arc, time::Duration};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::{net::{TcpListener, TcpStream}, process::{Child, Command}, sync::RwLock as ARwLock, time::sleep};
use tracing::{error, info, warn};

use crate::global_context::GlobalContext;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub identity_file: Option<String>,
}

pub struct SshTunnel {
    pub remote_port_or_socket: String,
    pub local_port: String,
    pub process: Child,
}

pub async fn forward_remote_docker_if_needed(connect_to_daemon_at: &str, ssh_config: &SshConfig, gcx: Arc<ARwLock<GlobalContext>>) -> Result<(), String> 
{
    let ssh_tunnel_arc = {
        let gcx_locked = gcx.read().await;
        gcx_locked.docker_ssh_tunnel.clone()
    };
    let mut ssh_tunnel_locked = ssh_tunnel_arc.lock().await;

    if let Some(ssh_tunnel) = &mut *ssh_tunnel_locked {
        let exit_status = ssh_tunnel.process.try_wait().map_err(|e| e.to_string())?;
        if exit_status.is_none() {
            return Ok(());
        } else {
            warn!("SSH tunnel process exited unexpectedly, restarting...");
            *ssh_tunnel_locked = None;
        }
    }

    let remote_port_or_socket = if connect_to_daemon_at.starts_with("unix://") || connect_to_daemon_at.starts_with("npipe://") {
        connect_to_daemon_at.split("://").nth(1).unwrap_or_default().to_string()
    } else {
        connect_to_daemon_at.split(":").last().unwrap_or_default().to_string()
    };

    let ssh_tunnel = open_ssh_tunnel(&remote_port_or_socket, ssh_config).await?;
    info!("Forwarding remote docker to local port {}", &ssh_tunnel.local_port);
    *ssh_tunnel_locked = Some(ssh_tunnel);
    Ok(())
}

pub async fn open_ssh_tunnel(remote_port_or_socket: &str, ssh_config: &SshConfig) -> Result<SshTunnel, String> {
    let mut command = Command::new("ssh");
    command.arg("-N");
    if let Some(identity_file) = &ssh_config.identity_file {
        command.arg("-i").arg(identity_file);
    }
    command.arg("-p").arg(ssh_config.port.to_string());
    
    let mut local_port = None;
    for _ in 0..5 {
        let port = rand::thread_rng().gen_range(2u16.pow(14)..2u16.pow(15));
        match TcpListener::bind(("127.0.0.1", port)).await {
            Ok(_) => {
                local_port = Some(port);
                break;
            }
            Err(e) => error!("Failed to bind to port {}: {}", port, e),
        }
    }
    let local_port = local_port.ok_or("Failed to find a free local port")?;

    command.arg("-L").arg(format!("{}:{}", local_port, remote_port_or_socket));
    command.arg(&format!("{}@{}", ssh_config.user, ssh_config.host));
    
    let process = command.spawn().map_err(|e| format!("Failed to start ssh process: {}", e))?;

    for attempt in 0..10 {
        match TcpStream::connect(("127.0.0.1", local_port)).await {
            Ok(_) => {
                return Ok(SshTunnel {
                    remote_port_or_socket: remote_port_or_socket.to_string(),
                    local_port: local_port.to_string(),
                    process,
                });
            }
            Err(e) => {
                warn!("Failed to connect to 127.0.0.1:{} (attempt {}): {}", local_port, attempt + 1, e);
                sleep(Duration::from_millis(300)).await;
            },
        }
    }

    Err(format!("Failed to connect to 127.0.0.1:{}, max attempts reached", local_port))
}