use std::error::Error;
use tracing::error;
use hyper::{Body, Response, StatusCode};
use serde_json::json;
use std::fmt;
use axum::Json;
use axum::response::IntoResponse;


#[derive(Debug, Clone)]
pub struct ScratchError {
    pub status_code: StatusCode,
    pub message: String,
    pub telemetry_skip: bool,    // because already posted a better description directly
}

impl IntoResponse for ScratchError {
    fn into_response(self) -> axum::response::Response {
        let payload = json!({
            "detail": self.message,
        });
        (self.status_code, Json(payload)).into_response()
    }
}

impl Error for ScratchError {}
unsafe impl Send for ScratchError {}
unsafe impl Sync for ScratchError {}
impl fmt::Display for ScratchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.status_code, self.message)
    }
}

impl ScratchError {
    pub fn new(status_code: StatusCode, message: String) -> Self {
        ScratchError {
            status_code,
            message,
            telemetry_skip: false,
        }
    }

    pub fn new_but_skip_telemetry(status_code: StatusCode, message: String) -> Self {
        ScratchError {
            status_code,
            message,
            telemetry_skip: true,
        }
    }

    pub fn to_response(&self) -> Response<Body> {
        let body = json!({"detail": self.message}).to_string();
        error!("client will see {}", body);
        let response = Response::builder()
            .status(self.status_code)
            .header("Content-Type", "application/json")
            .body(Body::from(body))
            .unwrap();
        response
    }
}

pub trait MapErrToString<T> {
    /// Same as .map_err(|e| e.to_string())
    fn map_err_to_string(self) -> Result<T, String>;
    /// Same as .map_err(|e| format!("{} {}", pref, e))
    fn map_err_with_prefix<P: std::fmt::Display>(self, pref: P) -> Result<T, String>;
}

impl<T, E: std::fmt::Display> MapErrToString<T> for Result<T, E> {
    fn map_err_to_string(self) -> Result<T, String> {
        self.map_err(|e| e.to_string())
    }

    fn map_err_with_prefix<P: std::fmt::Display>(self, pref: P) -> Result<T, String> {
        self.map_err(|e| format!("{pref} {e}"))
    }
}