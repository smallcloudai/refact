use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use serde::Deserialize;
use serde::Serialize;
use async_trait::async_trait;
use itertools::Itertools;
use serde_json::Value;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{ToolParam, Tool, ToolDesc};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::postprocessing::pp_command_output::CmdlineOutputFilter;
use crate::integrations::integr_abstract::IntegrationTrait;
use crate::integrations::integr_cmdline::{execute_blocking_command, CmdlineToolConfig};

use tracing::info;

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct SettingsShell {
    #[serde(default)]
    pub timeout: String,
    #[serde(default)]
    pub output_filter: CmdlineOutputFilter,
}

#[derive(Default)]
pub struct ToolShell {
    pub cfg: SettingsShell,
}

impl IntegrationTrait for ToolShell {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn integr_settings_apply(&mut self, value: &Value) -> Result<(), String> {
        match serde_json::from_value::<SettingsShell>(value.clone()) {
            Ok(x) => self.cfg = x,
            Err(e) => {
                tracing::error!("Failed to apply settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        Ok(())
    }

    fn integr_settings_as_json(&self) -> Value {
        serde_json::to_value(&self.cfg).unwrap()
    }

    fn integr_upgrade_to_tool(&self, _integr_name: &str) -> Box<dyn Tool + Send> {
        Box::new(ToolShell {
            cfg: self.cfg.clone(),
        }) as Box<dyn Tool + Send>
    }

    fn integr_schema(&self) -> &str
    {
        SHELL_INTEGRATION_SCHEMA
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
        let gcx = {
            let ccx_lock = ccx.lock().await;
            ccx_lock.global_context.clone()
        };

        let (command, workdir) = parse_args(args)?;
        let env_variables = crate::integrations::setting_up_integrations::get_vars_for_replacements(gcx.clone()).await;
        let project_dirs = crate::files_correction::get_project_dirs(gcx.clone()).await;

        let cmdline_cfg = CmdlineToolConfig {
            timeout: self.cfg.timeout.clone(), output_filter: self.cfg.output_filter.clone(),
            command: "".to_string(), command_workdir: "".to_string(), description: "".to_string(),
            parameters: vec![], parameters_required: None,
            startup_wait_port: None, startup_wait: 0u64, startup_wait_keyword: None,
        };
        // let tool_output = execute_blocking_command(&command, &cmdline_cfg, &workdir, &env_variables, project_dirs).await?;
        let tool_output = "Waiting for rules logic to be implemented.".to_string();

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
            description: vec![
                "Execute single shell command with user's confirmation.",
                "Use it for external agent calls like deps installation.",
            ].join("\n"),
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

    // TODO: this tool needs strict confirmation/deny rules and we need to move this logic into tools itself
    fn command_to_match_against_confirm_deny(
        &self,
        args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        let (command, _) = parse_args(args)?;
        Ok(command)
    }
}

fn parse_args(args: &HashMap<String, Value>) -> Result<(String, String), String> {
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
            let workdir = PathBuf::from(s.clone());
            if !workdir.exists() {
                return Err("Workdir doesn't exist".to_string());
            } else {
                s.clone()
            }
        },
        Some(v) => return Err(format!("argument `workdir` is not a string: {:?}", v)),
        None => return Err("Missing argument `workdir`".to_string())
    };

    Ok((command, workdir))
}

pub const SHELL_INTEGRATION_SCHEMA: &str = r#"
fields:
  timeout:
    f_type: string_short
    f_desc: "The command must immediately return the results, it can't be interactive. If the command runs for too long, it will be terminated and stderr/stdout collected will be presented to the model."
    f_default: "10"
    f_extra: true
  output_filter:
    f_type: "output_filter"
    f_desc: "The output from the command can be long or even quasi-infinite. This section allows to set limits, prioritize top or bottom, or use regexp to show the model the relevant part."
    f_placeholder: "filter"
    f_extra: true
description: |
  Allows to execute any command line tool with confirmation from the chat itself.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
"#;
