use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;
use tokio::io::BufReader;
use async_trait::async_trait;
use regex::Regex;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tracing::{info, warn};
use tokio::process::{Command, Child, ChildStdout, ChildStderr};
use tokio::time::{timeout, Duration};
use std::process::Stdio;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{AtParamDict, Tool};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::global_context::GlobalContext;
use crate::integrations::process_io_utils::{kill_process_and_children, read_until_token_or_timeout};
use crate::integrations::sessions::IntegrationSession;


pub struct ToolCustom {
    pub name: String,
    #[allow(dead_code)]
    pub parameters: Vec<AtParamDict>,
    pub parameters_required: Vec<String>,
    pub command: String,
    pub runs_in_background: bool,
    pub runs_in_background_false_timeout: usize,
}

pub struct ToolSession {
    process: Child,
    #[allow(dead_code)]
    stdout: BufReader<ChildStdout>,
    #[allow(dead_code)]
    stderr: BufReader<ChildStderr>,
}

impl IntegrationSession for ToolSession {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn is_expired(&self) -> bool { false }
}

fn replace_magics_from_command(
    command: &str,
    args: &HashMap<String, Value>,
    required_params: &Vec<String>
) -> Result<String, String> {
    let args_str: HashMap<String, String> = args.iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
        .collect();

    let mut command_clone = command.to_string();
    for (key, value) in args_str {
        let pattern = format!("%{}%", key);
        let re = Regex::new(&regex::escape(&pattern)).unwrap();
        command_clone = re.replace_all(&command_clone, value.as_str()).to_string();
    }

    for param in required_params {
        let pattern = format!("%{}%", param);
        if command_clone.contains(&pattern) {
            return Err(format!("Required parameter '{}' is missing in the arguments", param));
        }
    }
    Ok(command_clone)
}

async fn execute_command_with_timeout(
    command: &str,
    timeout_duration: Duration
) -> Result<String, String> {
    info!("EXEC: {command}");
    let command_future = async {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let mut res = "".to_string();
        if output.status.success() {
            res.push_str(&format!("EXEC SUCCESS: {command}\n"));
        } else {
            res.push_str(&format!("EXEC FAIL: {command}\n"));
        }
        res.push_str(&format!("STDOUT:\n{stdout}"));
        if !stderr.is_empty() {
            res.push_str(&format!("\nSTDERR:\n{stderr}"));
        }
        Ok(res)
    };

    timeout(timeout_duration, command_future).await
        .unwrap_or_else(|_| Err("Command execution timed out".to_string()))
}

async fn start_session(
    gcx: Arc<ARwLock<GlobalContext>>,
    session_key: &str,
    command: &str,
) -> Result<String, String> {
    info!("EXEC: {command}");
    let mut process = Command::new("sh")
       .arg("-c")
       .arg(command)
       .stdout(Stdio::piped())
       .stderr(Stdio::piped())
       .spawn()
       .map_err(|e| format!("failed to create process: {e}"))?;

    let mut stdout = BufReader::new(process.stdout.take().ok_or("Failed to open stdout")?);
    let mut stderr = BufReader::new(process.stderr.take().ok_or("Failed to open stderr")?);

    let stdout_out = read_until_token_or_timeout(&mut stdout, 1000, "").await?;
    let stderr_out = read_until_token_or_timeout(&mut stderr, 1000, "").await?;

    let mut out = String::new();
    if !stdout_out.is_empty() {
        out.push_str(&format!("STDOUT:\n{stdout_out}"));
    }
    if !stderr_out.is_empty() {
        out.push_str(&format!("STDERR:\n{stderr_out}"));
    }

    // todo: check if RegExp is in stdout/ stderr to ensure that the tool is running
    
    let exit_status = process.try_wait().map_err(|e| e.to_string())?;
    if exit_status.is_some() {
        let status = exit_status.unwrap().code().unwrap();
        warn!("tool process exited with status: {:?}. Output:\n{out}", status);
        return Err(format!("tool process exited with status: {:?}; Output:\n{out}", status));
    }
    
    let session: Box<dyn IntegrationSession> = Box::new(ToolSession { process, stdout, stderr});
    gcx.write().await.integration_sessions.insert(session_key.to_string(), Arc::new(AMutex::new(session)));
    
    Ok(out)
}

async fn execute_background_command(
    gcx: Arc<ARwLock<GlobalContext>>,
    tool_name: &str,
    command: &str,
    // restart, stop, status, [start=default]
    args: &HashMap<String, Value>,
) -> Result<String, String> {
    let session_key = format!("tool_{tool_name}");
    let session_mb = gcx.read().await.integration_sessions.get(&session_key).cloned();

    if !(args.contains_key("restart") || args.contains_key("stop") || args.contains_key("status"))
        && session_mb.is_some() {
        return Err(format!("cannot execute tool '{tool_name}'. Reason: tool '{tool_name}' is already running.\n"));
    }    
    if session_mb.is_none() && (args.contains_key("restart") || args.contains_key("stop") || args.contains_key("status")) {
        return Err(format!("cannot execute this action on tool '{tool_name}'. Reason: tool '{tool_name}' is not running.\n"));
    }
    
    if args.contains_key("restart") || args.contains_key("stop") || args.contains_key("status") {
        let session = session_mb.clone().unwrap();
        let mut session_lock = session.lock().await;
        let tool_session = session_lock.as_any_mut().downcast_mut::<ToolSession>()
            .ok_or("Failed to downcast tool session".to_string())?;
        
        if args.contains_key("status") {
            return Ok(format!("Tool '{tool_name}' is running.\n"));
        }
        kill_process_and_children(&tool_session.process, tool_name).await
            .map_err(|e| format!("Failed to kill tool '{tool_name}'. Error: {}", e))?;
        drop(session_lock);
        gcx.write().await.integration_sessions.remove(&session_key);

        if args.contains_key("stop") {
            return Ok(format!("Tool '{tool_name}' is stopped.\n"));
        }
    }
    
    let output = start_session(gcx, &session_key, command).await
        .map_err(|e| format!("Failed to start tool '{tool_name}'. Error: {}", e))?;
    
    return Ok(format!("Tool '{tool_name}' is up and running in a background:\n{output}"));
}

#[async_trait]
impl Tool for ToolCustom {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let gcx = ccx.lock().await.global_context.clone();

        let command = replace_magics_from_command(self.command.as_str(), &args, &self.parameters_required)?;

        let resp = if !self.runs_in_background {
            execute_command_with_timeout(&command, Duration::from_secs(self.runs_in_background_false_timeout as u64)).await
        } else {
            execute_background_command(gcx, &self.name, &command, &args).await
        }?;

        let result = vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(resp),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })];

        Ok((false, result))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}
