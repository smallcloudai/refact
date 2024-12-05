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
use crate::integrations::integr_abstract::IntegrationTrait;


#[derive(Deserialize, Serialize, Clone, Default)]
pub struct CmdlineToolConfig {
    pub command: String,
    pub command_workdir: String,

    pub description: String,
    pub parameters: Vec<ToolParam>,
    pub parameters_required: Option<Vec<String>>,

    // blocking
    #[serde(default = "_default_timeout")]
    pub timeout: u64,
    #[serde(default)]
    pub output_filter: CmdlineOutputFilter,

    // background
    #[serde(default)]
    pub startup_wait_port: Option<u16>,
    #[serde(default = "_default_startup_wait")]
    pub startup_wait: u64,
    #[serde(default)]
    pub startup_wait_keyword: Option<String>,
}

fn _default_timeout() -> u64 {
    120
}

fn _default_startup_wait() -> u64 {
    10
}

#[derive(Default)]
pub struct ToolCmdline {
    // is_service: bool,
    pub name: String,
    pub cfg: CmdlineToolConfig,
}

impl IntegrationTrait for ToolCmdline {
    fn integr_settings_apply(&mut self, value: &serde_json::Value) -> Result<(), String> {
        match serde_json::from_value::<CmdlineToolConfig>(value.clone()) {
            Ok(x) => self.cfg = x,
            Err(e) => {
                tracing::error!("Failed to apply settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        Ok(())
    }

    fn integr_settings_as_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap()
    }

    fn integr_upgrade_to_tool(&self, integr_name: &String) -> Box<dyn Tool + Send> {
        Box::new(ToolCmdline {
            // is_service: self.is_service,
            name: integr_name.clone(),
            cfg: self.cfg.clone(),
        }) as Box<dyn Tool + Send>
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

pub fn _create_command_from_string(
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

pub async fn execute_blocking_command(
    command: &str,
    cfg: &CmdlineToolConfig,
    command_workdir: &String,
) -> Result<String, String> {
    info!("EXEC workdir {}:\n{:?}", command_workdir, command);
    let command_future = async {
        let mut cmd = _create_command_from_string(command, command_workdir)?;
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

#[async_trait]
impl Tool for ToolCmdline {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {

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

        let command = replace_args(self.cfg.command.as_str(), &args_str);
        let workdir = replace_args(self.cfg.command_workdir.as_str(), &args_str);

        let tool_ouput = execute_blocking_command(&command, &self.cfg, &workdir).await?;

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

pub const CMDLINE_INTEGRATION_SCHEMA: &str = r#"
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
    f_placeholder: ""
  parameters:
    f_type: "tool_parameters"
    f_desc: "The model will fill in those parameters."
  timeout:
    f_type: integer
    f_desc: "The command must immediately return the results, it can't be interactive. If the command runs for too long, it will be terminated and stderr/stdout collected will be presented to the model."
    f_default: 10
  output_filter:
    f_type: "output_filter"
    f_desc: "The output from the command can be long or even quasi-infinite. This section allows to set limits, prioritize top or bottom, or use regexp to show the model the relevant part."
    f_placeholder: "filter"
description: |
  There you can adapt any command line tool for use by AI model. You can give the model instructions why to call it, which parameters to provide,
  set a timeout and restrict the output. If you want a tool that runs in the background such as a web server, use service_* instead.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
"#;
