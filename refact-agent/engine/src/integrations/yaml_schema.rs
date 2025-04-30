use serde::{Deserialize, Serialize};
use indexmap::IndexMap;
use crate::call_validation::ChatMessage;


#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DockerService {
    pub image: String,
    #[serde(default)]
    pub environment: IndexMap<String, String>,
    #[serde(default, skip_serializing_if="is_empty")]
    pub ports: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ISchemaField {
    pub f_type: String,
    #[serde(default, skip_serializing_if="is_default")]
    pub f_desc: String,
    #[serde(default, skip_serializing_if="is_default")]
    pub f_default: serde_json::Value,
    #[serde(default, skip_serializing_if="is_default")]
    pub f_placeholder: String,
    #[serde(default, skip_serializing_if="is_default")]
    pub f_label: String,
    #[serde(default, skip_serializing_if="is_empty")]
    pub smartlinks: Vec<ISmartLink>,
    #[serde(default, skip_serializing_if="is_default")]
    pub f_extra: bool,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ISmartLink {
    pub sl_label: String,
    #[serde(default, skip_serializing_if="is_empty")]
    pub sl_chat: Vec<ChatMessage>,
    #[serde(default, skip_serializing_if="is_default")]
    pub sl_goto: String,
    #[serde(default, skip_serializing_if="is_default")]
    pub sl_enable_only_with_tool: bool,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ISchemaAvailable {
    pub on_your_laptop_possible: bool,
    pub when_isolated_possible: bool,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ISchemaDocker {
    pub filter_label: String,
    pub filter_image: String,
    pub new_container_default: DockerService,
    #[serde(default, skip_serializing_if="is_empty")]
    pub smartlinks: Vec<ISmartLink>,
    #[serde(default, skip_serializing_if="is_empty")]
    pub smartlinks_for_each_container: Vec<ISmartLink>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ISchemaConfirmation {
    #[serde(default, skip_serializing_if="is_default")]
    pub not_applicable: bool,
    #[serde(default)]
    pub ask_user_default: Vec<String>,
    #[serde(default)]
    pub deny_default: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ISchema {
    pub fields: IndexMap<String, ISchemaField>,
    #[serde(default, skip_serializing_if="is_default")]
    pub description: String,
    pub available: ISchemaAvailable,
    pub confirmation: ISchemaConfirmation,
    #[serde(default, skip_serializing_if="is_empty")]
    pub smartlinks: Vec<ISmartLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker: Option<ISchemaDocker>,
}

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

fn is_empty<T>(t: &Vec<T>) -> bool {
    t.is_empty()
}
