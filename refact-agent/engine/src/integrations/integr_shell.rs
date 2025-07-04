use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use async_trait::async_trait;
use tokio::process::Command;

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
use crate::integrations::process_io_utils::{execute_command, AnsiStrippable};
use crate::tools::tools_description::{ToolParam, Tool, ToolDesc, ToolSource, ToolSourceType, MatchConfirmDeny, MatchConfirmDenyResult};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::postprocessing::pp_command_output::CmdlineOutputFilter;
use crate::integrations::integr_abstract::{IntegrationCommon, IntegrationTrait};
use crate::custom_error::YamlError;
use crate::tools::tools_execute::command_should_be_denied;


#[derive(Deserialize, Serialize, Clone, Default)]
pub struct SettingsShell {
    #[serde(default)]
    pub timeout: String,
    #[serde(default)]
    pub output_filter: CmdlineOutputFilter,
}

#[derive(Default)]
pub struct ToolShell {
    pub common: IntegrationCommon,
    pub cfg: SettingsShell,
    pub config_path: String,
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
        let (command, workdir_maybe) = parse_args(gcx.clone(), args).await?;
        let timeout = self.cfg.timeout.parse::<u64>().unwrap_or(10);

        let mut error_log = Vec::<YamlError>::new();
        let env_variables = crate::integrations::setting_up_integrations::get_vars_for_replacements(gcx.clone(), &mut error_log).await;

        let tool_output = execute_shell_command(
            &command,
            &workdir_maybe,
            timeout,
            &self.cfg.output_filter,
            &env_variables,
            gcx.clone(),
        ).await?;

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
            display_name: "Shell".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Integration,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Execute a single command, using the \"sh\" on unix-like systems and \"powershell.exe\" on windows. Use it for one-time tasks like dependencies installation. Don't call this unless you have to. Not suitable for regular work because it requires a confirmation at each step.".to_string(),
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
        let (command, _) = parse_args(gcx, args).await?;
        Ok(command)
    }

    fn has_config_path(&self) -> Option<String> {
        Some(self.config_path.clone())
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

    let mut out = crate::integrations::integr_cmdline::format_output(&filtered_stdout, &filtered_stderr);
    let exit_code = output.status.code().unwrap_or_default();
    out.push_str(&format!("The command was running {:.3}s, finished with exit code {exit_code}\n", duration.as_secs_f64()));
    Ok(out)
}

async fn parse_args(gcx: Arc<ARwLock<GlobalContext>>, args: &HashMap<String, Value>) -> Result<(String, Option<PathBuf>), String> {
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

    Ok((command, workdir))
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
description: |
  Allows to execute any command line tool with confirmation from the chat itself.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
confirmation:
  ask_user_default: ["*"]
  deny_default: ["sudo*"]
"#;
