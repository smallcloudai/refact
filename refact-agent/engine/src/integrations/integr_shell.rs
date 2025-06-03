use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use std::any::Any;
use std::future::Future;
use std::process::Stdio;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tokio::io::BufReader;
use async_trait::async_trait;
use tokio::process::{Command, Child, ChildStdout, ChildStderr};
use md5;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::return_one_candidate_or_a_good_error;
use crate::files_correction::canonical_path;
use crate::files_correction::canonicalize_normalized_path;
use crate::files_correction::check_if_its_inside_a_workspace_or_config;
use crate::files_correction::correct_to_nearest_dir_path;
use crate::files_correction::get_active_project_path;
use crate::files_correction::get_project_dirs;
use crate::files_correction::preprocess_path_for_normalization;
use crate::files_correction::CommandSimplifiedDirExt;
use crate::global_context::GlobalContext;
use crate::integrations::process_io_utils::{execute_command, AnsiStrippable, blocking_read_until_token_or_timeout, is_someone_listening_on_that_tcp_port};
use crate::tools::tools_description::{ToolParam, Tool, ToolDesc, MatchConfirmDeny, MatchConfirmDenyResult};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::postprocessing::pp_command_output::CmdlineOutputFilter;
use crate::integrations::integr_abstract::{IntegrationCommon, IntegrationTrait, IntegrationConfirmation};
use crate::integrations::sessions::{IntegrationSession, get_session_hashmap_key};
use crate::integrations::integr_cmdline::format_output;
use crate::custom_error::YamlError;
use crate::tools::tools_execute::command_should_be_denied;


#[derive(Deserialize, Serialize, Clone, Default)]
pub struct SettingsShell {
    #[serde(default)]
    pub timeout: String,
    #[serde(default)]
    pub output_filter: CmdlineOutputFilter,
    #[serde(default)]
    pub startup_wait_port: Option<u16>,
    #[serde(default = "_default_startup_wait")]
    pub startup_wait: u64,
    #[serde(default)]
    pub startup_wait_keyword: String,
}

fn _default_startup_wait() -> u64 {
    10
}

#[derive(Default)]
pub struct ToolShell {
    pub common: IntegrationCommon,
    pub cfg: SettingsShell,
    pub config_path: String,
}

pub struct ShellSession {
    pub shell_process: Child,
    pub shell_command: String,
    pub shell_workdir: String,
    pub shell_stdout: BufReader<ChildStdout>,
    pub shell_stderr: BufReader<ChildStderr>,
    #[allow(dead_code)]
    pub session_id: String,
}

#[async_trait]
impl IntegrationSession for ShellSession {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn is_expired(&self) -> bool { 
        false 
    }

    fn try_stop(&mut self, self_arc: Arc<AMutex<Box<dyn IntegrationSession>>>) -> Box<dyn Future<Output = String> + Send> {
        Box::new(async move {
            let mut session_locked = self_arc.lock().await;
            let session = session_locked.as_any_mut().downcast_mut::<ShellSession>().unwrap();
            _stop_shell_session(session).await
        })
    }
}

async fn _stop_shell_session(sess: &mut ShellSession) -> String {
    tracing::info!("SHELL BACKGROUND STOP workdir {}:\n{:?}", sess.shell_workdir, sess.shell_command);
    let t0 = tokio::time::Instant::now();
    match sess.shell_process.kill().await {
        Ok(_) => {
            format!("Success, it took {:.3}s to stop the background process.\n\n", t0.elapsed().as_secs_f64())
        },
        Err(e) => {
            tracing::warn!("Failed to kill background shell process. Error: {}. Assuming process died on its own.", e);
            format!("Failed to kill background shell process. Error: {}.\nAssuming process died on its own, let's continue.\n\n", e)
        }
    }
}

#[async_trait]
impl IntegrationTrait for ToolShell {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn integr_schema(&self) -> &str
    {
        SHELL_INTEGRATION_SCHEMA
    }

    async fn integr_settings_apply(&mut self, _gcx: Arc<ARwLock<GlobalContext>>, config_path: String, value: &serde_json::Value) -> Result<(), serde_json::Error> {
        self.cfg = serde_json::from_value(value.clone())?;
        self.common = serde_json::from_value(value.clone())?;
        self.config_path = config_path;
        Ok(())
    }

