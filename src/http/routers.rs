use std::future::Future;

use axum::Router;
use axum::routing::get;

use crate::http::handler_404;

pub mod v1;
pub mod info;


pub fn make_refact_http_server() -> Router {
    Router::new()
        .fallback(handler_404)
        .nest("/v1", v1::make_v1_router())
        .route("/info", get(info::handle_info))
}
