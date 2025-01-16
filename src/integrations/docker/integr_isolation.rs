use serde::{Serialize, Deserialize};
use serde_json::Value;
use async_trait::async_trait;

use crate::integrations::utils::{serialize_num_to_str, deserialize_str_to_num, serialize_ports, deserialize_ports};
use crate::integrations::docker::docker_container_manager::Port;
use crate::integrations::integr_abstract::{IntegrationTrait, IntegrationCommon};


#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct SettingsIsolation {
    pub container_workspace_folder: String,
    pub docker_image_id: String,
    pub docker_network: String,
    pub host_lsp_path: String,
    #[serde(serialize_with = "serialize_ports", deserialize_with = "deserialize_ports")]
    pub ports: Vec<Port>,
    #[serde(serialize_with = "serialize_num_to_str", deserialize_with = "deserialize_str_to_num")]
    pub keep_containers_alive_for_x_minutes: u64,
}

#[derive(Clone, Default)]
pub struct IntegrationIsolation {
    pub common:  IntegrationCommon,
    pub settings_isolation: SettingsIsolation,
}

#[async_trait]
impl IntegrationTrait for IntegrationIsolation {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn integr_settings_apply(&mut self, value: &Value, _config_path: String) -> Result<(), String> {
        match serde_json::from_value::<SettingsIsolation>(value.clone()) {
            Ok(settings_isolation) => {
                tracing::info!("Isolation settings applied: {:?}", settings_isolation);
                self.settings_isolation = settings_isolation
            },
            Err(e) => {
                tracing::error!("Failed to apply settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        match serde_json::from_value::<IntegrationCommon>(value.clone()) {
            Ok(x) => self.common = x,
            Err(e) => {
                tracing::error!("Failed to apply common settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        Ok(())
    }

    fn integr_settings_as_json(&self) -> Value {
        serde_json::to_value(&self.settings_isolation).unwrap()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    fn integr_tools(&self, _integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
      vec![]
    }

    fn integr_schema(&self) -> &str
    {
        ISOLATION_INTEGRATION_SCHEMA
    }
}

pub const ISOLATION_INTEGRATION_SCHEMA: &str = r#"
fields:
  container_workspace_folder:
    f_type: string_long
    f_desc: "The workspace folder inside the container."
    f_default: "/app"
  docker_image_id:
    f_type: string_long
    f_desc: "The Docker image ID to use."
  host_lsp_path:
    f_type: string_long
    f_desc: "Path to the LSP on the host."
    f_default: "/opt/refact/bin/refact-lsp"
  command:
    f_type: string_long
    f_desc: "Command to run inside the Docker container."
  keep_containers_alive_for_x_minutes:
    f_type: string_short
    f_desc: "How long to keep containers alive in minutes."
    f_default: "60"
  ports:
    f_type: string_long
    f_desc: "Comma separated published:target notation for ports to publish, example '8080:3000,5000:5432'"
available:
  on_your_laptop_possible: true
  when_isolated_possible: false
confirmation:
  ask_user_default: []
  deny_default: ["docker* rm *", "docker* rmi *", "docker* pause *", "docker* stop *", "docker* kill *"]
"#;
