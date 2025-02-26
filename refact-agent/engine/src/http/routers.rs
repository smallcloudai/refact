use axum::Router;
use axum::routing::get;
use axum::http::{Method, HeaderValue, header};
use tower_http::cors::CorsLayer;
use std::time::Duration;

use crate::http::handler_404;

pub mod v1;
pub mod info;


pub fn make_refact_http_server() -> Router {
    // Create a CORS layer that allows specific methods and all origins
    let cors = CorsLayer::new()
        .allow_origin("*".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
        ])
        .max_age(Duration::from_secs(3600));

    Router::new()
        .fallback(handler_404)
        .nest("/v1", v1::make_v1_router())
        .nest("/db_v1", v1::make_db_v1_router())
        .route("/build_info", get(info::handle_info))
        .layer(cors)  // Add the CORS middleware
}
