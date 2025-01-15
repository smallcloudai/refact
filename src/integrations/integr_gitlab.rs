use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex as AMutex;
use tokio::process::Command;
use async_trait::async_trait;
use tracing::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ContextEnum, ChatMessage, ChatContent, ChatUsage};
use crate::files_correction::to_pathbuf_normalize;
use crate::integrations::go_to_configuration_message;
use crate::tools::tools_description::Tool;
use crate::integrations::integr_abstract::{IntegrationCommon, IntegrationConfirmation, IntegrationTrait};

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[allow(non_snake_case)]
pub struct SettingsGitLab {
    pub glab_binary_path: String,
    pub glab_token: String,
}

#[derive(Default)]
pub struct ToolGitlab {
    pub common: IntegrationCommon,
    pub settings_gitlab: SettingsGitLab,
    pub config_path: String,
}

impl IntegrationTrait for ToolGitlab {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn integr_settings_apply(&mut self, value: &Value, config_path: String) -> Result<(), String> {
        match serde_json::from_value::<SettingsGitLab>(value.clone()) {
            Ok(settings_gitlab) => {
                info!("GitLab settings applied: {:?}", settings_gitlab);
                self.settings_gitlab = settings_gitlab;
            },
            Err(e) => {
                error!("Failed to apply settings: {}\n{:?}", e, value);
                return Err(e.to_string())
            }
        };
        match serde_json::from_value::<IntegrationCommon>(value.clone()) {
            Ok(x) => self.common = x,
            Err(e) => {
                error!("Failed to apply common settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        };
        self.config_path = config_path;
        Ok(())
    }

    fn integr_settings_as_json(&self) -> Value {
        serde_json::to_value(&self.settings_gitlab).unwrap_or_default()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    fn integr_tools(&self, _integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        vec![Box::new(ToolGitlab {
            common: self.common.clone(),
            settings_gitlab: self.settings_gitlab.clone(),
            config_path: self.config_path.clone(),
        })]
    }

    fn integr_schema(&self) -> &str { GITLAB_INTEGRATION_SCHEMA }
}

#[async_trait]
impl Tool for ToolGitlab {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let project_dir = match args.get("project_dir") {
            Some(Value::String(s)) => s,
            Some(v) => return Err(format!("argument `project_dir` is not a string: {:?}", v)),
            None => return Err("Missing argument `project_dir`".to_string())
        };
        let command_args = parse_command_args(args)?;

        let mut glab_binary_path = self.settings_gitlab.glab_binary_path.clone();
        if glab_binary_path.is_empty() {
            glab_binary_path = "glab".to_string();
        }
        let output = Command::new(&glab_binary_path)
            .args(&command_args)
            .current_dir(&to_pathbuf_normalize(&project_dir))
            .env("GITLAB_TOKEN", &self.settings_gitlab.glab_token)
            .stdin(std::process::Stdio::null())
            .output()
            .await
            .map_err(|e| format!("!{}, {} failed:\n{}",
                go_to_configuration_message("gitlab"), glab_binary_path, e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let stdout_content = if stdout.starts_with("[") {
            match serde_json::from_str::<Value>(&stdout) {
                Ok(Value::Array(arr)) => {
                    let row_count = arr.len();
                    format!("{}\n\n💿 The UI has the capability to view tool result json efficiently. The result contains {} rows. Unless user specified otherwise, write no more than 3 rows as text and possibly \"and N more\" wording, keep it short.",
                        stdout, row_count
                    )
                },
                Ok(_) => stdout,
                Err(_) => stdout,
            }
        } else {
            stdout
        };

        let mut content = String::new();
        if !stdout_content.is_empty() {
            content.push_str(format!("stdout:\n{}\n", stdout_content).as_str());
        }
        if !stderr.is_empty() {
            content.push_str(format!("stderr:\n{}\n", stderr).as_str());
        }

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(content),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok((false, results))
    }

    fn command_to_match_against_confirm_deny(
        &self,
        args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        let mut command_args = parse_command_args(args)?;
        command_args.insert(0, "glab".to_string());
        Ok(command_args.join(" "))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }

    fn usage(&mut self) -> &mut Option<ChatUsage> {
        static mut DEFAULT_USAGE: Option<ChatUsage> = None;
        #[allow(static_mut_refs)]
        unsafe { &mut DEFAULT_USAGE }
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(self.integr_common().confirmation)
    }

    fn has_config_path(&self) -> Option<String> {
        Some(self.config_path.clone())
    }
}

fn parse_command_args(args: &HashMap<String, Value>) -> Result<Vec<String>, String> {
    let command = match args.get("command") {
        Some(Value::String(s)) => s,
        Some(v) => return Err(format!("argument `command` is not a string: {:?}", v)),
        None => return Err("Missing argument `command`".to_string())
    };

    let mut parsed_args = shell_words::split(&command).map_err(|e| e.to_string())?;
    if parsed_args.is_empty() {
        return Err("Parsed command is empty".to_string());
    }
    for (i, arg) in parsed_args.iter().enumerate() {
        info!("argument[{}]: {}", i, arg);
    }
    if parsed_args[0] == "glab" {
        parsed_args.remove(0);
    }

    Ok(parsed_args)
}

const GITLAB_INTEGRATION_SCHEMA: &str = r#"
fields:
  glab_token:
    f_type: string_long
    f_desc: "GitLab Personal Access Token, you can get one [here](https://gitlab.com/-/user_settings/personal_access_tokens). If you don't want to send your key to the AI model that helps you to configure the agent, put it into secrets.yaml and write `$MY_SECRET_VARIABLE` in this field."
    f_placeholder: "glpat_xxxxxxxxxxxxxxxx"
    smartlinks:
      - sl_label: "Open secrets.yaml"
        sl_goto: "EDITOR:secrets.yaml"
  glab_binary_path:
    f_type: string_long
    f_desc: "Path to the GitLab CLI binary. Leave empty to use the default 'glab' command. On Windows, `glab` installed via Chocolatey may have issues, consider installing it from the official website or Winget instead."
    f_placeholder: "/usr/local/bin/glab"
    f_label: "glab binary path"
    f_extra: true
description: |
  The GitLab integration allows interaction with GitLab repositories using the GitLab CLI.
  It provides functionality for various GitLab operations such as creating issues, merge requests, and more.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
confirmation:
  ask_user_default: ["glab * delete *"]
  deny_default: ["glab auth token *"]
smartlinks:
  - sl_label: "Test"
    sl_chat:
      - role: "user"
        content: |
          🔧 The `gitlab` tool should be visible now. To test the tool, list opened merge requests for the current project on GitLab, and briefly describe them.
          If it doesn't work or the tool isn't available, go through the usual plan in the system prompt.
    sl_enable_only_with_tool: true
"#;
