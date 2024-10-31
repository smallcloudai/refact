use std::{ops::DerefMut, process::Stdio, sync::Arc};
use serde::{Deserialize, Serialize};
use tokio::{net::{TcpListener, TcpStream}, process::{Child, ChildStderr, Command}, sync::RwLock as ARwLock};
use tracing::{info, warn};

use crate::global_context::GlobalContext;
use crate::integrations::process_io_utils::read_until_token_or_timeout;
use crate::integrations::docker::docker_container_manager::Port;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SshConfig {
    pub host: String,
    #[serde(default = "default_user")]
    pub user: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub identity_file: Option<String>,
}
fn default_user() -> String { "root".to_string() }
fn default_port() -> u16 { 22 }

pub struct SshTunnel {
    pub forwarded_ports: Vec<Port>,
    pub process: Child,
    pub stderr: ChildStderr,
}

impl SshTunnel {
    pub fn get_first_external_port(&self) -> Result<String, String> {
        self.forwarded_ports.iter().next()
          .map(|port| port.external.clone())
          .ok_or_else(|| "Internal error: No forwarded ports found.".to_string())
    }
}

pub async fn forward_remote_docker_if_needed(connect_to_daemon_at: &str, ssh_config: &SshConfig, gcx: Arc<ARwLock<GlobalContext>>) -> Result<String, String> 
{
    let ssh_tunnel_arc = {
        let gcx_locked = gcx.read().await;
        gcx_locked.docker_ssh_tunnel.clone()
    };
    let mut ssh_tunnel_locked = ssh_tunnel_arc.lock().await;

    if let Some(ssh_tunnel) = ssh_tunnel_locked.deref_mut() {
        match ssh_tunnel_check_status(ssh_tunnel).await {
            Ok(()) => return ssh_tunnel.get_first_external_port(),
            Err(e) => {
                warn!("{}, restarting..", e);
                *ssh_tunnel_locked = None;
            }
        }
    }

    let remote_port_or_socket = if connect_to_daemon_at.starts_with("unix://") || connect_to_daemon_at.starts_with("npipe://") {
        connect_to_daemon_at.split("://").nth(1).unwrap_or_default().to_string()
    } else {
        connect_to_daemon_at.split(":").last().unwrap_or_default().to_string()
    };

    let ssh_tunnel = ssh_tunnel_open(&mut vec![Port { external: "0".to_string(), internal: remote_port_or_socket }], ssh_config).await?;
    let port = ssh_tunnel.get_first_external_port()?;
    *ssh_tunnel_locked = Some(ssh_tunnel);
    info!("Forwarding remote docker to local port {port}");
    Ok(port)
}

pub async fn ssh_tunnel_check_status(ssh_tunnel: &mut SshTunnel) -> Result<(), String> 
{
    let exit_status = ssh_tunnel.process.try_wait().map_err(|e| e.to_string())?;
    if let Some(status) = exit_status {
        return Err(format!("SSH tunnel process exited with status: {:?}", status));
    }

    let stderr_output = read_until_token_or_timeout(&mut ssh_tunnel.stderr, 50, "").await?;
    if !stderr_output.is_empty() {
        return Err(format!("SSH tunnel error: {}", stderr_output));
    }

    Ok(())
}

pub async fn ssh_tunnel_open(ports_to_forward: &mut Vec<Port>, ssh_config: &SshConfig) -> Result<SshTunnel, String> 
{
    let mut command = Command::new("ssh");
    command.arg("-N");
    if let Some(identity_file) = &ssh_config.identity_file {
        command.arg("-i").arg(identity_file);
    }
    command.arg("-p").arg(ssh_config.port.to_string());
    command.arg(&format!("{}@{}", ssh_config.user, ssh_config.host));
    command.stderr(Stdio::piped());

    for port in ports_to_forward.iter_mut() {
        if port.external == "0" {
            // Bind to port 0, so the OS will assign a free port.
            let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| format!("Failed to bind to address: {}", e))?;
            let local_addr = listener.local_addr().map_err(|e| format!("Failed to get local address: {}", e))?;
            port.external = local_addr.port().to_string();
        }
        let local_addr = format!("127.0.0.1:{}", port.external);
        let remote_addr = if port.internal.parse::<u16>().is_ok() {
            format!("127.0.0.1:{}", port.internal)
        } else {
            port.internal.clone()
        };
        command.arg("-L").arg(format!("{local_addr}:{remote_addr}"));
    }

    let mut process = command.spawn().map_err(|e| format!("Failed to start ssh process: {}", e))?;
    let mut stderr = process.stderr.take().ok_or("Failed to open stderr for ssh process")?;

    let output_stderr = read_until_token_or_timeout(&mut stderr, 100, "").await?;
    if !output_stderr.is_empty() {
        return Err(format!("SSH error: {}", output_stderr));
    }
 
    let port_to_test_connection = ports_to_forward.iter().next().ok_or_else(|| "Failed to get port to test connection".to_string())?;
    for attempt in 0..10 {
        match TcpStream::connect(format!("127.0.0.1:{}", &port_to_test_connection.external)).await {
            Ok(_) => {
                return Ok(SshTunnel {
                    forwarded_ports: ports_to_forward.clone(),
                    process,
                    stderr,
                });
            }
            Err(e) => {
                warn!("Failed to connect to 127.0.0.1:{} (attempt {}): {}", &port_to_test_connection.external, attempt + 1, e);
                let stderr_output = read_until_token_or_timeout(&mut stderr, 300, "").await?;
                if !stderr_output.is_empty() {
                    return Err(format!("Failed to open ssh tunnel: {}", stderr_output));
                }
            },
        }
    }

    return Err(format!("Failed to connect to 127.0.0.1:{}, max attempts reached", &port_to_test_connection.external));
}