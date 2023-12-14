use std::collections::HashMap;

use axum::http::Response;
use hyper::Body;
use serde_json::json;
use shadow_rs::shadow;

use crate::custom_error::ScratchError;

shadow!(build);

pub fn get_build_info() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("version", build::PKG_VERSION),
        ("commit", build::COMMIT_HASH),
        ("build_os", build::BUILD_OS),
        ("rust_version", build::RUST_VERSION),
        ("cargo_version", build::CARGO_VERSION),
    ])
}

pub async fn handle_info() -> axum::response::Result<Response<Body>, ScratchError> {
    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(json!(get_build_info()).to_string()))
        .unwrap())
}
