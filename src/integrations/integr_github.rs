use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex as AMutex;
use tokio::process::Command;
use async_trait::async_trait;
use tracing::{error, info};
use serde::{Deserialize, Serialize};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ContextEnum, ChatMessage, ChatContent, ChatUsage};

use crate::files_correction::to_pathbuf_normalize;
use crate::integrations::go_to_configuration_message;
use crate::tools::tools_description::Tool;
use serde_json::Value;
use crate::integrations::integr_abstract::{IntegrationCommon, IntegrationConfirmation, IntegrationTrait};


#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[allow(non_snake_case)]
pub struct SettingsGitHub {
    pub gh_binary_path: String,
    pub gh_token: String,
}

#[derive(Default)]
pub struct ToolGithub {
    pub common: IntegrationCommon,
    pub settings_github: SettingsGitHub,
    pub config_path: String,
}

#[async_trait]
impl IntegrationTrait for ToolGithub {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn integr_settings_apply(&mut self, value: &Value, config_path: String) -> Result<(), String> {
        match serde_json::from_value::<SettingsGitHub>(value.clone()) {
            Ok(settings_github) => {
                self.settings_github = settings_github;
            },
            Err(e) => {
                error!("Failed to apply settings: {}\n{:?}", e, value);
                return Err(e.to_string());
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
        serde_json::to_value(&self.settings_github).unwrap_or_default()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    fn integr_tools(&self, _integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        vec![Box::new(ToolGithub {
            common: self.common.clone(),
            settings_github: self.settings_github.clone(),
            config_path: self.config_path.clone(),
        })]
    }

    fn integr_schema(&self) -> &str { GITHUB_INTEGRATION_SCHEMA }
}

#[async_trait]
impl Tool for ToolGithub {
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

        let mut gh_binary_path = self.settings_github.gh_binary_path.clone();
        if gh_binary_path.is_empty() {
            gh_binary_path = "gh".to_string();
        }
        let output = Command::new(&gh_binary_path)
            .args(&command_args)
            .current_dir(&to_pathbuf_normalize(&project_dir))
            .env("GH_TOKEN", &self.settings_github.gh_token)
            .env("GITHUB_TOKEN", &self.settings_github.gh_token)
            .stdin(std::process::Stdio::null())
            .output()
            .await
            .map_err(|e| format!("!{}, {} failed:\n{}",
                go_to_configuration_message("github"), gh_binary_path, e.to_string()))?;

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
        command_args.insert(0, "gh".to_string());
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
    if parsed_args[0] == "gh" {
        parsed_args.remove(0);
    }

    Ok(parsed_args)
}

const GITHUB_INTEGRATION_SCHEMA: &str = r#"
fields:
  gh_token:
    f_type: string_long
    f_desc: "GitHub Personal Access Token, you can create one [here](https://github.com/settings/tokens). If you don't want to send your key to the AI model that helps you to configure the agent, put it into secrets.yaml and write `$MY_SECRET_VARIABLE` in this field."
    f_placeholder: "ghp_xxxxxxxxxxxxxxxx"
    f_label: "Token"
    smartlinks:
      - sl_label: "Open secrets.yaml"
        sl_goto: "EDITOR:secrets.yaml"
  gh_binary_path:
    f_type: string_long
    f_desc: "Path to the GitHub CLI binary. Leave empty if you have it in PATH."
    f_placeholder: "/usr/local/bin/gh"
    f_label: "GH Binary Path"
    f_extra: true
description: |
  The GitHub integration allows interaction with GitHub repositories using the GitHub CLI.
  It provides functionality for various GitHub operations such as creating issues, pull requests, and more.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
confirmation:
  ask_user_default: ["gh * delete *", "gh * close *"]
  deny_default: ["gh auth token *"]
smartlinks:
  - sl_label: "Test"
    sl_chat:
      - role: "user"
        content: |
          🔧 The `github` tool should be visible now. To test the tool, list opened pull requests for the current project, and briefly describe them.
          If it doesn't work or the tool isn't available, go through the usual plan in the system prompt.
    sl_enable_only_with_tool: true
"#;
