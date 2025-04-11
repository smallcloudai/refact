use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;

use reqwest;
use serde::{Serialize, Deserialize};
use tracing::info;
use tokio::io;
use tokio::io::AsyncBufReadExt;
use crate::dashboard::dashboard::records2plots;
use crate::dashboard::structs::RHData;


#[derive(Debug, Deserialize)]
struct RHResponse {
    // retcode: String,
    data: Vec<RHData>,
}

#[derive(Debug, Serialize)]
struct DashboardPlotsResponse {
    data: String,
}

async fn fetch_data(
    http_client: &reqwest::Client,
    url: &String,
    api_key: &String,
) -> Result<Vec<RHData>, String> {
    let response = match http_client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send().await {
        Ok(response) => response,
        Err(e) => return Err(format!("Error fetching reports: {}", e)),
    };
    info!("{:?}", &response.status());
    if !response.status().is_success() {
        return Err(format!("Error fetching reports: status code: {}", response.status()));
    }
    let body_mb = response.bytes().await;
    if body_mb.is_err() {
        return Err("Error fetching reports".to_string());
    }
    let body = body_mb.unwrap();
    let mut reader = io::BufReader::new(&body[..]);
    let mut line = String::new();
    let mut data = vec![];
    while reader.read_line(&mut line).await.is_ok() {
        let response_data_mb: Result<RHResponse, _> = serde_json::from_str(&line);
        if response_data_mb.is_err() {
            break;
        }
        data.extend(response_data_mb.unwrap().data);
        line.clear();
    }
    Ok(data)
}

pub async fn get_dashboard_plots(
    Extension(global_context): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {

    let caps = crate::global_context::try_load_caps_quickly_if_not_present(global_context.clone(), 0).await?;
    let (http_client, api_key, url) = {
        let gcx_locked = global_context.read().await;
        (gcx_locked.http_client.clone(), gcx_locked.cmdline.api_key.clone(), caps.telemetry_basic_retrieve_my_own.clone())
    };
    if url.is_empty() {
        return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "Error: no url provided from caps".to_string()));
    }

    let mut records = match fetch_data(
        &http_client,
        &url,
        &api_key
    ).await {
        Ok(res) => res,
        Err(e) => {
            return Err(ScratchError::new(StatusCode::NO_CONTENT, format!("Error fetching reports: {}", e)));
        }
    };

    let plots = match records2plots(&mut records).await {
        Ok(plots) => plots,
        Err(e) => {
            return Err(ScratchError::new(StatusCode::NO_CONTENT, format!("Error plotting reports: {}", e)));
        }
    };
    let body = match serde_json::to_string_pretty(&DashboardPlotsResponse{data: plots.to_string()}) {
        Ok(res) => res,
        Err(e) => {
            return Err(ScratchError::new(StatusCode::NO_CONTENT, format!("Error serializing plots: {}", e)));
        }
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(body))
        .unwrap())
}
