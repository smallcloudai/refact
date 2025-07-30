use std::net::IpAddr;

use tracing::info;
use url::Url;

const BASE_REFACT_URL: &str = "app.refact.ai";

/// Extracts the host (and optional port) from a URL string and determines if the protocol is secure, e.g.:
///   ws://app.refact.ai/v1/graphql -> (app.refact.ai, Some(false))
///   https://example.com:8080/path -> (example.com:8080, Some(true))
///   app.refact.ai -> (app.refact.ai, None)
fn get_host_and_is_protocol_secure(address: &str) -> (String, Option<bool>) {
    if let Ok(url) = Url::parse(address) {
        if let Some(host) = url.host_str() {
            let host_with_port = if let Some(port) = url.port() {
                format!("{}:{}", host, port)
            } else {
                host.to_string()
            };

            let is_secure = match url.scheme() {
                "https" | "wss" => Some(true),
                "http" | "ws" => Some(false),
                _ => None,
            };

            return (host_with_port, is_secure);
        }
    }

    let mut address = address;
    let mut is_secure = None;

    for (prefix, secure) in [
        ("https://", Some(true)),
        ("wss://", Some(true)),
        ("http://", Some(false)),
        ("ws://", Some(false)),
    ] {
        if let Some(stripped) = address.strip_prefix(prefix) {
            address = stripped;
            is_secure = secure;
            break;
        }
    }

    let host = if let Some(idx) = address.find('/') {
        address[..idx].to_string()
    } else {
        address.to_string()
    };

    (host, is_secure)
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
    if let Ok(ip) = address.parse::<IpAddr>() {
        return ip.is_loopback();
    }
    match address.to_ascii_lowercase().as_str() {
        "localhost" | "127.0.0.1" | "::1" | "[::1]" | "host.docker.internal" => true,
        _ => false,
    }
}

pub fn get_cloud_url(cmd_address_url: &str) -> String {
    let final_address = if cmd_address_url.to_lowercase() == "refact" {
        format!("https://{}/v1", BASE_REFACT_URL)
    } else {
        let (host, is_secure) = get_host_and_is_protocol_secure(cmd_address_url);
        let protocol = match is_secure {
            Some(true) => "https",
            Some(false) => "http",
            None => if is_localhost(&host) { "http" } else { "https" },
        };
        format!("{}://{}/v1", protocol, host)
    };
    info!("resolved cloud url: {}", final_address);
    final_address
}

pub fn get_graphql_ws_url(cmd_address_url: &str) -> String {
    let final_address = if cmd_address_url.to_lowercase() == "refact" {
        format!("wss://{}/v1/graphql", BASE_REFACT_URL)
    } else {
        let (host, is_secure) = get_host_and_is_protocol_secure(cmd_address_url);
        let protocol = match is_secure {
            Some(true) => "wss",
            Some(false) => "ws",
            None => if is_localhost(&host) { "ws" } else { "wss" },
        };
        format!("{}://{}/v1/graphql", protocol, host)
    };
    info!("resolved graphql ws url: {}", final_address);
    final_address
}

pub fn get_graphql_url(cmd_address_url: &str) -> String {
    let final_address = if cmd_address_url.to_lowercase() == "refact" {
        format!("https://{}/v1/graphql", BASE_REFACT_URL)
    } else {
        let (host, is_secure) = get_host_and_is_protocol_secure(cmd_address_url);
        let protocol = match is_secure {
            Some(true) => "https",
            Some(false) => "http",
            None => if is_localhost(&host) { "http" } else { "https" },
        };
        format!("{}://{}/v1/graphql", protocol, host)
    };
    info!("resolved graphql url: {}", final_address);
    final_address
}
