use serde::Deserialize;
use serde::Serialize;
use async_trait::async_trait;


#[async_trait]
pub trait IntegrationTrait: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
    fn integr_schema(&self) -> &str;
    async fn integr_settings_apply(&mut self, value: &serde_json::Value, config_path: String) -> Result<(), String>;
    fn integr_settings_as_json(&self) -> serde_json::Value;
    fn integr_common(&self) -> IntegrationCommon;
    fn integr_tools(&self, integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>>;  // integr_name is sometimes different, "cmdline_compile_my_project" != "cmdline"
}

#[derive(Deserialize, Serialize, Clone, Default)]
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
    pub ask_user: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct IntegrationCommon {
    #[serde(default)]
    pub available: IntegrationAvailable,
    #[serde(default)]
    pub confirmation: IntegrationConfirmation,
}
