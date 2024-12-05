use serde::{Serialize, Deserialize};
use serde_json::Value;

use crate::integrations::docker::integr_docker::{serialize_num_to_str, deserialize_str_to_num};
use crate::integrations::docker::docker_container_manager::Port;
use crate::integrations::integr_abstract::IntegrationTrait;
use crate::tools::tools_description::Tool;

#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct SettingsIsolation {
    pub container_workspace_folder: String,
    pub docker_image_id: String,
    pub host_lsp_path: String,
    #[serde(serialize_with = "serialize_ports", deserialize_with = "deserialize_ports")]
    pub ports: Vec<Port>,
    #[serde(serialize_with = "serialize_num_to_str", deserialize_with = "deserialize_str_to_num")]
    pub keep_containers_alive_for_x_minutes: u64,
}

fn serialize_ports<S: serde::Serializer>(ports: &Vec<Port>, serializer: S) -> Result<S::Ok, S::Error> {
    let ports_str = ports.iter().map(|port| format!("{}:{}", port.published, port.target))
        .collect::<Vec<_>>().join(",");
    serializer.serialize_str(&ports_str)
}
fn deserialize_ports<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<Port>, D::Error> {
    let ports_str = String::deserialize(deserializer)?;
    ports_str.split(',').filter(|s| !s.is_empty()).map(|port_str| {
        let (published, target) = port_str.split_once(':')
            .ok_or_else(|| serde::de::Error::custom("expected format 'published:target'"))?;
        Ok(Port { published: published.to_string(), target: target.to_string() })
    }).collect()
}

#[derive(Clone, Default, Debug)]
pub struct IntegrationIsolation {
    pub settings_isolation: SettingsIsolation,
}

impl IntegrationTrait for IntegrationIsolation {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn integr_settings_apply(&mut self, value: &Value) -> Result<(), String> {
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
        Ok(())
    }

    fn integr_settings_as_json(&self) -> Value {
        serde_json::to_value(&self.settings_isolation).unwrap()
    }

    fn can_upgrade_to_tool(&self) -> bool { false }

    fn integr_upgrade_to_tool(&self, _integr_name: &str) -> Box<dyn Tool + Send> {
        unimplemented!("Isolation cannot be upgraded to a tool, its configuration is used to run the project in isolation.")
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
smartlinks: []
"#;