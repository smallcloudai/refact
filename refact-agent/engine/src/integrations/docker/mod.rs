use std::sync::Arc;

use tokio::sync::RwLock as ARwLock;
use crate::global_context::GlobalContext;
// use crate::integrations::integr_abstract::IntegrationTrait;
use crate::integrations::running_integrations::load_integrations;
use crate::integrations::docker::integr_docker::ToolDocker;
use crate::integrations::docker::integr_isolation::{SettingsIsolation, IntegrationIsolation};

pub mod integr_docker;
pub mod integr_isolation;
pub mod docker_ssh_tunnel_utils;
pub mod docker_container_manager;

pub async fn docker_and_isolation_load(gcx: Arc<ARwLock<GlobalContext>>) -> Result<(ToolDocker, Option<SettingsIsolation>), String>
{
    // XXX: why load all integrations if we need one or two?
    let (integrations, _yaml_errors) = load_integrations(gcx.clone(), true).await;

    let docker_tools = integrations.get("docker")
        .ok_or("Docker integration not found".to_string())?
        .integr_tools("docker").await;

    let docker_tool = docker_tools[0]
        .as_any().downcast_ref::<ToolDocker>()
        .ok_or("Failed to downcast docker tool".to_string())?
        .clone();

    let isolation_integration = integrations.get("isolation")
        .and_then(|integration| integration.as_any().downcast_ref::<IntegrationIsolation>())
        .map(|isolation| isolation.settings_isolation.clone());

    Ok((docker_tool, isolation_integration))
}
