use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use tokio::sync::RwLock as ARwLock;


pub async fn handle_v1_tools_available(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _: hyper::body::Bytes,
)  -> axum::response::Result<Response<Body>, ScratchError> {
    let mut result: Vec<serde_json::Value> = vec![];
    let tools_compiled_in = crate::at_tools::at_tools::at_tools_compiled_in_only();
    if tools_compiled_in.is_err() {
        tracing::error!("Error loading tools: {:?}", tools_compiled_in.err().unwrap());
    } else {
        for tool in tools_compiled_in.unwrap() {
            result.push(tool.into_openai_style());
        }
    }

    let tconfig_maybe = crate::toolbox::toolbox_config::load_customization(gcx.clone()).await;
    if tconfig_maybe.is_err() {
        tracing::error!("Error loading toolbox config: {:?}", tconfig_maybe.err().unwrap());
    } else {
        for x in tconfig_maybe.unwrap().tools {
            result.push(x.into_openai_style());
        }
    }

    let body = serde_json::to_string_pretty(&result).map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap()
    )
}
