use std::sync::Arc;

use tokio::sync::RwLock as ARwLock;
use crate::global_context::GlobalContext;
use crate::integrations::integr_abstract::IntegrationTrait;
use crate::integrations::running_integrations::load_integrations;
use crate::integrations::docker::integr_docker::ToolDocker;
use crate::integrations::docker::integr_isolation::{SettingsIsolation, IntegrationIsolation};

pub mod integr_docker;
pub mod integr_isolation;
pub mod docker_ssh_tunnel_utils;
pub mod docker_container_manager;

pub async fn docker_and_isolation_load(gcx: Arc<ARwLock<GlobalContext>>) -> Result<(ToolDocker, Option<SettingsIsolation>), String> {
    let integrations = load_integrations(gcx.clone(), "".to_string(), true).await;

    let docker_tool = integrations.get("docker")
        .ok_or("Docker integration not found".to_string())?
        .integr_upgrade_to_tool("docker")
        .as_any().downcast_ref::<ToolDocker>()
        .ok_or("Failed to downcast docker tool".to_string())?
        .clone();

    let isolation_integration = integrations.get("isolation")
        .and_then(|integration| integration.as_any().downcast_ref::<IntegrationIsolation>())
        .map(|isolation| isolation.settings_isolation.clone());

    Ok((docker_tool, isolation_integration))
}