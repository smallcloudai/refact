pub trait IntegrationTrait: Send + Sync {
    fn integr_schema(&self) -> &str;
    fn integr_settings_apply(&mut self, value: &serde_json::Value) -> Result<(), String>;
    fn integr_settings_as_json(&self) -> serde_json::Value;
    fn integr_upgrade_to_tool(&self, integr_name: &String) -> Box<dyn crate::tools::tools_description::Tool + Send>;   // integr_name is sometimes different, "cmdline_compile_by_project" != "cmdline"
}
