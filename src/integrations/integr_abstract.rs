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
    pub on_your_laptop: bool,
    pub when_isolated: bool,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct IntegrationConfirmation {
    pub ask_user: Vec<String>,
    pub deny: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct IntegrationCommon {
    pub available: IntegrationConfirmation,
    pub confirmation: IntegrationAvailable,
}
