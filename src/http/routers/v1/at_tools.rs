use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use tokio::sync::RwLock as ARwLock;

use crate::at_tools::tools::{tools_compiled_in, tools_from_customization};
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;


pub async fn handle_v1_tools_available(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _: hyper::body::Bytes,
)  -> axum::response::Result<Response<Body>, ScratchError> {
    let turned_on = crate::at_tools::tools::at_tools_merged_and_filtered(gcx.clone()).await.keys().cloned().collect::<Vec<_>>();
    let tools_compiled_in_only = tools_compiled_in(&turned_on).unwrap_or_else(|e|{
        tracing::error!("Error loading compiled_in_tools: {:?}", e);
        vec![]
    });
    let tools_customization = tools_from_customization(gcx.clone(), &turned_on).await;
    let tools = tools_compiled_in_only.into_iter().map(|x|x.into_openai_style())
        .chain(tools_customization.into_iter().map(|x|x.into_openai_style())).collect::<Vec<_>>();

    let body = serde_json::to_string_pretty(&tools).map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap()
    )
}
