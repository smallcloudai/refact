use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::process::Stdio;
use indexmap::IndexMap;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tokio::io::BufReader;
use serde::Deserialize;
use async_trait::async_trait;
use tokio::process::Command;
use tracing::info;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{ToolParam, Tool, ToolDesc};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::global_context::GlobalContext;
use crate::integrations::process_io_utils::{kill_process_and_children, blocking_read_until_token_or_timeout, is_someone_listening_on_that_tcp_port};
use crate::integrations::sessions::IntegrationSession;
use crate::postprocessing::pp_command_output::{CmdlineOutputFilter, output_mini_postprocessing};


const REALLY_HORRIBLE_ROUNDTRIP: u64 = 3000;   // 3000 should be a really bad ping via internet, just in rare case it's a remote port


#[derive(Deserialize)]
struct CmdlineToolConfig {
    description: String,
    parameters: Vec<ToolParam>,
    parameters_required: Option<Vec<String>>,
    command: String,
    command_workdir: String,

    // blocking
    #[serde(default = "_default_timeout")]
    timeout: u64,
    #[serde(default)]
    output_filter: CmdlineOutputFilter,

    // background
    #[serde(default)]
    startup_wait_port: Option<u16>,
    #[serde(default = "_default_startup_wait")]
    startup_wait: u64,
    #[serde(default)]
    startup_wait_keyword: Option<String>,
}

fn _default_timeout() -> u64 {
    120
}

fn _default_startup_wait() -> u64 {
    10
}

pub struct ToolCmdline {
    a_service: bool,
    name: String,
    cfg: CmdlineToolConfig,
}

pub fn cmdline_tool_from_yaml_value(
    cfg_cmdline_value: &serde_yaml::Value,
    background: bool,
) -> Result<IndexMap<String, Arc<AMutex<Box<dyn Tool + Send>>>>, String> {
    let mut result = IndexMap::new();
    let cfgmap = match serde_yaml::from_value::<IndexMap<String, CmdlineToolConfig>>(cfg_cmdline_value.clone()) {
        Ok(cfgmap) => cfgmap,
        Err(e) => {
            let location = e.location().map(|loc| format!(" at line {}, column {}", loc.line(), loc.column())).unwrap_or_default();
            return Err(format!("failed to parse cmdline section: {:?}{}", e, location));
        }
    };
    for (c_name, mut c_cmd_tool) in cfgmap.into_iter() {
        if background {
            c_cmd_tool.parameters.push(ToolParam {
                name: "action".to_string(),
                param_type: "string".to_string(),
                description: "start | stop | restart | status".to_string(),
            });
        }
        let tool = Arc::new(AMutex::new(Box::new(
            ToolCmdline {
                a_service: background,
                name: c_name.clone(),
                cfg: c_cmd_tool,
            }
        ) as Box<dyn Tool + Send>));
        result.insert(c_name, tool);
    }
    Ok(result)
}

pub struct CmdlineSession {
    cmdline_string: String,
    cmdline_workdir: String,
    cmdline_process: tokio::process::Child,
    #[allow(dead_code)]
    cmdline_stdout: BufReader<tokio::process::ChildStdout>,
    #[allow(dead_code)]
    cmdline_stderr: BufReader<tokio::process::ChildStderr>,
}

impl IntegrationSession for CmdlineSession {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn is_expired(&self) -> bool { false }
}

fn _replace_args(x: &str, args_str: &HashMap<String, String>) -> String {
    let mut result = x.to_string();
    for (key, value) in args_str {
        result = result.replace(&format!("%{}%", key), value);
    }
    result
}

fn format_output(stdout_out: &str, stderr_out: &str) -> String {
    let mut out = String::new();
    if !stdout_out.is_empty() && stderr_out.is_empty() {
        // special case: just clean output, nice
        out.push_str(&format!("{}\n", stdout_out));
    } else {
        if !stdout_out.is_empty() {
            out.push_str(&format!("STDOUT\n```\n{}```\n\n", stdout_out));
        }
        if !stderr_out.is_empty() {
            out.push_str(&format!("STDERR\n```\n{}```\n\n", stderr_out));
        }
    }
    out
}

async fn create_command_from_string(
    cmd_string: &str,
    command_workdir: &String,
) -> Result<Command, String> {
    let command_args = shell_words::split(cmd_string)
        .map_err(|e| format!("Failed to parse command: {}", e))?;
    if command_args.is_empty() {
        return Err("Command is empty after parsing".to_string());
    }
    let mut cmd = Command::new(&command_args[0]);
    if command_args.len() > 1 {
        cmd.args(&command_args[1..]);
    }
    cmd.current_dir(command_workdir);
    Ok(cmd)
}

