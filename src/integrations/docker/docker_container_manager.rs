use std::path::PathBuf;
use std::{sync::Arc, sync::Weak, time::SystemTime};
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tokio::time::Duration;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};
use tracing::{error, info, warn};
use url::Url;
use walkdir::WalkDir;

use crate::files_correction::get_project_dirs;
use crate::files_in_workspace::retrieve_files_in_workspace_folders;
use crate::global_context::GlobalContext;
use crate::http::http_post;
use crate::http::routers::v1::lsp_like_handlers::LspLikeInit;
use crate::http::routers::v1::sync_files::SyncFilesExtractTarPost;
use crate::integrations::sessions::get_session_hashmap_key;
use crate::integrations::sessions::IntegrationSession;
use crate::integrations::docker::docker_ssh_tunnel_utils::{ssh_tunnel_open, SshTunnel};
use crate::integrations::docker::integr_docker::{docker_tool_load, ToolDocker};

use super::docker_ssh_tunnel_utils::ssh_tunnel_check_status;

pub const DEFAULT_CONTAINER_LSP_PATH: &str = "/usr/local/bin/refact-lsp";
pub const TARGET_LSP_PORT: &str = "8001";

#[derive(Clone, Debug)]
pub struct Port {
    pub published: String,
    pub target: String,
}

impl<'de> Deserialize<'de> for Port {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let (published, target) = s.split_once(':')
            .ok_or_else(|| serde::de::Error::custom("expected format '8080:3000'"))?;
        Ok(Port { published: published.to_string(), target: target.to_string() })
    }
}

impl Serialize for Port {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}:{}", self.published, self.target))
    }
}

pub struct DockerContainerSession {
    container_id: String,
    connection: DockerContainerConnectionEnum,
    last_usage_ts: u64,
    session_timeout_after_inactivity: Duration,
    weak_gcx: Weak<ARwLock<GlobalContext>>,
}

pub enum DockerContainerConnectionEnum {
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
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn is_expired(&self) -> bool {
        let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        self.last_usage_ts + self.session_timeout_after_inactivity.as_secs() < current_time
    }
}

