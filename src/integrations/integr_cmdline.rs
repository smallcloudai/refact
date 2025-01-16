use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use std::process::Stdio;
use tokio::sync::Mutex as AMutex;
use serde::Deserialize;
use serde::Serialize;
use async_trait::async_trait;
use tokio::process::Command;
use tracing::info;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{ToolParam, Tool, ToolDesc};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::postprocessing::pp_command_output::{CmdlineOutputFilter, output_mini_postprocessing};
use crate::integrations::integr_abstract::{IntegrationTrait, IntegrationCommon, IntegrationConfirmation};
use crate::integrations::utils::{serialize_num_to_str, deserialize_str_to_num, serialize_opt_num_to_str, deserialize_str_to_opt_num};
use crate::integrations::setting_up_integrations::YamlError;


#[derive(Deserialize, Serialize, Clone, Default)]
pub struct CmdlineToolConfig {
    pub command: String,
    pub command_workdir: String,

    pub description: String,
    pub parameters: Vec<ToolParam>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters_required: Option<Vec<String>>,

    // blocking
    #[serde(default)]
    pub timeout: String,
    #[serde(default)]
    pub output_filter: CmdlineOutputFilter,

    // background
    #[serde(default, serialize_with = "serialize_opt_num_to_str", deserialize_with = "deserialize_str_to_opt_num")]
    pub startup_wait_port: Option<u16>,
    #[serde(default = "_default_startup_wait", serialize_with = "serialize_num_to_str", deserialize_with = "deserialize_str_to_num")]
    pub startup_wait: u64,
    #[serde(default)]
    pub startup_wait_keyword: String,
}

fn _default_startup_wait() -> u64 {
    10
}

#[derive(Default)]
pub struct ToolCmdline {
    pub common: IntegrationCommon,
    pub name: String,
    pub cfg: CmdlineToolConfig,
    pub config_path: String,
}

#[async_trait]
impl IntegrationTrait for ToolCmdline {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn integr_settings_apply(&mut self, value: &serde_json::Value, config_path: String) -> Result<(), String> {
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

    fn integr_tools(&self, integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        vec![Box::new(ToolCmdline {
            common: self.common.clone(),
            name: integr_name.to_string(),
            cfg: self.cfg.clone(),
            config_path: self.config_path.clone(),
        })]
    }

