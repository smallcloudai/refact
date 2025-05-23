use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Weak;
use std::process::Stdio;
use async_trait::async_trait;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use tokio::time::timeout;
use tokio::time::Duration;
use rmcp::serve_client;
use rmcp::{RoleClient, service::RunningService};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::global_context::GlobalContext;
use crate::integrations::integr_abstract::{IntegrationTrait, IntegrationCommon};
use super::session_mcp::add_log_entry;
use super::integr_mcp_common::{CommonMCPSettings, MCPTransportInitializer, mcp_integr_tools, mcp_session_setup};

#[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
pub struct SettingsMCPStdio {
    #[serde(rename = "command", default)]
    pub mcp_command: String,
    #[serde(default, rename = "env")]
    pub mcp_env: HashMap<String, String>,
    #[serde(flatten)]
    pub common: CommonMCPSettings,
}

#[derive(Default, Clone)]
pub struct IntegrationMCPStdio {
    pub gcx_option: Option<Weak<ARwLock<GlobalContext>>>,
    pub cfg: SettingsMCPStdio,
    pub common: IntegrationCommon,
    pub config_path: String,
}

#[async_trait]
impl MCPTransportInitializer for IntegrationMCPStdio {
    async fn init_mcp_transport(
        &self,
        logs: Arc<AMutex<Vec<String>>>,
        debug_name: String,
        init_timeout: u64,
        _request_timeout: u64,
        session_arc_clone: Arc<AMutex<Box<dyn crate::integrations::sessions::IntegrationSession>>>
    ) -> Option<RunningService<RoleClient, ()>> {
        let log = async |level: tracing::Level, msg: String| {
            match level {
                tracing::Level::ERROR => tracing::error!("{msg} for {debug_name}"),
                tracing::Level::WARN => tracing::warn!("{msg} for {debug_name}"),
                _ => tracing::info!("{msg} for {debug_name}"),
            }
            add_log_entry(logs.clone(), msg).await;
        };

        let command = self.cfg.mcp_command.trim();
        if command.is_empty() {
            log(tracing::Level::ERROR, "Command is empty for STDIO transport".to_string()).await;
            return None;
        }

        let parsed_args = match shell_words::split(&command) {
            Ok(args) => {
                if args.is_empty() {
                    log(tracing::Level::ERROR, "Empty command".to_string()).await;
                    return None;
                }
                args
            }
            Err(e) => {
                log(tracing::Level::ERROR, format!("Failed to parse command: {}", e)).await;
                return None;
            }
        };

        let mut command = tokio::process::Command::new(&parsed_args[0]);
        command.args(&parsed_args[1..]);
        for (key, value) in &self.cfg.mcp_env {
            command.env(key, value);
        }

        match NamedTempFile::new().map(|f| f.keep()) {
            Ok(Ok((file, path))) => {
                {
                    let mut session_locked = session_arc_clone.lock().await;
                    if let Some(mcp_session) = session_locked.as_any_mut().downcast_mut::<super::session_mcp::SessionMCP>() {
                        mcp_session.stderr_file_path = Some(path.clone());
                        mcp_session.stderr_cursor = Arc::new(AMutex::new(0));
                    }
                }
                command.stderr(Stdio::from(file));
            },
            Ok(Err(e)) => tracing::error!("Failed to persist stderr file for {debug_name}: {e}"),
            Err(e)  => tracing::error!("Failed to create stderr file for {debug_name}: {e}"),
        }

        let transport = match rmcp::transport::TokioChildProcess::new(command) {
            Ok(t) => t,
            Err(e) => {
                log(tracing::Level::ERROR, format!("Failed to init Tokio child process: {}", e)).await;
                return None;
            }
        };

        match timeout(Duration::from_secs(init_timeout), serve_client((), transport)).await {
            Ok(Ok(client)) => Some(client),
            Ok(Err(e)) => {
                log(tracing::Level::ERROR, format!("Failed to init stdio server: {}", e)).await;
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
impl IntegrationTrait for IntegrationMCPStdio {
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
        include_str!("mcp_stdio_schema.yaml")
    }
}