    fn integr_settings_as_json(&self) -> Value {
        serde_json::to_value(&self.cfg).unwrap()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    async fn integr_tools(&self, _integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        vec![Box::new(ToolShell {
            common: self.common.clone(),
            cfg: self.cfg.clone(),
            config_path: self.config_path.clone(),
        })]
    }
}

#[async_trait]
impl Tool for ToolShell {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let gcx = ccx.lock().await.global_context.clone();
        let (command, workdir_maybe, background, action) = parse_args(gcx.clone(), args).await?;
        let timeout = self.cfg.timeout.parse::<u64>().unwrap_or(10);

        let mut error_log = Vec::<YamlError>::new();
        let env_variables = crate::integrations::setting_up_integrations::get_vars_for_replacements(gcx.clone(), &mut error_log).await;

        let tool_output = if background {
            // Generate a unique session ID based on the command and workdir
            let session_id = format!("{:x}", md5::compute(format!("{}{:?}", command, workdir_maybe)));
            
            // Execute as background process
            execute_background_shell_command(
                &command,
                &workdir_maybe,
                &action.unwrap(), // We validated this is present in parse_args
                &session_id,
                &self.cfg,
                &env_variables,
                gcx.clone(),
            ).await?
        } else {
            // Execute as regular foreground process
            execute_shell_command(
                &command,
                &workdir_maybe,
                timeout,
                &self.cfg.output_filter,
                &env_variables,
                gcx.clone(),
            ).await?
        };

        let result = vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(tool_output),
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
        ToolDesc {
            name: "shell".to_string(),
            agentic: true,
            experimental: false,
            description: "Execute a command, using the \"sh\" on unix-like systems and \"powershell.exe\" on windows. Can run in foreground (default) or background mode. Use it for one-time tasks like dependencies installation or starting background services. Not suitable for regular work because it requires a confirmation at each step.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "command".to_string(),
                    param_type: "string".to_string(),
                    description: "shell command to execute".to_string(),
                },
                ToolParam {
                    name: "workdir".to_string(),
                    param_type: "string".to_string(),
                    description: "workdir for the command".to_string(),
                },
                ToolParam {
                    name: "background".to_string(),
                    param_type: "boolean".to_string(),
                    description: "if true, run the command in the background (for long-running processes like servers)".to_string(),
                },
                ToolParam {
                    name: "action".to_string(),
                    param_type: "string".to_string(),
                    description: "when background=true, specify 'start', 'stop', or 'status' to control the background process".to_string(),
                },
            ],
            parameters_required: vec![
                "command".to_string(),
                "workdir".to_string(),
            ],
        }
    }

    async fn match_against_confirm_deny(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>
    ) -> Result<MatchConfirmDeny, String> {
        let command_to_match = self.command_to_match_against_confirm_deny(ccx.clone(), &args).await.map_err(|e| {
            format!("Error getting tool command to match: {}", e)
        })?;
        if command_to_match.is_empty() {
            return Err("Empty command to match".to_string());
        }
        if let Some(rules) = &self.confirm_deny_rules() {
            let (is_denied, deny_rule) = command_should_be_denied(&command_to_match, &rules.deny);
            if is_denied {
                return Ok(MatchConfirmDeny {
                    result: MatchConfirmDenyResult::DENY,
                    command: command_to_match.clone(),
                    rule: deny_rule.clone(),
                });
            }
        }
        // NOTE: do not match command if not denied, always wait for confirmation from user
        Ok(MatchConfirmDeny {
            result: MatchConfirmDenyResult::CONFIRMATION,
            command: command_to_match.clone(),
            rule: "*".to_string(),
        })
    }

    async fn command_to_match_against_confirm_deny(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        let gcx = ccx.lock().await.global_context.clone();
        let (command, _, _, _) = parse_args(gcx, args).await?;
        Ok(command)
    }

    fn has_config_path(&self) -> Option<String> {
        Some(self.config_path.clone())
    }
    
    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(self.common.confirmation.clone())
    }
}