    fn integr_schema(&self) -> &str
    {
        CMDLINE_INTEGRATION_SCHEMA
    }
}

pub fn replace_args(x: &str, args_str: &HashMap<String, String>) -> String {
    let mut result = x.to_string();
    for (key, value) in args_str {
        result = result.replace(&format!("%{}%", key), value);
    }
    result
}

pub fn format_output(stdout_out: &str, stderr_out: &str) -> String {
    let mut out = String::new();
    if !stdout_out.is_empty() && stderr_out.is_empty() {
        // special case: just clean output, nice
        out.push_str(&format!("{}\n\n", stdout_out));
    } else {
        if !stdout_out.is_empty() {
            out.push_str(&format!("STDOUT\n```\n{}```\n\n", stdout_out));
        }
        if !stderr_out.is_empty() {
            out.push_str(&format!("STDERR\n```\n{}```\n\n", stderr_out));
        }
        if stdout_out.is_empty() && stderr_out.is_empty() {
            out.push_str(&format!("Nothing in STDOUT/STDERR\n\n"));
        }
    }
    out
}

pub fn create_command_from_string(
    cmd_string: &str,
    command_workdir: &String,
    env_variables: &HashMap<String, String>,
    project_dirs: Vec<PathBuf>,
) -> Result<Command, String> {
    let shell = if cfg!(target_os = "windows") { "powershell.exe" } else { "sh" };
    let shell_arg = if cfg!(target_os = "windows") { "-Command" } else { "-c" };
    let mut cmd = Command::new(shell);

    if command_workdir.is_empty() {
        if let Some(first_project_dir) = project_dirs.first() {
            cmd.current_dir(first_project_dir);
        } else {
            tracing::warn!("no working directory, using whatever directory this binary is run :/");
        }
    } else {
        cmd.current_dir(command_workdir);
    }

    for (key, value) in env_variables {
        cmd.env(key, value);
    }

    if cmd_string.is_empty() {
        return Err("Command is empty".to_string());
    }
    cmd.stdin(std::process::Stdio::null());
    cmd.arg(shell_arg).arg(cmd_string);
    tracing::info!("command: {}", cmd_string);

    Ok(cmd)
}

pub async fn execute_blocking_command(
    command: &str,
    cfg: &CmdlineToolConfig,
    command_workdir: &String,
    env_variables: &HashMap<String, String>,
    project_dirs: Vec<PathBuf>,
) -> Result<String, String> {
    info!("EXEC workdir {:?}:\n{:?}", command_workdir, command);

    let command_future = async {
        let mut cmd = create_command_from_string(command, command_workdir, env_variables, project_dirs)?;
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
        out.push_str(&format!("The command was running {:.3}s, finished with exit code {exit_code}\n", duration.as_secs_f64()));
        Ok(out)
    };

    let timeout_duration = tokio::time::Duration::from_secs(cfg.timeout.parse::<u64>().unwrap_or(10));
    let result = tokio::time::timeout(timeout_duration, command_future).await;

    match result {
        Ok(res) => res,
        Err(_) => Err(format!("command timed out after {:?}", timeout_duration)),
    }
}

fn parse_command_args(args: &HashMap<String, serde_json::Value>, cfg: &CmdlineToolConfig) -> Result<(String, String), String>
{
    let mut args_str: HashMap<String, String> = HashMap::new();
    let valid_params: Vec<String> = cfg.parameters.iter().map(|p| p.name.clone()).collect();

    for (k, v) in args.iter() {
        if !valid_params.contains(k) {
            return Err(format!("Unexpected argument `{}`", k));
        }
        match v {
            serde_json::Value::String(s) => { args_str.insert(k.clone(), s.clone()); },
            _ => return Err(format!("argument `{}` is not a string: {:?}", k, v)),
        }
    }

    for param in &cfg.parameters {
        if cfg.parameters_required.as_ref().map_or(false, |req| req.contains(&param.name)) && !args_str.contains_key(&param.name) {
            return Err(format!("Missing required argument `{}`", param.name));
        }
    }

    let command = replace_args(cfg.command.as_str(), &args_str);
    let workdir = replace_args(cfg.command_workdir.as_str(), &args_str);
    Ok((command, workdir))
}

#[async_trait]
impl Tool for ToolCmdline {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let (command, workdir) = parse_command_args(args, &self.cfg)?;

        let gcx = ccx.lock().await.global_context.clone();
        let mut error_log = Vec::<YamlError>::new();
        let env_variables = crate::integrations::setting_up_integrations::get_vars_for_replacements(gcx.clone(), &mut error_log).await;
        let project_dirs = crate::files_correction::get_project_dirs(gcx.clone()).await;

        let tool_output = execute_blocking_command(&command, &self.cfg, &workdir, &env_variables, project_dirs).await?;

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

    fn command_to_match_against_confirm_deny(
        &self,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<String, String> {
        let (command, _workdir) = parse_command_args(args, &self.cfg)?;
        return Ok(command);
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(self.integr_common().confirmation)
    }

    fn has_config_path(&self) -> Option<String> {
        Some(self.config_path.clone())
    }
}

pub const CMDLINE_INTEGRATION_SCHEMA: &str = r#"
fields:
  command:
    f_type: string_long
    f_desc: "The command to execute. To let model produce part of the command, use %param_name% notation."
    f_placeholder: "echo Hello World"
  command_workdir:
    f_type: string_long
    f_desc: "The working directory for the command. If empty then workspace directory will be used. There you can use %param_name% as well."
    f_placeholder: "/path/to/workdir"
  description:
    f_type: string_long
    f_desc: "The model will see this description, why the model should call this?"
  parameters:
    f_type: "tool_parameters"
    f_desc: "The parameters that the model should fill out. Use description to tell the model what a parameter does. The only way you can use values coming from the model is to put them into %param_name% notation in the command or the working directory."
  timeout:
    f_type: string_short
    f_desc: "The command must immediately return the results, it can't be interactive. If the command runs for too long, it will be terminated and stderr/stdout collected will be presented to the model."
    f_default: "10"
  output_filter:
    f_type: "output_filter"
    f_desc: "The output from the command can be long or even quasi-infinite. This section allows to set limits, prioritize top or bottom, or use regexp to show the model the relevant part."
    f_placeholder: "filter"
    f_extra: true
description: |
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
          🔧 Please write %CURRENT_CONFIG% based on what you see in the project. Follow the plan in the system prompt.
"#;
