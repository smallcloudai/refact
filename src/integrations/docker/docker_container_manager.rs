use std::path::PathBuf;
use std::{sync::Arc, sync::Weak, time::SystemTime};
use std::future::Future;
use tokio::fs::File;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tokio::time::Duration;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};
use tracing::{error, info, warn};
use url::Url;
use walkdir::WalkDir;

use crate::files_correction::get_project_dirs;
use crate::global_context::GlobalContext;
use crate::http::http_post;
use crate::http::routers::v1::lsp_like_handlers::LspLikeInit;
use crate::http::routers::v1::sync_files::SyncFilesExtractTarPost;
use crate::integrations::sessions::get_session_hashmap_key;
use crate::integrations::sessions::IntegrationSession;
use crate::integrations::docker::docker_ssh_tunnel_utils::{ssh_tunnel_open, SshTunnel, ssh_tunnel_check_status};
use crate::integrations::docker::integr_docker::ToolDocker;
use crate::integrations::docker::docker_and_isolation_load;
use crate::integrations::docker::integr_isolation::SettingsIsolation;

pub const DEFAULT_CONTAINER_LSP_PATH: &str = "/usr/local/bin/refact-lsp";


#[derive(Clone, Debug)]
pub struct Port {
    pub published: String,
    pub target: String,
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

impl IntegrationSession for DockerContainerSession {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn is_expired(&self) -> bool {
        let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        self.last_usage_ts + self.session_timeout_after_inactivity.as_secs() < current_time
    }

    fn try_stop(&mut self) -> Box<dyn Future<Output = String> + Send + '_> {
        Box::new(async {
            if let Some(gcx) = self.weak_gcx.upgrade() {
                let container_id = self.container_id.clone();
                match docker_container_kill(gcx, &container_id).await {
                    Ok(()) => format!("Cleanup docker container session: {}", container_id),
                    Err(e) => {
                        let message = format!("Failed to cleanup docker container session: {}", e);
                        error!(message);
                        message
                    }
                }
            } else {
                let message = "Detected program shutdown, quit.".to_string();
                info!(message);
                message
            }
        })
    }
}

