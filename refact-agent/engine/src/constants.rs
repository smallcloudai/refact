use tracing::info;
use url::Url;

const BASE_REFACT_URL: &str = "app.refact.ai";

/// Extracts the host (and optional port) from a URL string, e.g.:
///   ws://app.refact.ai/v1/graphql -> app.refact.ai
///   https://example.com:8080/path -> example.com:8080
///   app.refact.ai -> app.refact.ai
fn extract_base_host(address: &str) -> String {
    if let Ok(url) = Url::parse(address) {
        if let Some(host) = url.host_str() {
            return if let Some(port) = url.port() {
                format!("{}:{}", host, port)
            } else {
                host.to_string()
            }
        }
    }
    let mut address = address;
    for prefix in ["ws://", "wss://", "http://", "https://"] {
        if let Some(stripped) = address.strip_prefix(prefix) {
            address = stripped;
            break;
        }
    }
    let address = address;
    if let Some(idx) = address.find('/') {
        address[..idx].to_string()
    } else {
        address.to_string()
    }
}

fn is_localhost(address: &str) -> bool {
    let address = if let Ok(url) = Url::parse(address) {
        if let Some(host) = url.host_str() {
            host.to_string()
        } else {
            if let Some(idx) = address.find(':') {
                address[..idx].to_string()
            } else {
                address.to_string()
            }
        }
    } else {
        if let Some(idx) = address.find(':') {
            address[..idx].to_string()
        } else {
            address.to_string()
        }
    };
    match address.to_ascii_lowercase().as_str() {
        "localhost" | "127.0.0.1" | "::1" | "[::1]" => true,
        _ => false,
    }
}

pub fn get_cloud_url(cmd_address_url: &str) -> String {
    let final_address = if cmd_address_url.to_lowercase() == "refact" {
        format!("https://{}/v1", BASE_REFACT_URL)
    } else {
        let base_part = extract_base_host(cmd_address_url);
        let protocol = if is_localhost(&base_part) { "http" } else { "https" };
        format!("{}://{}/v1", protocol, base_part)
    };
    info!("resolved cloud url: {}", final_address);
    final_address
}

pub fn get_graphql_ws_url(cmd_address_url: &str) -> String {
    let final_address = if cmd_address_url.to_lowercase() == "refact" {
        format!("wss://{}/v1/graphql", BASE_REFACT_URL)
    } else {
        let base_part = extract_base_host(cmd_address_url);
        let protocol = if is_localhost(&base_part) { "ws" } else { "wss" };
        format!("{}://{}/v1/graphql", protocol, base_part)
    };
    info!("resolved graphql ws url: {}", final_address);
    final_address
}

pub fn get_graphql_url(cmd_address_url: &str) -> String {
    let final_address = if cmd_address_url.to_lowercase() == "refact" {
        format!("https://{}/v1/graphql", BASE_REFACT_URL)
    } else {
        let base_part = extract_base_host(cmd_address_url);
        let protocol = if is_localhost(&base_part) { "http" } else { "https" };
        format!("{}://{}/v1/graphql", protocol, base_part)
    };
    info!("resolved graphql url: {}", final_address);
    final_address
}
