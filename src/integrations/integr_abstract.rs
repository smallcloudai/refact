pub trait IntegrationTrait: Send + Sync {
    fn integr_schema(&self) -> &str;
    fn integr_settings_apply(&mut self, value: &serde_json::Value) -> Result<(), String>;
    fn integr_settings_as_json(&self) -> serde_json::Value;
    fn integr_upgrade_to_tool(&self) -> Box<dyn crate::tools::tools_description::Tool + Send>;
}
