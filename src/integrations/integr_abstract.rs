use serde::Deserialize;
use serde::Serialize;


pub trait IntegrationTrait: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
    fn integr_schema(&self) -> &str;
    fn integr_settings_apply(&mut self, value: &serde_json::Value) -> Result<(), String>;
    fn integr_settings_as_json(&self) -> serde_json::Value;
    fn integr_common(&self) -> IntegrationCommon;
    fn can_upgrade_to_tool(&self) -> bool { true }
    fn integr_upgrade_to_tool(&self, integr_name: &str) -> Box<dyn crate::tools::tools_description::Tool + Send>;   // integr_name is sometimes different, "cmdline_compile_my_project" != "cmdline"
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

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct IntegrationConfirmation {
    #[serde(default)]
    pub ask_user: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct IntegrationCommon {
    #[serde(default)]
    pub available: IntegrationConfirmation,
    #[serde(default)]
    pub confirmation: IntegrationAvailable,
}
