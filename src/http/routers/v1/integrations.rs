use std::path::PathBuf;
use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use tokio::sync::RwLock as ARwLock;
use hyper::Body;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use url::Url;
use std::fs;
use std::io::Read;
#[allow(deprecated)]
use base64::encode;
use indexmap::IndexMap;
use reqwest::Client;
use tokio::fs as async_fs;
use tracing::info;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::integrations::{get_empty_integrations, get_integration_path, get_integrations, json_for_integration, validate_integration_value};
use crate::yaml_configs::create_configs::{integrations_enabled_cfg, read_yaml_into_value, write_yaml_value};


#[derive(Serialize, Deserialize)]
struct IntegrationItem {
    name: String,
    enabled: bool,
    schema: Option<Value>,
    value: Option<Value>,
}

#[derive(Serialize)]
struct IntegrationIcon {
    name: String,
    value: String,
}

async fn load_integration_schema_and_json(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<IndexMap<String, (Value, Value)>, String> {
    let integrations = get_empty_integrations();
    let cache_dir = gcx.read().await.cache_dir.clone();
    let integrations_yaml_value = read_yaml_into_value(&cache_dir.join("integrations.yaml")).await?;

    let mut results = IndexMap::new();
    for (i_name, i) in integrations.iter() {
        let path = get_integration_path(&cache_dir, &i_name);
        let j_value = json_for_integration(&path, integrations_yaml_value.get(&i_name), &i).await?;
        results.insert(i_name.clone(), (i.to_schema_json(), j_value));
    }
    
    Ok(results)
}

async fn get_image_base64(
    cache_dir: &PathBuf, 
    icon_name: &str, 
    icon_url: &str,
) -> Result<String, String> {
    let assets_path = cache_dir.join("assets/integrations");

    // Parse the URL to get the file extension
    let url = Url::parse(icon_url).map_err(|e| e.to_string())?;
    let extension = url
        .path_segments()
        .and_then(|segments| segments.last())
        .and_then(|name| name.split('.').last())
        .unwrap_or("png"); // Default to "png" if no extension is found

    let file_path = assets_path.join(format!("{}.{}", icon_name, extension));

    // Check if the file already exists
    if file_path.exists() {
        info!("Using image from cache: {}", file_path.display());
        let mut file = fs::File::open(&file_path).map_err(|e| e.to_string())?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
        #[allow(deprecated)]
        let b64_image = encode(&buffer);
        let image_str = format!("data:{};base64,{}", extension, b64_image);
        return Ok(image_str);
    }

    // Create the cache directory if it doesn't exist
    async_fs::create_dir_all(&assets_path).await.map_err(|e| e.to_string())?;

    // Download the image
    info!("Downloading image from {}", icon_url);
    let client = Client::new();
    let response = client.get(icon_url).send().await.map_err(|e| e.to_string())?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;

    // Save the image to the cache directory
    async_fs::write(&file_path, &bytes).await.map_err(|e| e.to_string())?;

    // Return the base64 string
    #[allow(deprecated)]
    let b64_image = encode(&bytes);
    let image_str = format!("data:{};base64,{}", extension, b64_image);
    Ok(image_str)
}

pub async fn handle_v1_integrations_icons(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let integrations = get_integrations(gcx.clone()).await.map_err(|e|{
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load integrations: {}", e))
    })?;
    
    let mut results = vec![];
    for (i_name, i) in integrations.iter() {
        let image_base64 = get_image_base64(&cache_dir, i_name, &i.icon_link()).await.map_err(|e|{
            ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get image: {}", e))
        })?;
        results.push(IntegrationIcon {
            name: i_name.clone(),
            value: image_base64,
        });
    }

    let payload = serde_json::to_string_pretty(&json!(results)).expect("Failed to serialize results");
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(payload))
        .unwrap())
}

pub async fn handle_v1_integrations(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let schemas_and_json_dict = load_integration_schema_and_json(gcx.clone()).await.map_err(|e|{
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load integrations: {}", e))
    })?;
    
    let cache_dir = gcx.read().await.cache_dir.clone();
    let enabled_path = cache_dir.join("integrations-enabled.yaml");
    let enabled_mapping = match integrations_enabled_cfg(&enabled_path).await {
        serde_yaml::Value::Mapping(map) => map,
        _ => serde_yaml::Mapping::new(),
    };
    
    let mut items = vec![];
    for (name, (schema, value)) in schemas_and_json_dict {
        let item = IntegrationItem {
            name: name.clone(),
            enabled: enabled_mapping.get(&name).and_then(|v| v.as_bool()).unwrap_or(false),
            schema: Some(schema),
            value: Some(value),
        };
        
        items.push(item);
    }
    
    let payload = serde_json::to_string_pretty(&json!(items)).expect("Failed to serialize items");
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(payload))
        .unwrap())
}


pub async fn handle_v1_integrations_save(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<IntegrationItem>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let cache_dir = gcx.read().await.cache_dir.clone();
    let enabled_path = cache_dir.join("integrations-enabled.yaml");
    let mut enabled_value = integrations_enabled_cfg(&enabled_path).await;
    if let serde_yaml::Value::Mapping(ref mut map) = enabled_value {
        map.insert(serde_yaml::Value::String(post.name.clone()), serde_yaml::Value::Bool(post.enabled));
    } else {
        return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse {:?} as YAML::Mapping", enabled_path)));
    }
    write_yaml_value(&enabled_path, &enabled_value).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write YAML: {}", e)))?;
    
    if let Some(post_value) = &post.value {
        let yaml_value: serde_yaml::Value = serde_json::to_string(post_value).map_err(|e|e.to_string())
            .and_then(|s|serde_yaml::from_str(&s).map_err(|e|e.to_string()))
            .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("ERROR converting JSON to YAML: {}", e)))?;

        let yaml_value = validate_integration_value(&post.name, yaml_value).await
            .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("ERROR validating integration value: {}", e)))?;

        let path = get_integration_path(&cache_dir, &post.name);

        write_yaml_value(&path, &yaml_value).await.map_err(|e|{
            ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write YAML: {}", e))
        })?;
    }

    Ok(Response::builder()
       .status(StatusCode::OK)
       .header("Content-Type", "application/json")
       .body(Body::from(format!("Integration {} updated", post.name)))
       .unwrap())
}