pub async fn docker_container_check_status_or_start(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_id: &str,
) -> Result<(), String>
{
    let (docker, isolation_maybe) = docker_and_isolation_load(gcx.clone()).await?;
    let isolation = isolation_maybe.ok_or_else(|| "No isolation tool available".to_string())?;
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
                            let ssh_config = docker.settings_docker.get_ssh_config().ok_or_else(|| "No ssh config for docker container".to_string())?;
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
            let ssh_config_maybe = docker.settings_docker.get_ssh_config();

            const LSP_PORT: &str = "8001";
            let mut ports_to_forward = if ssh_config_maybe.is_some() {
                isolation.ports.iter()
                    .map(|p| Port {published: "0".to_string(), target: p.target.clone()}).collect::<Vec<_>>()
            } else {
                isolation.ports.clone()
            };
            ports_to_forward.insert(0, Port {published: "0".to_string(), target: LSP_PORT.to_string()});

            let container_id = docker_container_create(&docker, &isolation, &chat_id, &ports_to_forward, LSP_PORT, gcx.clone()).await?;
            docker_container_sync_config_folder(&docker, &container_id, gcx.clone()).await?;
            docker_container_start(gcx.clone(), &docker, &container_id).await?;
            let exposed_ports = docker_container_get_exposed_ports(&docker, &container_id, &ports_to_forward, gcx.clone()).await?;
            let host_lsp_port = exposed_ports.iter().find(|p| p.target == LSP_PORT)
                .ok_or_else(|| "No LSP port exposed".to_string())?.published.clone();

            let connection = match ssh_config_maybe {
                Some(ssh_config) => {
                    let mut ports_to_forward_through_ssh = exposed_ports.into_iter()
                        .map(|exposed_port| {
                            let matched_external_port = isolation.ports.iter()
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
            docker_container_sync_workspace(gcx.clone(), &docker, &isolation, &container_id, &lsp_port_to_connect).await?;

            let session: Arc<AMutex<Box<dyn IntegrationSession>>> = Arc::new(AMutex::new(Box::new(DockerContainerSession {
                container_id,
                connection,
                last_usage_ts: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
                session_timeout_after_inactivity: Duration::from_secs(60 * isolation.keep_containers_alive_for_x_minutes),
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

pub fn get_container_name(chat_id: &str) -> String {
    format!("refact-{chat_id}")
}

async fn docker_container_create(
    docker: &ToolDocker,
    isolation: &SettingsIsolation,
    chat_id: &str,
    ports_to_forward: &Vec<Port>,
    lsp_port: &str,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<String, String> {
    let docker_image_id = isolation.docker_image_id.clone();
    if docker_image_id.is_empty() {
        return Err("No image ID to run container from, please specify one.".to_string());
    }
    let host_lsp_path  = isolation.host_lsp_path.clone();

    let (address_url, api_key, integrations_yaml) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.cmdline.address_url.clone(), gcx_locked.cmdline.api_key.clone(), gcx_locked.cmdline.integrations_yaml.clone())
    };

    let mut lsp_command = format!(
        "{DEFAULT_CONTAINER_LSP_PATH} --http-port {lsp_port} --logs-stderr --inside-container \
        --address-url {address_url} --api-key {api_key} --vecdb --reset-memory --ast --experimental",
    );
    if !integrations_yaml.is_empty() { 
        lsp_command.push_str(" --integrations-yaml ~/.config/refact/integrations.yaml"); 
    }

    let ports_to_forward_as_arg_list = ports_to_forward.iter()
        .map(|p| format!("--publish={}:{}", p.published, p.target)).collect::<Vec<_>>().join(" ");
    let network_if_set = if !isolation.docker_network.is_empty() {
        format!("--network {}", isolation.docker_network)
    } else {
        String::new()
    };
    let container_name = get_container_name(chat_id);
    let run_command = format!(
        "container create --name={container_name} --volume={host_lsp_path}:{DEFAULT_CONTAINER_LSP_PATH} \
        {ports_to_forward_as_arg_list} {network_if_set} --entrypoint sh {docker_image_id} -c '{lsp_command}'",
    );

    info!("Executing docker command: {}", &run_command);
    let (run_output, _) = docker.command_execute(&run_command, gcx.clone(), true, true).await?;

    let container_id = run_output.trim();
    if container_id.len() < 12 {
        return Err("Docker run error: no container ID returned.".into());
    }

    Ok(container_id[..12].to_string())
}

async fn docker_container_sync_config_folder(
    docker: &ToolDocker,
    container_id: &str,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<(), String> {
    let (config_dir, integrations_yaml, variables_yaml) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.config_dir.clone(), gcx_locked.cmdline.integrations_yaml.clone(), gcx_locked.cmdline.variables_yaml.clone())
    };
    let config_dir_string = config_dir.to_string_lossy().to_string();
    let container_home_dir = docker_container_get_home_dir(&docker, &container_id, gcx.clone()).await?;

    // Creating intermediate folders one by one, as docker cp does not support --parents
    let temp_dir = tempfile::Builder::new().tempdir()
        .map_err(|e| format!("Error creating temporary directory: {}", e))?;
    let temp_dir_path = temp_dir.path().to_string_lossy().to_string();
    docker.command_execute(&format!("container cp \"{temp_dir_path}\" {container_id}:{container_home_dir}/.config/"), gcx.clone(), true, true).await?;
    docker.command_execute(&format!("container cp \"{config_dir_string}\" {container_id}:{container_home_dir}/.config/refact"), gcx.clone(), true, true).await?;

    if !integrations_yaml.is_empty() {
        let cp_integrations_command = format!("container cp \"{integrations_yaml}\" {container_id}:{container_home_dir}/.config/refact/integrations.yaml");
        docker.command_execute(&cp_integrations_command, gcx.clone(), true, true).await?;
    }
    if !variables_yaml.is_empty() {
        let cp_variables_command = format!("container cp \"{variables_yaml}\" {container_id}:{container_home_dir}/.config/refact/variables.yaml");
        docker.command_execute(&cp_variables_command, gcx.clone(), true, true).await?;
    }

    Ok(())
}

async fn docker_container_get_home_dir(
    docker: &ToolDocker,
    container_id: &str,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<String, String> {
    let inspect_config_command = "container inspect --format '{{json .Config}}' ".to_string() + &container_id;
    let (inspect_config_output, _) = docker.command_execute(&inspect_config_command, gcx.clone(), true, true).await?;

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
    docker.command_execute(&start_command, gcx.clone(), true, true).await?;

    // If docker container is not running, print last lines of logs.
    let inspect_command = "container inspect --format '{{json .State.Running}}' ".to_string() + &container_id;
    let (inspect_output, _) = docker.command_execute(&inspect_command, gcx.clone(), true, true).await?;
    if inspect_output.trim() != "true" {
        let (logs_output, _) = docker.command_execute(&format!("container logs --tail 10 {container_id}"), gcx.clone(), true, true).await?;
        return Err(format!("Docker container is not running: \n{logs_output}"));
    }

    Ok(())
}

async fn docker_container_sync_workspace(
    gcx: Arc<ARwLock<GlobalContext>>,
    docker: &ToolDocker,
    isolation: &SettingsIsolation,
    container_id: &str,
    lsp_port_to_connect: &str,
) -> Result<(), String> {
    // XXX should be many dirs
    let workspace_folder = get_project_dirs(gcx.clone())
        .await
        .into_iter()
        .next()
        .ok_or_else(|| "No workspace folders found".to_string())?;
    let container_workspace_folder = isolation.container_workspace_folder.clone();

    let temp_tar_file = tempfile::Builder::new().suffix(".tar").tempfile()
        .map_err(|e| format!("Error creating temporary tar file: {}", e))?.into_temp_path();
    let tar_file_name = temp_tar_file.file_name().unwrap_or_default().to_string_lossy().to_string();
    let tar_async_file = File::create(&temp_tar_file).await
        .map_err(|e| format!("Error opening temporary tar file: {}", e))?;

    let mut tar_builder = async_tar::Builder::new(tar_async_file.compat_write());
    tar_builder.follow_symlinks(true);
    tar_builder.mode(async_tar::HeaderMode::Complete);

    let (all_files, _vcs_folders) = crate::files_in_workspace::retrieve_files_in_workspace_folders(
        vec![workspace_folder.clone()],
        false,
        false,
    ).await;

    for file in &all_files {
        let relative_path = file.strip_prefix(&workspace_folder)
           .map_err(|e| format!("Error stripping prefix: {}", e))?;

        tar_builder.append_path_with_name(file, relative_path).await
           .map_err(|e| format!("Error adding file to tar archive: {}", e))?;
    }

    append_folder_if_exists(&mut tar_builder, &workspace_folder, ".git").await?;
    append_folder_if_exists(&mut tar_builder, &workspace_folder, ".hg").await?;
    append_folder_if_exists(&mut tar_builder, &workspace_folder, ".svn").await?;

    tar_builder.finish().await.map_err(|e| format!("Error finishing tar archive: {}", e))?;

    let cp_command = format!("container cp \"{}\" {}:{}", temp_tar_file.to_string_lossy(), container_id, container_workspace_folder);
    docker.command_execute(&cp_command, gcx.clone(), true, true).await?;

    let sync_files_post = SyncFilesExtractTarPost {
        tar_path: format!("{}/{}", container_workspace_folder.trim_end_matches('/'), tar_file_name),
        extract_to: container_workspace_folder.clone(),
    };
    http_post(&format!("http://localhost:{lsp_port_to_connect}/v1/sync-files-extract-tar"), &sync_files_post).await?;

    tokio::fs::remove_file(&temp_tar_file).await
        .map_err(|e| format!("Error removing temporary archive: {}", e))?;

    info!("Workspace synced successfully.");

    let initialize_post = LspLikeInit {
        project_roots: vec![Url::parse(&format!("file://{container_workspace_folder}")).unwrap()],
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
    let (inspect_output, _) = docker.command_execute(&inspect_command, gcx.clone(), true, true).await?;
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
    let (docker, _) = docker_and_isolation_load(gcx.clone()).await?;

    docker.command_execute(&format!("container stop {container_id}"), gcx.clone(), true, true).await?;
    info!("Stopped docker container {container_id}.");
    docker.command_execute(&format!("container remove {container_id}"), gcx.clone(), true, true).await?;
    info!("Removed docker container {container_id}.");
    Ok(())
}