async fn execute_blocking_command(
    command: &str,
    cfg: &CmdlineToolConfig,
    command_workdir: &String,
) -> Result<String, String> {
    info!("EXEC workdir {}:\n{:?}", command_workdir, command);
    let command_future = async {
        let mut cmd = create_command_from_string(command, command_workdir).await?;
        let t0 = tokio::time::Instant::now();
        let result = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;
        let duration = t0.elapsed();
        info!("EXEC: /finished in {:?}", duration);

        let output = match result {
            Ok(output) => output,
            Err(e) => {
                let msg = format!("cannot run command: '{}'. workdir: '{}'. Error: {}", &command, command_workdir, e);
                tracing::error!("{msg}");
                return Err(msg);
            }
        };

        let stdout = output_mini_postprocessing(&cfg.output_filter, &String::from_utf8_lossy(&output.stdout).to_string());
        let stderr = output_mini_postprocessing(&cfg.output_filter, &String::from_utf8_lossy(&output.stderr).to_string());

        let mut out = format_output(&stdout, &stderr);
        let exit_code = output.status.code().unwrap_or_default();
        out.push_str(&format!("command was running {:.3}s, finished with exit code {exit_code}\n", duration.as_secs_f64()));
        Ok(out)
    };

    let timeout_duration = tokio::time::Duration::from_secs(cfg.timeout);
    let result = tokio::time::timeout(timeout_duration, command_future).await;

    match result {
        Ok(res) => res,
        Err(_) => Err(format!("command timed out after {:?}", timeout_duration)),
    }
}

async fn get_stdout_and_stderr(
    timeout_ms: u64,
    stdout: &mut BufReader<tokio::process::ChildStdout>,
    stderr: &mut BufReader<tokio::process::ChildStderr>,
) -> Result<(String, String), String> {
    let (stdout_out, stderr_out, _) = blocking_read_until_token_or_timeout(stdout, stderr, timeout_ms, "").await?;
    Ok((stdout_out, stderr_out))
}

