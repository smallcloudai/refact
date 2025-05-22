use serde::{Deserialize, Serialize};
use crate::integrations::utils::{serialize_num_to_str, deserialize_str_to_num};

#[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
pub struct CommonMCPSettings {
    #[serde(default = "default_init_timeout", serialize_with = "serialize_num_to_str", deserialize_with = "deserialize_str_to_num")]
    pub init_timeout: u64,
    #[serde(default = "default_request_timeout", serialize_with = "serialize_num_to_str", deserialize_with = "deserialize_str_to_num")]
    pub request_timeout: u64,
}

pub fn default_init_timeout() -> u64 { 60 }

pub fn default_request_timeout() -> u64 { 30 }