pub async fn docker_container_check_status_or_start(
    gcx: Arc<ARwLock<GlobalContext>>,
    docker_tool_maybe: Option<Arc<ToolDocker>>,
    chat_id: &str,
) -> Result<(), String>
{
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
                            let ssh_config = docker.settings_docker.ssh_config.clone().ok_or_else(|| "No ssh config for docker container".to_string())?;
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
            let ssh_config_maybe = docker.settings_docker.ssh_config.clone();
            
            const LSP_PORT: &str = "8001";
            let mut ports_to_forward = if ssh_config_maybe.is_some() {
                docker.settings_docker.ports.iter()
                    .map(|p| Port {published: "0".to_string(), target: p.target.clone()}).collect::<Vec<_>>()
            } else {
                docker.settings_docker.ports.clone()
            };
            ports_to_forward.insert(0, Port {published: "0".to_string(), target: LSP_PORT.to_string()});

            let container_id = docker_container_create(&docker, &chat_id, &ports_to_forward, LSP_PORT, gcx.clone()).await?;
            docker_container_sync_yaml_configs(&docker, &container_id, gcx.clone()).await?;
            docker_container_start(gcx.clone(), &docker, &container_id).await?;
            let exposed_ports = docker_container_get_exposed_ports(&docker, &container_id, &ports_to_forward, gcx.clone()).await?;
            let host_lsp_port = exposed_ports.iter().find(|p| p.target == LSP_PORT)
                .ok_or_else(|| "No LSP port exposed".to_string())?.published.clone();

            let keep_containers_alive_for_x_minutes = docker.settings_docker.keep_containers_alive_for_x_minutes;

            let connection = match ssh_config_maybe {
                Some(ssh_config) => {
                    let mut ports_to_forward_through_ssh = exposed_ports.into_iter()
                        .map(|exposed_port| {
                            let matched_external_port = docker.settings_docker.ports.iter()
                                .find(|configured_port| configured_port.target == exposed_port.target)
                                .map_or_else(|| "0".to_string(), |forwarded_port| forwarded_port.published.clone());
                            Port {
                                published: matched_external_port,
                                target: exposed_port.published,
                            }
                        }).collect::<Vec<_>>();
                    let ssh_tunnel = ssh_tunnel_open(&mut ports_to_forward_through_ssh, &ssh_config).await?;
                    DockerContainerConnectionEnum::SshTunnel(ssh_tunnel)
                },
                None => DockerContainerConnectionEnum::LocalPort(host_lsp_port),
            };

            let lsp_port_to_connect = match &connection {
                DockerContainerConnectionEnum::SshTunnel(ssh_tunnel) => {
                    ssh_tunnel.get_first_published_port()?
                },
                DockerContainerConnectionEnum::LocalPort(internal_port) => {
                    internal_port.to_string()
                }
            };
            docker_container_sync_workspace(gcx.clone(), &docker, &container_id, &lsp_port_to_connect).await?;

            if !docker.settings_docker.command.is_empty() {
                let cmd_to_execute = format!("exec --detach {} {}", container_id, docker.settings_docker.command);
                match docker.command_execute(&cmd_to_execute, gcx.clone(), false).await {
                    Ok((cmd_stdout, cmd_stderr)) => { info!("Command executed: {cmd_stdout}\n{cmd_stderr}") },
                    Err(e) => { error!("Command execution failed: {}", e) },
                };
            }

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

pub async fn docker_container_get_host_lsp_port_to_connect(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_id: &str,
) -> Result<String, String> 
{
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
                    ssh_tunnel.get_first_published_port()
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

async fn docker_container_create(
    docker: &ToolDocker,
    chat_id: &str,
    ports_to_forward: &Vec<Port>,
    lsp_port: &str,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<String, String> {
    let docker_image_id = docker.settings_docker.docker_image_id.clone();
    if docker_image_id.is_empty() {
        return Err("No image ID to run container from, please specify one.".to_string());
    }
    let workspace_folder = docker.settings_docker.container_workspace_folder.clone();
    let host_lsp_path  = docker.settings_docker.host_lsp_path.clone();

    let (address_url, api_key) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.cmdline.address_url.clone(), gcx_locked.cmdline.api_key.clone())
    };

    let lsp_command = format!(
        "{DEFAULT_CONTAINER_LSP_PATH} --http-port {lsp_port} --logs-stderr --inside-container \
        --address-url {address_url} --api-key {api_key} --vecdb --reset-memory --ast --experimental",
    );
    
    let ports_to_forward_as_arg_list = ports_to_forward.iter()
        .map(|p| format!("--publish={}:{}", p.published, p.target)).collect::<Vec<_>>().join(" ");
    let run_command = format!(
        "container create --name=refact-{chat_id} --volume={host_lsp_path}:{DEFAULT_CONTAINER_LSP_PATH} \
        {ports_to_forward_as_arg_list} --entrypoint sh {docker_image_id} -c '{lsp_command}'",
    );

    info!("Executing docker command: {}", &run_command);
    let (run_output, _) = docker.command_execute(&run_command, gcx.clone(), true).await?;

    let container_id = run_output.trim();
    if container_id.len() < 12 {
        return Err("Docker run error: no container ID returned.".into());
    }

    Ok(container_id[..12].to_string())
}

async fn docker_container_sync_yaml_configs(
    docker: &ToolDocker,
    container_id: &str,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<(), String> {
    let (cache_dir, config_dir) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.cache_dir.clone(), gcx_locked.config_dir.clone())
    };
    let container_home_dir = docker_container_get_home_dir(&docker, &container_id, gcx.clone()).await?;

    // Creating intermediate folders one by one, as docker cp does not support --parents
    let temp_dir = tempfile::Builder::new().tempdir()
        .map_err(|e| format!("Error creating temporary directory: {}", e))?;
    let temp_dir_path = temp_dir.path().to_string_lossy().to_string();
    docker.command_execute(&format!("container cp {temp_dir_path} {container_id}:{container_home_dir}/.cache/"), gcx.clone(), true).await?;
    docker.command_execute(&format!("container cp {temp_dir_path} {container_id}:{container_home_dir}/.cache/refact"), gcx.clone(), true).await?;
    
    let config_files_to_sync = ["privacy.yaml", "integrations.yaml", "bring-your-own-key.yaml", "competency.yaml"];
    let (remote_integrations_path, competency_path) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.cmdline.remote_integrations.clone(), gcx_locked.cmdline.competency.clone())
    };
    for file in &config_files_to_sync {
        let local_path = match *file {
            "integrations.yaml" if !remote_integrations_path.is_empty() => remote_integrations_path.clone(),
            "competency.yaml" if !competency_path.is_empty() => competency_path.clone(),
            _ => cache_dir.join(file).to_string_lossy().to_string(),
        };
        let container_path = format!("{container_id}:{container_home_dir}/.cache/refact/{file}");
        docker.command_execute(&format!("container cp {local_path} {container_path}"), gcx.clone(), true).await?;
    }

    // Copying config folder
    let config_dir_string = config_dir.to_string_lossy().to_string();
    docker.command_execute(&format!("container cp {temp_dir_path} {container_id}:{container_home_dir}/.config/"), gcx.clone(), true).await?;
    docker.command_execute(&format!("container cp {config_dir_string} {container_id}:{container_home_dir}/.config/refact"), gcx.clone(), true).await?;

    Ok(())
}

