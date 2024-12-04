use std::{ops::DerefMut, process::Stdio, sync::Arc};
use serde::{Deserialize, Serialize};
use tokio::{net::{TcpListener, TcpStream}, process::{Child, ChildStderr, ChildStdout, Command}, sync::RwLock as ARwLock};
use tracing::{info, warn};

use crate::global_context::GlobalContext;
use crate::integrations::process_io_utils::blocking_read_until_token_or_timeout;
use crate::integrations::docker::docker_container_manager::Port;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SshConfig {
    pub host: String,
    pub user: String,
    pub port: u16,
    pub identity_file: Option<String>,
}

pub struct SshTunnel {
    pub forwarded_ports: Vec<Port>,
    pub process: Child,
    pub stdout: ChildStdout,
    pub stderr: ChildStderr,
}

impl SshTunnel {
    pub fn get_first_published_port(&self) -> Result<String, String> {
        self.forwarded_ports.iter().next()
          .map(|port| port.published.clone())
          .ok_or_else(|| "Internal error: No forwarded ports found.".to_string())
    }
}

pub async fn forward_remote_docker_if_needed(docker_daemon_address: &str, ssh_config: &SshConfig, gcx: Arc<ARwLock<GlobalContext>>) -> Result<String, String>
{
    let ssh_tunnel_arc = {
        let gcx_locked = gcx.read().await;
        gcx_locked.docker_ssh_tunnel.clone()
    };
    let mut ssh_tunnel_locked = ssh_tunnel_arc.lock().await;

    if let Some(ssh_tunnel) = ssh_tunnel_locked.deref_mut() {
        match ssh_tunnel_check_status(ssh_tunnel).await {
            Ok(()) => return ssh_tunnel.get_first_published_port(),
            Err(e) => {
                warn!("{}, restarting...", e);
                *ssh_tunnel_locked = None;
            }
        }
    }

    let remote_port_or_socket = match docker_daemon_address {
        "" => "/var/run/docker.sock".to_string(),
        _ if docker_daemon_address.starts_with("unix://") || docker_daemon_address.starts_with("npipe://") => {
            docker_daemon_address.split("://").nth(1).unwrap_or_default().to_string()
        },
        _ => {
            docker_daemon_address.split(":").last().unwrap_or_default().to_string()
        }
    };

    let ssh_tunnel = ssh_tunnel_open(&mut vec![Port { published: "0".to_string(), target: remote_port_or_socket }], ssh_config).await?;
    let port = ssh_tunnel.get_first_published_port()?;
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

    let (_, stderr_output, _) = blocking_read_until_token_or_timeout(&mut ssh_tunnel.stdout, &mut ssh_tunnel.stderr, 100, "").await?;
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
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    for port in ports_to_forward.iter_mut() {
        if port.published == "0" {
            // Bind to port 0, so the OS will assign a free port.
            let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| format!("Failed to bind to address: {}", e))?;
            let local_addr = listener.local_addr().map_err(|e| format!("Failed to get local address: {}", e))?;
            port.published = local_addr.port().to_string();
        }
        let local_addr = format!("127.0.0.1:{}", port.published);
        let remote_addr = if port.target.parse::<u16>().is_ok() {
            format!("127.0.0.1:{}", port.target)
        } else {
            port.target.clone()
        };
        command.arg("-L").arg(format!("{local_addr}:{remote_addr}"));
    }

    let mut process = command.spawn().map_err(|e| format!("Failed to start ssh process: {}", e))?;
    let mut stdout = process.stdout.take().ok_or("Failed to open stdout for ssh process")?;
    let mut stderr = process.stderr.take().ok_or("Failed to open stderr for ssh process")?;

    let (_, output_stderr, _) = blocking_read_until_token_or_timeout(&mut stdout, &mut stderr, 100, "").await?;
    if !output_stderr.is_empty() {
        return Err(format!("SSH error: {}", output_stderr));
    }

    let port_to_test_connection = ports_to_forward.iter().next().ok_or_else(|| "Failed to get port to test connection".to_string())?;
    for attempt in 0..25 {
        match TcpStream::connect(format!("127.0.0.1:{}", &port_to_test_connection.published)).await {
            Ok(_) => {
                info!("huzzah, it worked: connect to 127.0.0.1:{}", port_to_test_connection.published);
                return Ok(SshTunnel {
                    forwarded_ports: ports_to_forward.clone(),
                    process,
                    stdout,
                    stderr,
                });
            }
            Err(e) => {
                info!("this should eventually work: connect to 127.0.0.1:{} attempt {}: {}", port_to_test_connection.published, attempt + 1, e);
                let (_, stderr_output, _) = blocking_read_until_token_or_timeout(&mut stdout, &mut stderr, 400, "").await?;
                if !stderr_output.is_empty() {
                    return Err(format!("Failed to open ssh tunnel: {}", stderr_output));
                }
            },
        }
    }

    return Err(format!("Failed to connect to 127.0.0.1:{}, max attempts reached", &port_to_test_connection.published));
}
