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
use tokio::time::{timeout, Duration, sleep, Instant};
use std::process::Stdio;
use serde::{Serialize, Deserialize};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{AtParamDict, Tool, ToolDict};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::global_context::GlobalContext;
use crate::integrations::process_io_utils::{kill_process_and_children, read_until_token_or_timeout, wait_until_port_gets_occupied};
use crate::integrations::sessions::IntegrationSession;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomCMDLineTool {
    pub description: String,
    pub parameters: Vec<AtParamDict>,
    pub parameters_required: Vec<String>,
    pub command: String,
    #[serde(default)]
    pub blocking: Option<CustomCMDLineToolBlockingCfg>,
    #[serde(default)]
    pub background: Option<CustomCMDLineToolBackgroundCfg>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomCMDLineToolBlockingCfg {
    pub timeout_s: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomCMDLineToolBackgroundCfg {
    #[serde(default)]
    pub wait_port: Option<u16>,
    #[serde(default)]
    pub wait_keyword: Option<String>,
    pub wait_timeout_s: u64,
}

impl CustomCMDLineTool {
    pub fn into_tool_dict(&self, name: String) -> ToolDict {
        ToolDict {
            name,
            agentic: true,
            experimental: false,
            description: self.description.clone(),
            parameters: self.parameters.clone(),
            parameters_required: self.parameters_required.clone(),
        }
    }
}

pub struct ToolCustom {
    pub name: String,
    #[allow(dead_code)]
    pub parameters: Vec<AtParamDict>,
    pub parameters_required: Vec<String>,
    pub command: String,
    pub blocking: Option<CustomCMDLineToolBlockingCfg>,
    pub background: Option<CustomCMDLineToolBackgroundCfg>,
}

pub struct ToolSession {
    process: Child,
    command: String,
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
    args_str: &HashMap<String, String>,
    required_params: &Vec<String>
) -> Result<String, String> {
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
    cfg: CustomCMDLineToolBlockingCfg,
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

    let timeout_duration = Duration::from_secs(cfg.timeout_s);
    timeout(timeout_duration, command_future).await
        .unwrap_or_else(|_| Err("Command execution timed out".to_string()))
}

async fn get_stdout_and_stderr(
    timeout_ms: u64,
    stdout: &mut BufReader<ChildStdout>,
    stderr: &mut BufReader<ChildStderr>,
    token: Option<String>,
) -> Result<(String, String), String> {
    let token = token.unwrap_or_default();
    let stdout_out = read_until_token_or_timeout(stdout, timeout_ms, token.as_str()).await?;
    let stderr_out = read_until_token_or_timeout(stderr, timeout_ms, token.as_str()).await?;
    Ok((stdout_out, stderr_out))
}

async fn read_until_text_in_output_or_timeout(
    timeout: Duration,
    stdout: &mut BufReader<ChildStdout>,
    stderr: &mut BufReader<ChildStderr>,
    text: &str,
) -> Result<(String, String), String> {

    let start = Instant::now();
    let step_duration = Duration::from_millis(100);
    let mut stdout_text = String::new();
    let mut stderr_text = String::new();

    while start.elapsed() < timeout {
        let stdout_out = read_until_token_or_timeout(stdout, step_duration.as_millis() as u64, text).await?;
        let stderr_out = read_until_token_or_timeout(stderr, step_duration.as_millis() as u64, text).await?;
        stdout_text.push_str(&stdout_out);
        stderr_text.push_str(&stderr_out);

        if !text.is_empty() && format!("{}{}", stdout_text, stderr_text).contains(text) {
            return Ok((stdout_text, stderr_text));
        }

        sleep(step_duration).await;
    }
    Err(format!("Timeout reached. Output:\nSTDOUT:{}\nSTDERR:\n{}", stdout_text, stderr_text))
}

async fn start_session(
    gcx: Arc<ARwLock<GlobalContext>>,
    session_key: &str,
    command: String,
    cfg: &CustomCMDLineToolBackgroundCfg,
) -> Result<String, String> {
    info!("EXEC: {command}");
    let mut process = Command::new("sh")
       .arg("-c")
       .arg(command.clone())
       .stdout(Stdio::piped())
       .stderr(Stdio::piped())
       .spawn()
       .map_err(|e| format!("failed to create process: {e}"))?;

    let mut stdout = BufReader::new(process.stdout.take().ok_or("Failed to open stdout")?);
    let mut stderr = BufReader::new(process.stderr.take().ok_or("Failed to open stderr")?);

    let wait_timeout = Duration::from_secs(cfg.wait_timeout_s);

    let stdout_out: String;
    let stderr_out: String;

    // todo: does not work for npm run somewhy
    if let Some(wait_port) = cfg.wait_port {
        let resp = wait_until_port_gets_occupied(wait_port, &wait_timeout).await;
        (stdout_out, stderr_out) = get_stdout_and_stderr(100, &mut stdout, &mut stderr, None).await?;
        resp?;
    } else {
        (stdout_out, stderr_out) = read_until_text_in_output_or_timeout(wait_timeout, &mut stdout, &mut stderr, cfg.wait_keyword.clone().unwrap_or_default().as_str()).await?;
    }

    let mut out = String::new();
    if !stdout_out.is_empty() {
        out.push_str(&format!("STDOUT:\n{stdout_out}"));
    }
    if !stderr_out.is_empty() {
        out.push_str(&format!("STDERR:\n{stderr_out}"));
    }

    let exit_status = process.try_wait().map_err(|e| e.to_string())?;
    if exit_status.is_some() {
        let status = exit_status.unwrap().code().unwrap();
        warn!("tool process exited with status: {:?}. Output:\n{out}", status);
        return Err(format!("tool process exited with status: {:?}; Output:\n{out}", status));
    }

    let session: Box<dyn IntegrationSession> = Box::new(ToolSession {
        process,
        command,
        stdout,
        stderr
    });
    gcx.write().await.integration_sessions.insert(session_key.to_string(), Arc::new(AMutex::new(session)));

    Ok(out)
}

async fn execute_background_command(
    gcx: Arc<ARwLock<GlobalContext>>,
    tool_name: &str,
    command: &str,
    cfg: CustomCMDLineToolBackgroundCfg,
    action: &str,
) -> Result<String, String> {
    let session_key = format!("custom_service_{tool_name}");
    let session_mb = gcx.read().await.integration_sessions.get(&session_key).cloned();
    let mut command = command.to_string();

    if !(action == "restart" || action == "stop" || action == "status")
        && session_mb.is_some() {
        return Err(format!("cannot execute tool '{tool_name}'. Reason: tool '{tool_name}' is already running.\n"));
    }
    if session_mb.is_none() && (action == "restart" || action == "stop" || action == "status") {
        return Err(format!("cannot execute this action on tool '{tool_name}'. Reason: tool '{tool_name}' is not running.\n"));
    }

    if action == "restart" || action == "stop" || action == "status" {
        let session = session_mb.clone().unwrap();
        let mut session_lock = session.lock().await;
        let tool_session = session_lock.as_any_mut().downcast_mut::<ToolSession>()
            .ok_or("Failed to downcast tool session".to_string())?;

        if action == "status" {
            return Ok(format!("Tool '{tool_name}' is running.\n"));
        }
        kill_process_and_children(&tool_session.process, tool_name).await
            .map_err(|e| format!("Failed to kill tool '{tool_name}'. Error: {}", e))?;
        command = tool_session.command.clone();
        drop(session_lock);
        gcx.write().await.integration_sessions.remove(&session_key);

        if action == "stop" {
            return Ok(format!("Tool '{tool_name}' is stopped.\n"));
        }
    }

    let output = start_session(gcx, &session_key, command, &cfg).await
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

        let args_str: HashMap<String, String> = args.iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect();

        let command = replace_magics_from_command(self.command.as_str(), &args_str, &self.parameters_required)?;

        let resp = if self.blocking.is_some() {
            let blocking_cfg = self.blocking.clone().unwrap();
            execute_command_with_timeout(&command, blocking_cfg).await
        } else if self.background.is_some() {
            let action = args_str.get("action").cloned().unwrap_or("start".to_string());
            if !["start", "restart", "stop", "status"].contains(&action.as_str()) {
                return Err("Too call is invalid. Param 'action' must be one for 'start','restart','stop','status'. Try again".to_string());
            }
            let background_cfg = self.background.clone().unwrap();
            execute_background_command(gcx, &self.name, &command, background_cfg, action.as_str()).await
        } else {
            Err(format!("Custom tool '{}' is invalid. It must have one of 'blocking' or 'background' param.", self.name))
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


// #[serde(default)]
// pub custom_cmdline_tools: IndexMap<String, CustomCMDLineTool>,
