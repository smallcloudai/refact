use std::{process::Stdio, sync::Arc};
use serde::{Deserialize, Serialize};
use tokio::{net::{TcpListener, TcpStream}, process::{Child, ChildStderr, Command}, sync::RwLock as ARwLock};
use tracing::{info, warn};

use crate::{global_context::GlobalContext, integrations::integr_pdb::read_until_token_or_timeout};

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
    pub stderr: ChildStderr,
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

pub async fn open_ssh_tunnel(remote_port_or_socket: &str, ssh_config: &SshConfig) -> Result<SshTunnel, String> 
{
    let mut command = Command::new("ssh");
    command.arg("-N");
    if let Some(identity_file) = &ssh_config.identity_file {
        command.arg("-i").arg(identity_file);
    }
    command.arg("-p").arg(ssh_config.port.to_string());

    command.arg(&format!("{}@{}", ssh_config.user, ssh_config.host));
    command.stderr(Stdio::piped());

    let local_port = {
        let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| format!("Failed to bind to address: {}", e))?;
        let local_addr = listener.local_addr().map_err(|e| format!("Failed to get local address: {}", e))?;
        local_addr.port()
    };
    command.arg("-L").arg(format!("127.0.0.1:{}:{}", local_port, remote_port_or_socket));
    let mut process = command.spawn().map_err(|e| format!("Failed to start ssh process: {}", e))?;
    
    let mut stderr = process.stderr.take().ok_or("Failed to open stderr for ssh process")?;
    let output_stderr = read_until_token_or_timeout(&mut stderr, 100, "").await?;
    if !output_stderr.is_empty() {
        return Err(format!("SSH error: {}", output_stderr));
    }

    for attempt in 0..10 {
        match TcpStream::connect(("127.0.0.1", local_port)).await {
            Ok(_) => {
                return Ok(SshTunnel {
                    remote_port_or_socket: remote_port_or_socket.to_string(),
                    local_port: local_port.to_string(),
                    process,
                    stderr,
                });
            }
            Err(e) => {
                warn!("Failed to connect to 127.0.0.1:{} (attempt {}): {}", local_port, attempt + 1, e);
                let stderr_output = read_until_token_or_timeout(&mut stderr, 300, "").await?;
                if !stderr_output.is_empty() {
                    return Err(format!("Failed to open ssh tunnel: {}", stderr_output));
                }
            },
        }
    }

    return Err(format!("Failed to connect to 127.0.0.1:{}, max attempts reached", local_port));
}