use serde::Deserialize;
use serde::Serialize;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::global_context::GlobalContext;
use crate::integrations::setting_up_integrations::IntegrationRecord;


#[async_trait]
pub trait IntegrationTrait: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
    fn integr_schema(&self) -> &str;
    async fn integr_settings_apply(&mut self, gcx: Arc<ARwLock<GlobalContext>>, config_path: String, value: &serde_json::Value, common_settings: IntegrationCommon) -> Result<(), serde_json::Error>;
    fn integr_settings_as_json(&self) -> serde_json::Value;
    fn integr_common(&self) -> IntegrationCommon;
    async fn integr_tools(&self, integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>>;  // integr_name is sometimes different, "cmdline_compile_my_project" != "cmdline"
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct IntegrationAvailable {
    #[serde(default = "default_true")]
    pub on_your_laptop: bool,
    #[serde(default = "default_true")]
    pub when_isolated: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct IntegrationConfirmation {
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub allow: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct IntegrationCommon {
    #[serde(default)]
    pub available: IntegrationAvailable,
    #[serde(default)]
    pub confirmation: IntegrationConfirmation,
}

impl From<&IntegrationRecord> for IntegrationCommon {
    fn from(record: &IntegrationRecord) -> Self {
        IntegrationCommon {
            available: IntegrationAvailable {
                on_your_laptop: record.on_your_laptop,
                when_isolated: record.when_isolated,
            },
            confirmation: IntegrationConfirmation {
                deny: record.deny.clone(),
                allow: record.allow.clone(),
            },
        }
    }
}