pub async fn execute_shell_command(
    command: &str,
    workdir_maybe: &Option<PathBuf>,
    timeout: u64,
    output_filter: &CmdlineOutputFilter,
    env_variables: &HashMap<String, String>,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<String, String> {
    let shell = if cfg!(target_os = "windows") { "powershell.exe" } else { "sh" };
    let shell_arg = if cfg!(target_os = "windows") { "-Command" } else { "-c" };
    let mut cmd = Command::new(shell);

    if let Some(workdir) = workdir_maybe {
        cmd.current_dir_simplified(workdir);
    } else if let Some(project_path) = get_active_project_path(gcx.clone()).await {
        cmd.current_dir_simplified(&project_path);
    } else {
        tracing::warn!("no working directory, using whatever directory this binary is run :/");
    }

    for (key, value) in env_variables {
        cmd.env(key, value);
    }

    cmd.arg(shell_arg).arg(command);

    tracing::info!("SHELL: running command directory {:?}\n{:?}", workdir_maybe, command);
    let t0 = tokio::time::Instant::now();
    let output = execute_command(cmd, timeout, command).await?;
    let duration = t0.elapsed();
    tracing::info!("SHELL: /finished in {:.3}s", duration.as_secs_f64());

    let stdout = output.stdout.to_string_lossy_and_strip_ansi();
    let stderr = output.stderr.to_string_lossy_and_strip_ansi();

    let filtered_stdout = crate::postprocessing::pp_command_output::output_mini_postprocessing(output_filter, &stdout);
    let filtered_stderr = crate::postprocessing::pp_command_output::output_mini_postprocessing(output_filter, &stderr);

    let mut out = format_output(&filtered_stdout, &filtered_stderr);
    let exit_code = output.status.code().unwrap_or_default();
    out.push_str(&format!("The command was running {:.3}s, finished with exit code {exit_code}\n", duration.as_secs_f64()));
    Ok(out)
}

const REALLY_HORRIBLE_ROUNDTRIP: u64 = 3000; // 3000ms should be a really bad ping via internet

pub async fn execute_background_shell_command(
    command: &str,
    workdir_maybe: &Option<PathBuf>,
    action: &str,
    session_id: &str,
    cfg: &SettingsShell,
    env_variables: &HashMap<String, String>,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<String, String> {
    let session_key = get_session_hashmap_key("shell", session_id);
    let mut actions_log = String::new();
    let mut session_mb = gcx.read().await.integration_sessions.get(&session_key).cloned();
    
    // Get workdir as string for logging
    let workdir_str = match workdir_maybe {
        Some(path) => path.to_string_lossy().to_string(),
        None => match get_active_project_path(gcx.clone()).await {
            Some(path) => path.to_string_lossy().to_string(),
            None => "<default>".to_string(),
        },
    };

    // Check current status
    if session_mb.is_some() {
        let session_arc = session_mb.clone().unwrap();
        let mut session_locked = session_arc.lock().await;
        let session = session_locked.as_any_mut().downcast_mut::<ShellSession>().unwrap();
        actions_log.push_str(&format!("Currently the background process is running.\nworkdir: {}\ncommand: {}\n\n", 
            session.shell_workdir, session.shell_command));
        
        // Get latest output
        let (stdout_out, stderr_out) = get_stdout_and_stderr(100, &mut session.shell_stdout, &mut session.shell_stderr).await?;
        let filtered_stdout = crate::postprocessing::pp_command_output::output_mini_postprocessing(&cfg.output_filter, &stdout_out);
        let filtered_stderr = crate::postprocessing::pp_command_output::output_mini_postprocessing(&cfg.output_filter, &stderr_out);
        actions_log.push_str(&format!("Here is the latest stdout/stderr from the background process:\n{}\n\n", 
            format_output(&filtered_stdout, &filtered_stderr)));
    } else {
        actions_log.push_str("No background process is currently running with this ID.\n\n");
    }

    // Handle stop action
    if session_mb.is_some() && (action == "stop" || action == "start") {
        let session_arc = session_mb.clone().unwrap();
        {
            let mut session_locked = session_arc.lock().await;
            let mut session = session_locked.as_any_mut().downcast_mut::<ShellSession>().unwrap();
            actions_log.push_str("Stopping the background process...\n");
            let stop_msg = _stop_shell_session(&mut session).await;
            actions_log.push_str(&stop_msg);
        }
        gcx.write().await.integration_sessions.remove(&session_key);
        session_mb = None;
    }

    // Handle start action
    if session_mb.is_none() && action == "start" {
        let mut port_already_open = false;
        if let Some(wait_port) = cfg.startup_wait_port {
            port_already_open = is_someone_listening_on_that_tcp_port(wait_port, tokio::time::Duration::from_millis(REALLY_HORRIBLE_ROUNDTRIP)).await;
            if port_already_open {
                actions_log.push_str(&format!(
                    "This background process requires to wait until TCP port {} is occupied, but this port is already busy. Will try to run anyway.\n\n",
                    wait_port,
                ));
            }
        }
        
        tracing::info!("SHELL BACKGROUND START workdir {:?}:\n{:?}", workdir_maybe, command);
        actions_log.push_str(&format!("Starting background process with command:\n{}\n\n", command));
        
        // Create and configure the command
        let shell = if cfg!(target_os = "windows") { "powershell.exe" } else { "sh" };
        let shell_arg = if cfg!(target_os = "windows") { "-Command" } else { "-c" };
        let mut cmd = Command::new(shell);
        
        if let Some(workdir) = workdir_maybe {
            cmd.current_dir_simplified(workdir);
        } else if let Some(project_path) = get_active_project_path(gcx.clone()).await {
            cmd.current_dir_simplified(&project_path);
        } else {
            tracing::warn!("no working directory, using whatever directory this binary is run :/");
        }
        
        for (key, value) in env_variables {
            cmd.env(key, value);
        }
        
        cmd.arg(shell_arg).arg(command);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        // Spawn the process directly
        let mut process = cmd.spawn().map_err(|e| format!("Failed to create background process: {e}"))?;
        
        let stdout = process.stdout.take().ok_or("Failed to capture stdout")?;
        let stderr = process.stderr.take().ok_or("Failed to capture stderr")?;
        
        let mut stdout_reader = BufReader::new(stdout);
        let mut stderr_reader = BufReader::new(stderr);
        
        let t0 = tokio::time::Instant::now();
        let mut accumulated_stdout = String::new();
        let mut accumulated_stderr = String::new();
        let mut exit_code = -100000; // Special value to indicate process is still running
        
        // Wait for process to start properly
        loop {
            if t0.elapsed() >= tokio::time::Duration::from_secs(cfg.startup_wait) {
                actions_log.push_str(&format!("Timeout {:.2}s reached while waiting for the background process to start.\n\n", 
                    t0.elapsed().as_secs_f64()));
                break;
            }
            
            let (stdout_out, stderr_out) = get_stdout_and_stderr(100, &mut stdout_reader, &mut stderr_reader).await?;
            accumulated_stdout.push_str(&stdout_out);
            accumulated_stderr.push_str(&stderr_out);
            
            // Check for startup keyword
            if !cfg.startup_wait_keyword.is_empty() {
                if accumulated_stdout.contains(&cfg.startup_wait_keyword) || accumulated_stderr.contains(&cfg.startup_wait_keyword) {
                    actions_log.push_str(&format!("Startup keyword '{}' found in output, success!\n\n", cfg.startup_wait_keyword));
                    break;
                }
            }
            
            // Check if process exited prematurely
            match process.try_wait() {
                Ok(Some(status)) => {
                    exit_code = status.code().unwrap_or(-1);
                    actions_log.push_str(&format!("Background process exited prematurely with exit code: {}\nProcess did not start successfully.\n\n", exit_code));
                    break;
                },
                Ok(None) => {}, // Process still running
                Err(e) => {
                    actions_log.push_str(&format!("Error checking process status: {}\n", e));
                    break;
                }
            }
            
            // Check for port if specified
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
        
        // Process output
        let filtered_stdout = crate::postprocessing::pp_command_output::output_mini_postprocessing(&cfg.output_filter, &accumulated_stdout);
        let filtered_stderr = crate::postprocessing::pp_command_output::output_mini_postprocessing(&cfg.output_filter, &accumulated_stderr);
        let out = format_output(&filtered_stdout, &filtered_stderr);
        actions_log.push_str(&out);
        
        // Store session if process is still running
        if exit_code == -100000 {
            let session: Box<dyn IntegrationSession> = Box::new(ShellSession {
                shell_process: process,
                shell_command: command.to_string(),
                shell_workdir: workdir_str,
                shell_stdout: stdout_reader,
                shell_stderr: stderr_reader,
                session_id: session_id.to_string(),
            });
            gcx.write().await.integration_sessions.insert(session_key.to_string(), Arc::new(AMutex::new(session)));
            actions_log.push_str(&format!("Background process started successfully after {:.3}s\n", t0.elapsed().as_secs_f64()));
        }
        
        tracing::info!("SHELL BACKGROUND START LOG:\n{}", actions_log);
    }
    
    Ok(actions_log)
}

async fn get_stdout_and_stderr(
    timeout_ms: u64,
    stdout: &mut BufReader<ChildStdout>,
    stderr: &mut BufReader<ChildStderr>,
) -> Result<(String, String), String> {
    let (stdout_out, stderr_out, _) = blocking_read_until_token_or_timeout(stdout, stderr, timeout_ms, "").await?;
    Ok((stdout_out, stderr_out))
}

async fn parse_args(gcx: Arc<ARwLock<GlobalContext>>, args: &HashMap<String, Value>) -> Result<(String, Option<PathBuf>, bool, Option<String>), String> {
    let command = match args.get("command") {
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err("Command is empty".to_string());
            } else {
                s.clone()
            }
        },
        Some(v) => return Err(format!("argument `command` is not a string: {:?}", v)),
        None => return Err("Missing argument `command`".to_string())
    };

    let workdir = match args.get("workdir") {
        Some(Value::String(s)) => {
            if s.is_empty() {
                None
            } else {
                Some(resolve_shell_workdir(gcx.clone(), s).await?)
            }
        },
        Some(v) => return Err(format!("argument `workdir` is not a string: {:?}", v)),
        None => None
    };
    
    let background = match args.get("background") {
        Some(Value::String(s)) => {
            s.to_lowercase() == "true"
        },
        Some(Value::Bool(b)) => *b,
        Some(v) => return Err(format!("argument `background` is not a boolean or string: {:?}", v)),
        None => false
    };
    
    let action = match args.get("action") {
        Some(Value::String(s)) => {
            if !["start", "stop", "status"].contains(&s.as_str()) {
                return Err("Action must be one of 'start', 'stop', 'status'".to_string());
            }
            Some(s.clone())
        },
        Some(v) => return Err(format!("argument `action` is not a string: {:?}", v)),
        None => None
    };
    
    // If background is true, action is required
    if background && action.is_none() {
        return Err("When running in background mode, 'action' parameter is required".to_string());
    }

    Ok((command, workdir, background, action))
}

async fn resolve_shell_workdir(gcx: Arc<ARwLock<GlobalContext>>, raw_path: &str) -> Result<PathBuf, String> {
    let path_str = preprocess_path_for_normalization(raw_path.to_string());
    let path = PathBuf::from(&path_str);

    let workdir = if path.is_absolute() {
        let path = canonicalize_normalized_path(path);
        check_if_its_inside_a_workspace_or_config(gcx.clone(), &path).await?;
        path
    } else {
        let project_dirs = get_project_dirs(gcx.clone()).await;
        let candidates = correct_to_nearest_dir_path(gcx.clone(), &path_str, false, 3).await;
        canonical_path(
            return_one_candidate_or_a_good_error(gcx.clone(), &path_str, &candidates, &project_dirs, true).await?
        )
    };
    if !workdir.exists() {
        Err("Workdir doesn't exist".to_string())
    } else {
        Ok(workdir)
    }
}

pub const SHELL_INTEGRATION_SCHEMA: &str = r#"
fields:
  timeout:
    f_type: string_short
    f_desc: "The command must immediately return the results, it can't be interactive. If the command runs for too long, it will be terminated and stderr/stdout collected will be presented to the model."
    f_default: "10"
  output_filter:
    f_type: "output_filter"
    f_desc: "The output from the command can be long or even quasi-infinite. This section allows to set limits, prioritize top or bottom, or use regexp to show the model the relevant part."
    f_extra: true
  startup_wait_port:
    f_type: number
    f_desc: "For background processes, wait until this TCP port becomes busy. This is useful for services that bind to a port."
    f_extra: true
  startup_wait:
    f_type: number
    f_desc: "For background processes, maximum time in seconds to wait for the process to start."
    f_default: 10
    f_extra: true
  startup_wait_keyword:
    f_type: string_short
    f_desc: "For background processes, wait until this keyword appears in stdout or stderr."
    f_extra: true
description: |
  Allows to execute any command line tool with confirmation from the chat itself.
  Can run commands in foreground (default) or background mode for long-running processes like servers.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
confirmation:
  ask_user_default: ["*"]
  deny_default: ["sudo*"]
"#;