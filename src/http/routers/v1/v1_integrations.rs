use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::Deserialize;
// use url::Url;
// #[allow(deprecated)]
// use base64::encode;
// use indexmap::IndexMap;
use tokio::sync::RwLock as ARwLock;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
// use crate::integrations::{get_empty_integrations, get_integration_path};
// use crate::yaml_configs::create_configs::{integrations_enabled_cfg, read_yaml_into_value, write_yaml_value};


pub async fn handle_v1_integrations(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let with_icons = crate::integrations::setting_up_integrations::integrations_all_with_icons(gcx.clone()).await;
    let payload = serde_json::to_string_pretty(&with_icons).map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize payload: {}", e))
    })?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(payload))
        .unwrap())
}

#[derive(Deserialize)]
struct IntegrationGetPost {
    pub integr_config_path: String,
}

pub async fn handle_v1_integration_get(
    Extension(_gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<IntegrationGetPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let the_get = crate::integrations::setting_up_integrations::integration_config_get(
        post.integr_config_path,
    ).await.map_err(|e|{
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load integrations: {}", e))
    })?;

    let payload = serde_json::to_string_pretty(&the_get).map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize payload: {}", e))
    })?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(payload))
        .unwrap())
}

#[derive(Deserialize)]
struct IntegrationSavePost {
    pub integr_config_path: String,
    pub integr_values: serde_json::Value,
}

pub async fn handle_v1_integration_save(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<IntegrationSavePost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    crate::integrations::setting_up_integrations::integration_config_save(&post.integr_config_path, &post.integr_values).await.map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
    })?;

    Ok(Response::builder()
       .status(StatusCode::OK)
       .header("Content-Type", "application/json")
       .body(Body::from(format!("")))
       .unwrap())
}


// async fn get_image_base64(
//     cache_dir: &PathBuf,
//     icon_name: &str,
//     icon_url: &str,
// ) -> Result<String, String> {
//     let assets_path = cache_dir.join("assets/integrations");

//     // Parse the URL to get the file extension
//     let url = Url::parse(icon_url).map_err(|e| e.to_string())?;
//     let extension = url
//         .path_segments()
//         .and_then(|segments| segments.last())
//         .and_then(|name| name.split('.').last())
//         .unwrap_or("png"); // Default to "png" if no extension is found

//     let file_path = assets_path.join(format!("{}.{}", icon_name, extension));

//     // Check if the file already exists
//     if file_path.exists() {
//         info!("Using image from cache: {}", file_path.display());
//         let mut file = fs::File::open(&file_path).map_err(|e| e.to_string())?;
//         let mut buffer = Vec::new();
//         file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
//         #[allow(deprecated)]
//         let b64_image = encode(&buffer);
//         let image_str = format!("data:{};base64,{}", extension, b64_image);
//         return Ok(image_str);
//     }

//     // Create the cache directory if it doesn't exist
//     async_fs::create_dir_all(&assets_path).await.map_err(|e| e.to_string())?;

//     // Download the image
//     info!("Downloading image from {}", icon_url);
//     let client = Client::new();
//     let response = client.get(icon_url).send().await.map_err(|e| e.to_string())?;
//     let bytes = response.bytes().await.map_err(|e| e.to_string())?;

//     // Save the image to the cache directory
//     async_fs::write(&file_path, &bytes).await.map_err(|e| e.to_string())?;

//     // Return the base64 string
//     #[allow(deprecated)]
//     let b64_image = encode(&bytes);
//     let image_str = format!("data:{};base64,{}", extension, b64_image);
//     Ok(image_str)
// }
