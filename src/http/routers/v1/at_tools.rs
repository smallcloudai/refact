use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use tokio::sync::RwLock as ARwLock;

use crate::tools::tools_description::{tool_description_list_from_yaml, tools_from_customization};
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;


pub async fn handle_v1_tools(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _: hyper::body::Bytes,
)  -> axum::response::Result<Response<Body>, ScratchError> {
    let turned_on = crate::tools::tools_description::tools_merged_and_filtered(gcx.clone()).await.keys().cloned().collect::<Vec<_>>();
    let allow_experimental = gcx.read().await.cmdline.experimental;

    let tool_desclist = tool_description_list_from_yaml(&turned_on, allow_experimental).unwrap_or_else(|e|{
        tracing::error!("Error loading compiled_in_tools: {:?}", e);
        vec![]
    });

    let tools_openai_stype = tool_desclist.into_iter().map(|x|x.into_openai_style()).collect::<Vec<_>>();

    let body = serde_json::to_string_pretty(&tools_openai_stype).map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap()
    )
}
