use std::any::Any;
use std::path::PathBuf;
use std::sync::Arc;
use std::future::Future;
use tokio::sync::Mutex as AMutex;
use tokio::task::{AbortHandle, JoinHandle};
use rmcp::{RoleClient, service::RunningService};
use rmcp::model::{Annotated, RawResource, Tool as McpTool};
use tokio::time::{timeout, Duration};

use crate::integrations::sessions::IntegrationSession;
use crate::integrations::process_io_utils::read_file_with_cursor;
use super::integr_mcp::SettingsMCP;

pub struct SessionMCP {
    pub debug_name: String,
    pub config_path: String,        // to check if expired or not
    pub launched_cfg: SettingsMCP,  // a copy to compare against IntegrationMCP::cfg, to see if anything has changed
    pub mcp_client: Option<Arc<AMutex<Option<RunningService<RoleClient, ()>>>>>,
    pub mcp_tools: Vec<McpTool>,
    pub mcp_resources: Option<Vec<Annotated<RawResource>>>,
    pub startup_task_handles: Option<(Arc<AMutex<Option<JoinHandle<()>>>>, AbortHandle)>,
    pub logs: Arc<AMutex<Vec<String>>>,          // Store log messages
    pub stderr_file_path: Option<PathBuf>,       // Path to the temporary file for stderr
    pub stderr_cursor: Arc<AMutex<u64>>,         // Position in the file where we last read from
}

impl IntegrationSession for SessionMCP {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn is_expired(&self) -> bool {
        !std::path::Path::new(&self.config_path).exists()
    }

    fn try_stop(&mut self, self_arc: Arc<AMutex<Box<dyn IntegrationSession>>>) -> Box<dyn Future<Output = String> + Send> {
        Box::new(async move {
            let (debug_name, client, logs, startup_task_handles, stderr_file) = {
                let mut session_locked = self_arc.lock().await;
                let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
                (
                    session_downcasted.debug_name.clone(),
                    session_downcasted.mcp_client.clone(),
                    session_downcasted.logs.clone(),
                    session_downcasted.startup_task_handles.clone(),
                    session_downcasted.stderr_file_path.clone(),
                )
            };

            if let Some((_, abort_handle)) = startup_task_handles {
                _add_log_entry(logs.clone(), "Aborted startup task".to_string()).await;
                abort_handle.abort();
            }

            if let Some(client) = client {
                _session_kill_process(&debug_name, client, logs).await;
            }
            if let Some(stderr_file) = &stderr_file {
                if let Err(e) = tokio::fs::remove_file(stderr_file).await {
                    tracing::error!("Failed to remove {}: {}", stderr_file.to_string_lossy(), e);
                }
            }

            "".to_string()
        })
    }
}

pub async fn _add_log_entry(session_logs: Arc<AMutex<Vec<String>>>, entry: String) {
    let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
    let log_entry = format!("[{}] {}", timestamp, entry);

    let mut session_logs_locked = session_logs.lock().await;
    session_logs_locked.extend(log_entry.lines().into_iter().map(|s| s.to_string()));

    if session_logs_locked.len() > 100 {
        let excess = session_logs_locked.len() - 100;
        session_logs_locked.drain(0..excess);
    }
}

pub async fn update_logs_from_stderr(
    stderr_file_path: &PathBuf,
    stderr_cursor: Arc<AMutex<u64>>,
    session_logs: Arc<AMutex<Vec<String>>>
) -> Result<(), String> {
    let (buffer, bytes_read) = read_file_with_cursor(stderr_file_path, stderr_cursor.clone()).await
        .map_err(|e| format!("Failed to read file: {}", e))?;
    if bytes_read > 0 && !buffer.trim().is_empty() {
        _add_log_entry(session_logs, buffer.trim().to_string()).await;
    }
    Ok(())
}

pub async fn _session_kill_process(
    debug_name: &str,
    mcp_client: Arc<AMutex<Option<RunningService<RoleClient, ()>>>>,
    session_logs: Arc<AMutex<Vec<String>>>,
) {
    tracing::info!("Stopping MCP Server for {}", debug_name);
    _add_log_entry(session_logs.clone(), "Stopping MCP Server".to_string()).await;

    let client_to_cancel = {
        let mut mcp_client_locked = mcp_client.lock().await;
        mcp_client_locked.take()
    };

    if let Some(client) = client_to_cancel {
        match timeout(Duration::from_secs(3), client.cancel()).await {
            Ok(Ok(reason)) => {
                let success_msg = format!("MCP server stopped: {:?}", reason);
                tracing::info!("{} for {}", success_msg, debug_name);
                _add_log_entry(session_logs, success_msg).await;
            },
            Ok(Err(e)) => {
                let error_msg = format!("Failed to stop MCP: {:?}", e);
                tracing::error!("{} for {}", error_msg, debug_name);
                _add_log_entry(session_logs, error_msg).await;
            },
            Err(_) => {
                let error_msg = "MCP server stop operation timed out after 3 seconds".to_string();
                tracing::error!("{} for {}", error_msg, debug_name);
                _add_log_entry(session_logs, error_msg).await;
            }
        }
    }
}

pub async fn _session_wait_startup_task(
    session_arc: Arc<AMutex<Box<dyn IntegrationSession>>>,
) {
    let startup_task_handles = {
        let mut session_locked = session_arc.lock().await;
        let session_downcasted = session_locked.as_any_mut().downcast_mut::<SessionMCP>().unwrap();
        session_downcasted.startup_task_handles.clone()
    };

    if let Some((join_handler_arc, _)) = startup_task_handles {
        let mut join_handler_locked = join_handler_arc.lock().await;
        if let Some(join_handler) = join_handler_locked.take() {
            let _ = join_handler.await;
        }
    }
}
