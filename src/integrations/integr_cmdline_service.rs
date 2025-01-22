use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::process::Stdio;
use tokio::io::BufReader;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use async_trait::async_trait;
use process_wrap::tokio::*;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolParam, ToolDesc};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::global_context::GlobalContext;
use crate::postprocessing::pp_command_output::output_mini_postprocessing;
use crate::integrations::process_io_utils::{blocking_read_until_token_or_timeout, is_someone_listening_on_that_tcp_port};
use crate::integrations::sessions::IntegrationSession;
use crate::integrations::integr_abstract::{IntegrationTrait, IntegrationCommon, IntegrationConfirmation};
use crate::integrations::integr_cmdline::*;
use crate::integrations::setting_up_integrations::YamlError;


const REALLY_HORRIBLE_ROUNDTRIP: u64 = 3000;   // 3000 should be a really bad ping via internet, just in rare case it's a remote port

#[derive(Default)]
pub struct ToolService {
    pub common:  IntegrationCommon,
    pub name: String,
    pub cfg: CmdlineToolConfig,
    pub config_path: String,
}

#[async_trait]
impl IntegrationTrait for ToolService {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn integr_settings_apply(&mut self, _gcx: Arc<ARwLock<GlobalContext>>, config_path: String, value: &serde_json::Value) -> Result<(), String> {
        match serde_json::from_value::<CmdlineToolConfig>(value.clone()) {
            Ok(x) => self.cfg = x,
            Err(e) => {
                tracing::error!("Failed to apply settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        match serde_json::from_value::<IntegrationCommon>(value.clone()) {
            Ok(x) => self.common = x,
            Err(e) => {
                tracing::error!("Failed to apply common settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        self.config_path = config_path;
        Ok(())
    }

    fn integr_settings_as_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    async fn integr_tools(&self, integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        vec![Box::new(ToolService {
            common: self.common.clone(),
            name: integr_name.to_string(),
            cfg: self.cfg.clone(),
            config_path: self.config_path.clone(),
        })]
    }

    fn integr_schema(&self) -> &str
    {
        CMDLINE_SERVICE_INTEGRATION_SCHEMA
    }
}

pub struct CmdlineSession {
    cmdline_string: String,
    cmdline_workdir: String,
    cmdline_process: Box<dyn TokioChildWrapper>,
    cmdline_stdout: BufReader<tokio::process::ChildStdout>,
    cmdline_stderr: BufReader<tokio::process::ChildStderr>,
    service_name: String,
}

impl IntegrationSession for CmdlineSession {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn is_expired(&self) -> bool { false }

    fn try_stop(&mut self) -> Box<dyn Future<Output = String> + Send + '_> {
        Box::new(async {
            tracing::info!("SERVICE STOP workdir {}:\n{:?}", self.cmdline_workdir, self.cmdline_string);
            let t0 = tokio::time::Instant::now();
            match Box::into_pin(self.cmdline_process.kill()).await {
                Ok(_) => {
                    format!("Success, it took {:.3}s to stop it.\n\n", t0.elapsed().as_secs_f64())
                },
                Err(e) => {
                    tracing::warn!("Failed to kill service '{}'. Error: {}. Assuming process died on its own.", self.service_name, e);
                    format!("Failed to kill service. Error: {}.\nAssuming process died on its own, let's continue.\n\n", e)
                }
            }
        })
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
    env_variables: &HashMap<String, String>,
) -> Result<String, String> {
    let session_key = format!("custom_service_{service_name}");
    let mut session_mb = gcx.read().await.integration_sessions.get(&session_key).cloned();
    let command_str = command_str.to_string();
    let mut actions_log = String::new();

    if session_mb.is_some() {
        let session_arc = session_mb.clone().unwrap();
        let mut session_locked = session_arc.lock().await;
        let session = session_locked.as_any_mut().downcast_mut::<CmdlineSession>().unwrap();
        actions_log.push_str(&format!("Currently the service is running.\nworkdir: {}\ncommand line: {}\n\n", session.cmdline_workdir, session.cmdline_string));
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
            let stop_log = Box::into_pin(session.try_stop()).await;
            actions_log.push_str(&stop_log);
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
        let project_dirs = crate::files_correction::get_project_dirs(gcx.clone()).await;

        let mut command = create_command_from_string(&command_str, cmdline_workdir, env_variables, project_dirs)?;
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let mut command_wrap = TokioCommandWrap::from(command);
        #[cfg(unix)]
        command_wrap.wrap(ProcessGroup::leader());
        #[cfg(windows)]
        command_wrap.wrap(JobObject);
        let mut process = command_wrap.spawn().map_err(|e| format!("failed to create process: {e}"))?;

        let mut stdout_reader = BufReader::new(process.stdout().take().ok_or("Failed to open stdout")?);
        let mut stderr_reader = BufReader::new(process.stderr().take().ok_or("Failed to open stderr")?);

        let t0 = tokio::time::Instant::now();

        let mut accumulated_stdout = String::new();
        let mut accumulated_stderr = String::new();
        let mut exit_code: i32 = -100000;

        loop {
            if t0.elapsed() >= tokio::time::Duration::from_secs(cfg.startup_wait.to_string().parse::<u64>().unwrap_or(10)) {
                actions_log.push_str(&format!("Timeout {:.2}s reached while waiting for the service to start.\n\n", t0.elapsed().as_secs_f64()));
                break;
            }

            let (stdout_out, stderr_out) = get_stdout_and_stderr(100, &mut stdout_reader, &mut stderr_reader).await?;
            accumulated_stdout.push_str(&stdout_out);
            accumulated_stderr.push_str(&stderr_out);

            if !cfg.startup_wait_keyword.is_empty() {
                if accumulated_stdout.contains(&cfg.startup_wait_keyword) || accumulated_stderr.contains(&cfg.startup_wait_keyword) {
                    actions_log.push_str(&format!("Startup keyword '{}' found in output, success!\n\n", cfg.startup_wait_keyword));
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
                service_name: service_name.to_string(),
            });
            gcx.write().await.integration_sessions.insert(session_key.to_string(), Arc::new(AMutex::new(session)));
        }

        tracing::info!("SERVICE START LOG:\n{}", actions_log);
    }

    Ok(actions_log)
}

#[async_trait]
impl Tool for ToolService {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let gcx = ccx.lock().await.global_context.clone();
        let mut args_str: HashMap<String, String> = HashMap::new();

        for (k, v) in args.iter() {
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

        let command = replace_args(self.cfg.command.as_str(), &args_str);
        let workdir = replace_args(self.cfg.command_workdir.as_str(), &args_str);
        let mut error_log = Vec::<YamlError>::new();
        let env_variables = crate::integrations::setting_up_integrations::get_vars_for_replacements(gcx.clone(), &mut error_log).await;

        let tool_ouput = {
            let action = args_str.get("action").cloned().unwrap_or("start".to_string());
            if !["start", "restart", "stop", "status"].contains(&action.as_str()) {
                return Err("Tool call is invalid. Param 'action' must be one of 'start', 'restart', 'stop', 'status'. Try again".to_string());
            }
            execute_background_command(
                gcx, &self.name, &command, &workdir, &self.cfg, action.as_str(), &env_variables,
            ).await?
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
        let mut parameters = self.cfg.parameters.clone();
        parameters.push(ToolParam {
            name: "action".to_string(),
            param_type: "string".to_string(),
            description: "Action to perform: start, restart, stop, status".to_string(),
        });

        let parameters_required = self.cfg.parameters_required.clone().unwrap_or_else(|| {
            self.cfg.parameters.iter().map(|param| param.name.clone()).collect()
        });

        ToolDesc {
            name: self.name.clone(),
            agentic: true,
            experimental: false,
            description: self.cfg.description.clone(),
            parameters,
            parameters_required,
        }
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(self.integr_common().confirmation)
    }

    fn has_config_path(&self) -> Option<String> {
        Some(self.config_path.clone())
    }
}

pub const CMDLINE_SERVICE_INTEGRATION_SCHEMA: &str = r#"
fields:
  command:
    f_type: string_long
    f_desc: "The command to execute."
    f_placeholder: "echo Hello World"
  command_workdir:
    f_type: string_long
    f_desc: "The working directory for the command."
    f_placeholder: "/path/to/workdir"
  description:
    f_type: string_long
    f_desc: "The model will see this description, why the model should call this?"
  parameters:
    f_type: "tool_parameters"
    f_desc: "The model will fill in those parameters."
  startup_wait_port:
    f_type: string_short
    f_desc: "Wait for TCP to become occupied during startup."
    f_placeholder: "8080"
  startup_wait:
    f_type: string_short
    f_desc: "Max time to wait for service to start."
    f_default: "10"
  startup_wait_keyword:
    f_type: string
    f_desc: "Wait until a keyword appears in stdout or stderr at startup."
    f_placeholder: "Ready"
description: |
  As opposed to command line argumenets

  There you can adapt any command line tool for use by AI model. You can give the model instructions why to call it, which parameters to provide,
  set a timeout and restrict the output. If you want a tool that runs in the background such as a web server, use service_* instead.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
confirmation:
  ask_user_default: ["*"]
  deny_default: ["sudo*"]
smartlinks:
  - sl_label: "Test"
    sl_chat:
      - role: "user"
        content: |
          🔧 Test the tool that corresponds to %CURRENT_CONFIG%
          If the tool isn't available or doesn't work, go through the usual plan in the system prompt. If it works express happiness, and change nothing.
    sl_enable_only_with_tool: true
  - sl_label: "Auto Configure"
    sl_chat:
      - role: "user"
        content: |
          🔧 Please write %CURRENT_CONFIG% based on what you see in the project. Follow the plan in the system prompt. Remember that service_ tools
          are only suitable for blocking command line commands that run until you hit Ctrl+C, like web servers or `tail -f`.
"#;
