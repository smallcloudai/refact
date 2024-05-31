use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use crate::at_tools::at_tools_dict::at_tools_dicts;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use tokio::sync::RwLock as ARwLock;
use crate::toolbox::toolbox_config::at_custom_tools_dicts;


pub async fn handle_v1_tools_available(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    _: hyper::body::Bytes,
)  -> axum::response::Result<Response<Body>, ScratchError> {
    let at_dict = at_tools_dicts().map_err(|e| {
        tracing::warn!("can't load at_commands_dicts: {}", e);
        return ScratchError::new(StatusCode::NOT_FOUND, format!("can't load at_commands_dicts: {}", e));
    })?;
    let at_cust_dict = at_custom_tools_dicts(global_context.clone()).await.map_err(|e| {
        tracing::warn!("can't load at_custom_tools_dicts: {}", e);
        return ScratchError::new(StatusCode::NOT_FOUND, format!("can't load at_custom_tools_dicts: {}", e));
    })?;
    
    let dicts_combined = at_dict.iter().map(|x|x.clone().into_openai_style())
        .chain(at_cust_dict.iter().map(|x|x.clone().into_openai_style())).collect::<Vec<_>>();
    
    let body = serde_json::to_string_pretty(&dicts_combined).map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap()
    )
}