async fn execute_background_command(
    gcx: Arc<ARwLock<GlobalContext>>,
    service_name: &str,
    command_str: &str,
    cmdline_workdir: &String,
    cfg: &CmdlineToolConfig,
    action: &str,
) -> Result<String, String> {
    let session_key = format!("custom_service_{service_name}");
    let mut session_mb = gcx.read().await.integration_sessions.get(&session_key).cloned();
    let command_str = command_str.to_string();
    let mut actions_log = String::new();

    if session_mb.is_some() {
        let session_arc = session_mb.clone().unwrap();
        let mut session_locked = session_arc.lock().await;
        let session = session_locked.as_any_mut().downcast_mut::<CmdlineSession>().unwrap();
        actions_log.push_str(&format!("Currently have service running, workdir {}:\n{}\n", session.cmdline_workdir, session.cmdline_string));
        let (stdout_out, stderr_out) = get_stdout_and_stderr(100, &mut session.cmdline_stdout, &mut session.cmdline_stderr).await?;
        let filtered_stdout = output_mini_postprocessing(&cfg.output_filter, &stdout_out);
        let filtered_stderr = output_mini_postprocessing(&cfg.output_filter, &stderr_out);
        actions_log.push_str(&format!("Here are stdin/stderr since the last checking out on the service:\n{}\n\n", format_output(&filtered_stdout, &filtered_stderr)));
    } else {
        actions_log.push_str(&format!("Service is currently not running\n"));
    }

    if session_mb.is_some() && (action == "restart" || action == "stop") {
        let session_arc = session_mb.clone().unwrap();
        {
            let mut session_locked = session_arc.lock().await;
            let session = session_locked.as_any_mut().downcast_mut::<CmdlineSession>().unwrap();
            actions_log.push_str(&format!("Stopping it...\n"));
            let t0 = tokio::time::Instant::now();
            tracing::info!("SERVICE STOP workdir {}:\n{:?}", session.cmdline_workdir, session.cmdline_string);
            match kill_process_and_children(&session.cmdline_process, service_name).await {
                Ok(_) => {
                    actions_log.push_str(&format!("Success, it took {:.3}s to stop it.\n\n", t0.elapsed().as_secs_f64()));
                },
                Err(e) => {
                    tracing::warn!("Failed to kill service '{}'. Error: {}. Assuming process died on its own.", service_name, e);
                    actions_log.push_str(&format!("Failed to kill service. Error: {}.\nAssuming process died on its own, let's continue.\n\n", e));
                }
            }
        }
        gcx.write().await.integration_sessions.remove(&session_key);
        session_mb = None;
    }

    if session_mb.is_none() && (action == "restart" || action == "start") {
        let mut port_already_open = false;
        if let Some(wait_port) = cfg.startup_wait_port {
            port_already_open = is_someone_listening_on_that_tcp_port(wait_port, tokio::time::Duration::from_millis(REALLY_HORRIBLE_ROUNDTRIP)).await;
            if port_already_open {
                actions_log.push_str(&format!(
                    "This service startup sequence requires to wait until a TCP port gets occupied, but this port {} is already busy even before the service start is attempted. Not good, but let's try to run it anyway.\n\n",
                    wait_port,
                ));
            }
        }
        tracing::info!("SERVICE START workdir {}:\n{:?}", cmdline_workdir, command_str);
        actions_log.push_str(&format!("Starting service with the following command line:\n{}\n", command_str));

        let mut command = create_command_from_string(&command_str, cmdline_workdir).await?;
        let mut process = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to create process: {e}"))?;

        let mut stdout_reader = BufReader::new(process.stdout.take().ok_or("Failed to open stdout")?);
        let mut stderr_reader = BufReader::new(process.stderr.take().ok_or("Failed to open stderr")?);

        let t0 = tokio::time::Instant::now();

        let mut accumulated_stdout = String::new();
        let mut accumulated_stderr = String::new();
        let mut exit_code: i32 = -100000;

        loop {
            if t0.elapsed() >= tokio::time::Duration::from_secs(cfg.startup_wait) {
                actions_log.push_str(&format!("Timeout {:.2}s reached while waiting for the service to start.\n\n", t0.elapsed().as_secs_f64()));
                break;
            }

            let (stdout_out, stderr_out) = get_stdout_and_stderr(100, &mut stdout_reader, &mut stderr_reader).await?;
            accumulated_stdout.push_str(&stdout_out);
            accumulated_stderr.push_str(&stderr_out);

            // XXX rename keyword to phrase or something
            if let Some(keyword) = &cfg.startup_wait_keyword {
                if accumulated_stdout.contains(keyword) || accumulated_stderr.contains(keyword) {
                    actions_log.push_str(&format!("Startup keyword '{}' found in output, success!\n\n", keyword));
                    break;
                }
            }

            let exit_status = process.try_wait().map_err(|e| e.to_string())?;
            if let Some(status) = exit_status {
                exit_code = status.code().unwrap_or(-1);
                actions_log.push_str(&format!("Service process exited prematurely with exit code: {}\nService did not start.\n\n", exit_code));
                break;
            }

            if let Some(wait_port) = cfg.startup_wait_port {
                match is_someone_listening_on_that_tcp_port(wait_port, tokio::time::Duration::from_millis(REALLY_HORRIBLE_ROUNDTRIP)).await {
                    true => {
                        if !port_already_open {
                            actions_log.push_str(&format!("Port {} is now busy, success!\n", wait_port));
                            break;
                        }
                    },
                    false => {
                        if port_already_open {
                            port_already_open = false;
                            actions_log.push_str(&format!("Port {} is now free\n", wait_port));
                        }
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        let filtered_stdout = output_mini_postprocessing(&cfg.output_filter, &accumulated_stdout);
        let filtered_stderr = output_mini_postprocessing(&cfg.output_filter, &accumulated_stderr);
        let out = format_output(&filtered_stdout, &filtered_stderr);
        actions_log.push_str(&out);

        if exit_code == -100000 {
            let session: Box<dyn IntegrationSession> = Box::new(CmdlineSession {
                cmdline_process: process,
                cmdline_string: command_str,
                cmdline_workdir: cmdline_workdir.clone(),
                cmdline_stdout: stdout_reader,
                cmdline_stderr: stderr_reader,
            });
            gcx.write().await.integration_sessions.insert(session_key.to_string(), Arc::new(AMutex::new(session)));
        }

        tracing::info!("SERVICE START LOG:\n{}", actions_log);
    }

    Ok(actions_log)
}

#[async_trait]
impl Tool for ToolCmdline {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let gcx = ccx.lock().await.global_context.clone();

        let mut args_str: HashMap<String, String> = HashMap::new();
        let valid_params: Vec<String> = self.cfg.parameters.iter().map(|p| p.name.clone()).collect();

        for (k, v) in args.iter() {
            if !valid_params.contains(k) {
                return Err(format!("Unexpected argument `{}`", k));
            }
            match v {
                serde_json::Value::String(s) => { args_str.insert(k.clone(), s.clone()); },
                _ => return Err(format!("argument `{}` is not a string: {:?}", k, v)),
            }
        }

        for param in &self.cfg.parameters {
            if self.cfg.parameters_required.as_ref().map_or(false, |req| req.contains(&param.name)) && !args_str.contains_key(&param.name) {
                return Err(format!("Missing required argument `{}`", param.name));
            }
        }

        let command = _replace_args(self.cfg.command.as_str(), &args_str);
        let workdir = _replace_args(self.cfg.command_workdir.as_str(), &args_str);

        let tool_ouput = if self.a_service {
            let action = args_str.get("action").cloned().unwrap_or("start".to_string());
            if !["start", "restart", "stop", "status"].contains(&action.as_str()) {
                return Err("Tool call is invalid. Param 'action' must be one of 'start', 'restart', 'stop', 'status'. Try again".to_string());
            }
            execute_background_command(
                gcx, &self.name, &command, &workdir, &self.cfg, action.as_str()
            ).await?

        } else {
            execute_blocking_command(&command, &self.cfg, &workdir).await?
        };

        let result = vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(tool_ouput),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })];

        Ok((false, result))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }

    fn tool_description(&self) -> ToolDesc {
        let parameters_required = self.cfg.parameters_required.clone().unwrap_or_else(|| {
            self.cfg.parameters.iter().map(|param| param.name.clone()).collect()
        });
        ToolDesc {
            name: self.name.clone(),
            agentic: true,
            experimental: false,
            description: self.cfg.description.clone(),
            parameters: self.cfg.parameters.clone(),
            parameters_required,
        }
    }
}