async fn docker_container_get_home_dir(
    docker: &ToolDocker,
    container_id: &str,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<String, String> {
    let inspect_config_command = "container inspect --format '{{json .Config}}' ".to_string() + &container_id;
    let (inspect_config_output, _) = docker.command_execute(&inspect_config_command, gcx.clone(), true).await?;

    let config_json: serde_json::Value = serde_json::from_str(&inspect_config_output)
        .map_err(|e| format!("Error parsing docker config: {}", e))?;

    if let Some(home_env) = config_json.get("Env").and_then(|env| env.as_array())
        .and_then(|env| env.iter().find_map(|e| e.as_str()?.strip_prefix("HOME="))) {
        return Ok(home_env.to_string());
    }

    let user = config_json.get("User").and_then(serde_json::Value::as_str).unwrap_or("");
    Ok(if user.is_empty() || user == "root" { "root".to_string() } else { format!("/home/{user}") })
}

async fn docker_container_start(
    gcx: Arc<ARwLock<GlobalContext>>,
    docker: &ToolDocker,
    container_id: &str,
) -> Result<(), String> {
    let start_command = "container start ".to_string() + &container_id;
    docker.command_execute(&start_command, gcx.clone(), true).await?;

    // If docker container is not running, print last lines of logs.
    let inspect_command = "container inspect --format '{{json .State.Running}}' ".to_string() + &container_id;
    let (inspect_output, _) = docker.command_execute(&inspect_command, gcx.clone(), true).await?;
    if inspect_output.trim() != "true" {
        let (logs_output, _) = docker.command_execute(&format!("container logs --tail 10 {container_id}"), gcx.clone(), true).await?;
        return Err(format!("Docker container is not running: \n{logs_output}"));
    }

    Ok(())
}

async fn docker_container_sync_workspace(
    gcx: Arc<ARwLock<GlobalContext>>,
    docker: &ToolDocker,
    container_id: &str,
    lsp_port_to_connect: &str,
) -> Result<(), String> {
    let workspace_folder = get_project_dirs(gcx.clone())
        .await
        .into_iter()
        .next()
        .ok_or_else(|| "No workspace folders found".to_string())?;
    let container_workspace_folder = PathBuf::from(&docker.settings_docker.container_workspace_folder);

    let temp_tar_file = tempfile::Builder::new().suffix(".tar").tempfile()
        .map_err(|e| format!("Error creating temporary tar file: {}", e))?.into_temp_path();
    let tar_file_name = temp_tar_file.file_name().unwrap_or_default().to_string_lossy().to_string();
    let tar_async_file = File::create(&temp_tar_file).await
        .map_err(|e| format!("Error opening temporary tar file: {}", e))?;

    let mut tar_builder = async_tar::Builder::new(tar_async_file.compat_write());
    tar_builder.follow_symlinks(true);
    tar_builder.mode(async_tar::HeaderMode::Complete);

    let files = retrieve_files_in_workspace_folders(
        vec![workspace_folder.clone()], true, true).await;
    for file in &files {
        let relative_path = file.strip_prefix(&workspace_folder)
           .map_err(|e| format!("Error stripping prefix: {}", e))?;

        tar_builder.append_path_with_name(file, relative_path).await
           .map_err(|e| format!("Error adding file to tar archive: {}", e))?;
    }

    append_folder_if_exists(&mut tar_builder, &workspace_folder, ".git").await?;
    append_folder_if_exists(&mut tar_builder, &workspace_folder, ".hg").await?;
    append_folder_if_exists(&mut tar_builder, &workspace_folder, ".svn").await?;

    tar_builder.finish().await.map_err(|e| format!("Error finishing tar archive: {}", e))?;

    let cp_command = format!("container cp {} {}:{}", temp_tar_file.to_string_lossy(), container_id, container_workspace_folder.to_string_lossy());
    docker.command_execute(&cp_command, gcx.clone(), true).await?;

    let sync_files_post = SyncFilesExtractTarPost {
        tar_path: container_workspace_folder.join(&tar_file_name).to_string_lossy().to_string(),
        extract_to: container_workspace_folder.to_string_lossy().to_string(),
    };
    http_post(&format!("http://localhost:{lsp_port_to_connect}/v1/sync-files-extract-tar"), &sync_files_post).await?;

    tokio::fs::remove_file(&temp_tar_file).await
        .map_err(|e| format!("Error removing temporary archive: {}", e))?;

    info!("Workspace synced successfully.");

    let initialize_post = LspLikeInit {
        project_roots: vec![Url::parse(&format!("file://{}", container_workspace_folder.to_string_lossy())).unwrap()],
    };
    http_post(&format!("http://localhost:{lsp_port_to_connect}/v1/lsp-initialize"), &initialize_post).await?;
    info!("LSP initialized for workspace.");

    Ok(())
}

async fn append_folder_if_exists(
    tar_builder: &mut async_tar::Builder<Compat<File>>, 
    workspace_folder: &PathBuf, 
    folder_name: &str
) -> Result<(), String> {
    let folder_path = workspace_folder.join(folder_name);
    let mut num_files = 0;
    if folder_path.exists() {
        for entry in WalkDir::new(&folder_path) {
            let entry = entry.map_err(|e| format!("Error walking directory: {}", e))?;
            let relative_path = entry.path().strip_prefix(&workspace_folder)
              .map_err(|e| format!("Error stripping prefix: {}", e))?;
            tar_builder.append_path_with_name(entry.path(), relative_path).await
              .map_err(|e| format!("Error adding file to tar archive: {}", e))?;
            num_files += 1;
        }
        info!("Added folder {folder_name}, with {num_files} files.");
    } else {
        info!("Folder {folder_name} does not exist.");
    }
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

    let inspect_data: serde_json::Value = serde_json::from_str(&inspect_output)
        .map_err(|e| format!("Error parsing JSON output from docker inspect: {}", e))?;

    let mut exposed_ports = Vec::new();
    for port in ports_to_forward {
        let host_port = inspect_data[&format!("{}/tcp", port.target)][0]["HostPort"]
            .as_str()
            .ok_or_else(|| "Error getting host port from docker inspect output.".to_string())?;
        exposed_ports.push(Port { published: host_port.to_string(), target: port.target.to_string() });
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