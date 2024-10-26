use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;
use tokio::io::BufReader;
use async_trait::async_trait;
use regex::Regex;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tracing::{info, warn};
use tokio::time::{timeout, Duration, sleep, Instant};
use std::process::Stdio;
use serde::{Deserialize};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{AtParamDict, Tool, ToolDict};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::global_context::GlobalContext;
use crate::integrations::process_io_utils::{kill_process_and_children, read_until_token_or_timeout, wait_until_port_gets_occupied};
use crate::integrations::sessions::IntegrationSession;


#[derive(Deserialize, Clone)]
pub struct CmdlineToolBlocking {
    pub timeout_s: u64,
}

#[derive(Deserialize, Clone)]
pub struct CmdlineToolBackground {
    #[serde(default)]
    pub wait_port: Option<u16>,
    #[serde(default)]
    pub wait_keyword: Option<String>,
    #[serde(default)]
    pub wait_timeout_s: u64,
}

#[derive(Deserialize)]
pub struct CmdlineToolConfig {
    #[serde(alias="description")]
    pub cfg_description: String,
    #[serde(default, alias="parameters")]
    pub cfg_parameters: Vec<AtParamDict>,
    #[serde(default, alias="parameters_required")]
    pub cfg_parameters_required: Option<Vec<String>>,
    #[serde(default, alias="command")]
    pub cfg_command: String,
    #[serde(alias="workdir")]
    pub cfg_command_workdir: String,
    #[serde(default, alias="blocking")]
    pub blocking: Option<CmdlineToolBlocking>,
    #[serde(default, alias="background")]
    pub background: Option<CmdlineToolBackground>,
}

pub struct ToolCmdline {
    pub name: String,
    pub cfg: CmdlineToolConfig,
}


impl ToolCmdline {
    pub fn into_tool_dict(&self, name: String) -> ToolDict {
        let req = self.cfg.cfg_parameters_required.clone().unwrap_or_else(|| {
            self.cfg.cfg_parameters.iter().map(|param| param.name.clone()).collect()
        });
        ToolDict {
            name,
            agentic: true,
            experimental: false,
            description: self.cfg.cfg_description.clone(),
            parameters: self.cfg.cfg_parameters.clone(),
            parameters_required: req,
        }
    }
}

pub struct CmdlineSession {
    cmdline_string: String,
    cmdline_process: tokio::process::Child,
    // #[allow(dead_code)]
    cmdline_stdout: BufReader<tokio::process::ChildStdout>,
    // #[allow(dead_code)]
    cmdline_stderr: BufReader<tokio::process::ChildStderr>,
}

impl IntegrationSession for CmdlineSession {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn is_expired(&self) -> bool { false }
}

fn replace_magics_inside_command(
    command: &str,
    args_str: &HashMap<String, String>,
) -> Result<String, String> {
    let mut command_clone = command.to_string();
    for (key, value) in args_str {
        let pattern = format!("%{}%", key);
        let re = Regex::new(&regex::escape(&pattern)).unwrap();
        command_clone = re.replace_all(&command_clone, value.as_str()).to_string();
    }
    Ok(command_clone)
}

