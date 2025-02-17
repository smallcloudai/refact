use std::fmt::Display;

use serde::{Deserialize, Serializer, Deserializer};

use crate::integrations::docker::docker_container_manager::Port;

pub fn serialize_opt_num_to_str<T: Display, S: Serializer>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&value.as_ref().map_or_else(String::new, |v| v.to_string()))
}
pub fn deserialize_str_to_opt_num<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: std::str::FromStr, T::Err: Display, D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?.filter(|s| !s.is_empty())
        .map_or(Ok(None), |s| s.parse::<T>().map(Some).map_err(serde::de::Error::custom))
}

pub fn serialize_num_to_str<T: ToString, S: Serializer>(num: &T, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&num.to_string())
}
pub fn deserialize_str_to_num<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: std::str::FromStr, T::Err: Display, D: Deserializer<'de>,
{
    String::deserialize(deserializer)?.parse().map_err(serde::de::Error::custom)
}

pub fn serialize_ports<S: Serializer>(ports: &Vec<Port>, serializer: S) -> Result<S::Ok, S::Error> {
    let ports_str = ports.iter().map(|port| format!("{}:{}", port.published, port.target))
        .collect::<Vec<_>>().join(",");
    serializer.serialize_str(&ports_str)
}
pub fn deserialize_ports<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Port>, D::Error> {
    let ports_str = String::deserialize(deserializer)?;
    ports_str.split(',').filter(|s| !s.is_empty()).map(|port_str| {
        let (published, target) = port_str.split_once(':')
            .ok_or_else(|| serde::de::Error::custom("expected format 'published:target'"))?;
        Ok(Port { published: published.to_string(), target: target.to_string() })
    }).collect()
}