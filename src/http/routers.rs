use std::future::Future;
use std::pin::Pin;

use axum::Extension;
use axum::Router;
use axum::routing::{get, post};
use hyper::{Body, Response};

use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::http::routers::v1::caps::handle_v1_caps;
use crate::http::routers::v1::chat::handle_v1_chat;
use crate::http::routers::v1::code_completion::handle_v1_code_completion_web;
use crate::http::routers::v1::graceful_shutdown::handle_v1_graceful_shutdown;
use crate::http::routers::v1::snippet_accepted::handle_v1_snippet_accepted;
use crate::http::routers::v1::telemetry_network::handle_v1_telemetry_network;
use crate::http::utils::telemetry_wrapper;
use crate::telemetry_get;
use crate::telemetry_post;

pub mod v1;


pub fn make_v1_router() -> Router {
    Router::new()
        .route("/code-completion", telemetry_post!(handle_v1_code_completion_web))
        .route("/chat", telemetry_post!(handle_v1_chat))
        .route("/telemetry-network", telemetry_post!(handle_v1_telemetry_network))
        .route("/snippet-accepted", telemetry_post!(handle_v1_snippet_accepted))

        .route("/caps", telemetry_get!(handle_v1_caps))
        .route("/graceful-shutdown", telemetry_get!(handle_v1_graceful_shutdown))
}