async fn execute_blocking_command(
    command: &str,
    cfg: CmdlineToolBlocking,
    cfg_command_workdir: &String,
) -> Result<String, String> {
    info!("EXEC: {command}");
    let command_args = shell_words::split(command)
        .map_err(|e| format!("Failed to parse command: {}", e))?;
    if command_args.is_empty() {
        return Err("Command is empty after parsing".to_string());
    }
    let command_future = async {
        let mut cmd = tokio::process::Command::new(&command_args[0]);
        if command_args.len() > 1 {
            cmd.args(&command_args[1..]);
        }
        cmd.current_dir(cfg_command_workdir);
        let output = cmd
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
    timeout(
        timeout_duration,
        command_future
    ).await.unwrap_or_else(|_| Err("Command execution timed out".to_string()))
}

async fn get_stdout_and_stderr(
    timeout_ms: u64,
    stdout: &mut BufReader<tokio::process::ChildStdout>,
    stderr: &mut BufReader<tokio::process::ChildStderr>,
    token: Option<String>,
) -> Result<(String, String), String> {
    let token = token.unwrap_or_default();
    let stdout_out = read_until_token_or_timeout(stdout, timeout_ms, token.as_str()).await?;
    let stderr_out = read_until_token_or_timeout(stderr, timeout_ms, token.as_str()).await?;
    Ok((stdout_out, stderr_out))
}

async fn read_until_text_in_output_or_timeout(
    timeout: Duration,
    stdout: &mut BufReader<tokio::process::ChildStdout>,
    stderr: &mut BufReader<tokio::process::ChildStderr>,
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

async fn execute_background_command(
    gcx: Arc<ARwLock<GlobalContext>>,
    service_name: &str,
    command: &str,
    bg_cfg: CmdlineToolBackground,
    action: &str,
) -> Result<String, String> {
    let session_key = format!("custom_service_{service_name}");
    let session_mb = gcx.read().await.integration_sessions.get(&session_key).cloned();
    let mut command = command.to_string();

    // XXX: this needs re-thinking

    // if session_mb.is_some() && action == "start" {
    //     return Ok(format!("the service '{service_name}' is running"));
    // }

    // if session_mb.is_none() && action == "status" {
    //     return Err(format!("cannot execute this action on service '{service_name}'. Reason: service '{service_name}' is not running.\n"));
    // }

    if action == "restart" || action == "stop" || action == "status" {
        let session = session_mb.clone().unwrap();
        let mut session_lock = session.lock().await;
        let session = session_lock.as_any_mut().downcast_mut::<CmdlineSession>()
            .ok_or("Failed to downcast CmdlineSession".to_string())?;

        if action == "status" {
            return Ok(format!("service '{service_name}' is running.\n"));
        }
        kill_process_and_children(&session.cmdline_process, service_name).await
            .map_err(|e| format!("Failed to kill service '{service_name}'. Error: {}", e))?;
        command = session.cmdline_string.clone();
        drop(session_lock);
        gcx.write().await.integration_sessions.remove(&session_key);

        if action == "stop" {
            return Ok(format!("service '{service_name}' is stopped.\n"));
        }
    }

    let output = {
        info!("EXEC: {command}");
        let mut process = tokio::process::Command::new("sh")
           .arg("-c")
           .arg(command.clone())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped())
           .spawn()
           .map_err(|e| format!("failed to create process: {e}"))?;

        let mut stdout = BufReader::new(process.stdout.take().ok_or("Failed to open stdout")?);
        let mut stderr = BufReader::new(process.stderr.take().ok_or("Failed to open stderr")?);

        let wait_timeout = Duration::from_secs(bg_cfg.wait_timeout_s);

        let stdout_out: String;
        let stderr_out: String;

        // todo: does not work for npm run
        if let Some(wait_port) = bg_cfg.wait_port {
            let resp = wait_until_port_gets_occupied(wait_port, &wait_timeout).await;
            (stdout_out, stderr_out) = get_stdout_and_stderr(100, &mut stdout, &mut stderr, None).await?;
            resp?;
        } else {
            (stdout_out, stderr_out) = read_until_text_in_output_or_timeout(wait_timeout, &mut stdout, &mut stderr, bg_cfg.wait_keyword.clone().unwrap_or_default().as_str()).await?;
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
            warn!("service process exited with status: {:?}. Output:\n{out}", status);
            return Err(format!("service process exited with status: {:?}; Output:\n{out}", status));
        }

        let session: Box<dyn IntegrationSession> = Box::new(CmdlineSession {
            cmdline_process: process,
            cmdline_string: command,
            cmdline_stdout: stdout,
            cmdline_stderr: stderr,
        });
        gcx.write().await.integration_sessions.insert(session_key.to_string(), Arc::new(AMutex::new(session)));

        out
    };

    return Ok(format!("service '{service_name}' is up and running in a background:\n{output}"));
}

#[async_trait]
impl Tool for ToolCmdline {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let gcx = ccx.lock().await.global_context.clone();

        let mut args_str: HashMap<String, String> = HashMap::new();
        let valid_params: Vec<String> = self.cfg.cfg_parameters.iter().map(|p| p.name.clone()).collect();

        for (k, v) in args.iter() {
            if !valid_params.contains(k) {
                return Err(format!("Unexpected argument `{}`", k));
            }
            match v {
                Value::String(s) => { args_str.insert(k.clone(), s.clone()); },
                _ => return Err(format!("argument `{}` is not a string: {:?}", k, v)),
            }
        }

        for param in &self.cfg.cfg_parameters {
            if self.cfg.cfg_parameters_required.as_ref().map_or(false, |req| req.contains(&param.name)) && !args_str.contains_key(&param.name) {
                return Err(format!("Missing required argument `{}`", param.name));
            }
        }

        let command = replace_magics_inside_command(
            self.cfg.cfg_command.as_str(),
            &args_str,
        )?;

        let resp = if let Some(blocking_cfg) = &self.cfg.blocking {
            execute_blocking_command(&command, blocking_cfg.clone(), &self.cfg.cfg_command_workdir).await

        } else if let Some(background_cfg) = &self.cfg.background {
            let action = args_str.get("action").cloned().unwrap_or("start".to_string());
            if !["start", "restart", "stop", "status"].contains(&action.as_str()) {
                return Err("Tool call is invalid. Param 'action' must be one of 'start', 'restart', 'stop', 'status'. Try again".to_string());
            }
            execute_background_command(gcx, &self.name, &command, background_cfg.clone(), action.as_str()).await

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
