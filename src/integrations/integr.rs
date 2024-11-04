use schemars::{schema_for, JsonSchema};
use serde::Serialize;
use serde::de::DeserializeOwned;
use crate::tools::tools_description::Tool;


pub trait Integration: Send + Sync {
    fn name(&self) -> String;
    fn update_from_json(&mut self, value: &serde_json::Value) -> Result<(), String>;
    fn from_yaml_validate_to_json(&self, value: &serde_yaml::Value) -> Result<serde_json::Value, String>;
    fn to_tool(&self) -> Box<dyn Tool + Send>;
    fn to_json(&self) -> Result<serde_json::Value, String>;
    fn to_schema_json(&self) -> serde_json::Value;
    fn default_value(&self) -> String;
    fn icon_link(&self) -> String;
}

pub fn json_schema<T: JsonSchema + Serialize + DeserializeOwned + Default>() -> Result<serde_json::Value, String> {
    let schema = schema_for!(T);
    let mut json_schema = serde_json::to_value(&schema).map_err(|e| e.to_string())?;

    // Reorder properties in the json_schema based on the order of dummy_instance
    let dummy_instance: T = T::default();
    let serialized_value = serde_json::to_value(&dummy_instance).unwrap();

    // schemars breaks order. Instead, reordering in Value
    if let serde_json::Value::Object(ref mut schema_map) = json_schema {
        if let Some(serde_json::Value::Object(ref mut properties)) = schema_map.get_mut("properties") {
            if let serde_json::Value::Object(dummy_map) = serialized_value {
                let mut ordered_properties = serde_json::Map::new();
                for key in dummy_map.keys() {
                    if let Some(value) = properties.remove(key) {
                        ordered_properties.insert(key.clone(), value);
                    }
                }
                *properties = ordered_properties;
            }
        }
    }

    Ok(json_schema)
}
