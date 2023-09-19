use tracing::error;
use hyper::{Body, Response, StatusCode};
use serde_json::json;
use std::fmt;


#[derive(Debug)]
pub struct ScratchError {
    pub status_code: StatusCode,
    pub message: String,
    pub telemetry_skip: bool    // because already posted a better description directly
}

impl std::error::Error for ScratchError {}

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
