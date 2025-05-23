use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Weak;
use async_trait::async_trait;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use tokio::time::timeout;
use tokio::time::Duration;
use rmcp::transport::common::client_side_sse::ExponentialBackoff;
use rmcp::transport::sse_client::{SseClientTransport, SseClientConfig};
use rmcp::serve_client;
use rmcp::{RoleClient, service::RunningService};
use serde::{Deserialize, Serialize};

use crate::global_context::GlobalContext;
use crate::integrations::integr_abstract::{IntegrationTrait, IntegrationCommon};
use super::session_mcp::add_log_entry;
use super::integr_mcp_common::{CommonMCPSettings, MCPTransportInitializer, mcp_integr_tools, mcp_session_setup};

#[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
pub struct SettingsMCPSse {
    #[serde(default, rename = "url")]
    pub mcp_url: String,
    #[serde(default = "default_headers", rename = "headers")]
    pub mcp_headers: HashMap<String, String>,
    #[serde(flatten)]
    pub common: CommonMCPSettings,
}

pub fn default_headers() -> HashMap<String, String> {
    HashMap::from([
        ("User-Agent".to_string(), "Refact.ai (+https://github.com/smallcloudai/refact)".to_string()),
        ("Accept".to_string(), "text/event-stream".to_string()),
        ("Content-Type".to_string(), "application/json".to_string()),
    ])
}

#[derive(Default, Clone)]
pub struct IntegrationMCPSse {
    pub gcx_option: Option<Weak<ARwLock<GlobalContext>>>,
    pub cfg: SettingsMCPSse,
    pub common: IntegrationCommon,
    pub config_path: String,
}

#[async_trait]
impl MCPTransportInitializer for IntegrationMCPSse {
    async fn init_mcp_transport(
        &self,
        logs: Arc<AMutex<Vec<String>>>,
        debug_name: String,
        init_timeout: u64,
        _request_timeout: u64,
        _session: Arc<AMutex<Box<dyn crate::integrations::sessions::IntegrationSession>>>
    ) -> Option<RunningService<RoleClient, ()>> {
        let log = async |level: tracing::Level, msg: String| {
            match level {
                tracing::Level::ERROR => tracing::error!("{msg} for {debug_name}"),
                tracing::Level::WARN => tracing::warn!("{msg} for {debug_name}"),
                _ => tracing::info!("{msg} for {debug_name}"),
            }
            add_log_entry(logs.clone(), msg).await;
        };

        let url = self.cfg.mcp_url.trim();
        if url.is_empty() {
            log(tracing::Level::ERROR, "URL is empty for SSE transport".to_string()).await;
            return None;
        }

        let mut header_map = reqwest::header::HeaderMap::new();
        for (k, v) in &self.cfg.mcp_headers {
            match (reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                reqwest::header::HeaderValue::from_str(v),
            ) {
                (Ok(name), Ok(value)) => {
                    header_map.insert(name, value);
                }
                _ => log(tracing::Level::WARN, format!("Invalid header: {}: {}", k, v)).await,
            }
        }

        let client = match reqwest::Client::builder().default_headers(header_map).build() {
            Ok(reqwest_client) => reqwest_client,
            Err(e) => {
                log(tracing::Level::ERROR, format!("Failed to build reqwest client: {}", e)).await;
                return None;
            }
        };

        let client_config = SseClientConfig {
            sse_endpoint: Arc::<str>::from(url),
            retry_policy: Arc::new(ExponentialBackoff {
                max_times: Some(3),
                base_duration: Duration::from_millis(500),
            }),
            ..Default::default()
        };

        let transport = match SseClientTransport::start_with_client(client, client_config).await {
            Ok(t) => t,
            Err(e) => {
                log(tracing::Level::ERROR, format!("Failed to init SSE transport: {}", e)).await;
                return None;
            }
        };

        match timeout(Duration::from_secs(init_timeout), serve_client((), transport)).await {
            Ok(Ok(client)) => Some(client),
            Ok(Err(e)) => {
                log(tracing::Level::ERROR, format!("Failed to init SSE server: {}", e)).await;
                None
            },
            Err(_) => {
                log(tracing::Level::ERROR, format!("Request timed out after {} seconds", init_timeout)).await;
                None
            }
        }
    }
}

#[async_trait]
impl IntegrationTrait for IntegrationMCPSse {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn integr_settings_apply(&mut self, gcx: Arc<ARwLock<GlobalContext>>, config_path: String, value: &serde_json::Value) -> Result<(), serde_json::Error> {
        self.gcx_option = Some(Arc::downgrade(&gcx));
        self.cfg = serde_json::from_value(value.clone())?;
        self.common = serde_json::from_value(value.clone())?;
        self.config_path = config_path.clone();

        mcp_session_setup(
            gcx,
            config_path,
            serde_json::to_value(&self.cfg).unwrap_or_default(),
            self.clone(),
            self.cfg.common.init_timeout,
            self.cfg.common.request_timeout
        ).await;

        Ok(())
    }

    fn integr_settings_as_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    async fn integr_tools(&self, _integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        mcp_integr_tools(
            self.gcx_option.clone(),
            &self.config_path,
            &self.common,
            self.cfg.common.request_timeout
        ).await
    }

    fn integr_schema(&self) -> &str {
        include_str!("mcp_sse_schema.yaml")
    }
}
