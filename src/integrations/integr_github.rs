use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex as AMutex;
use tokio::process::Command;
use async_trait::async_trait;
use tracing::{error, info};
use serde::{Deserialize, Serialize};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ContextEnum, ChatMessage, ChatContent, ChatUsage};

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
    pub common:  IntegrationCommon,
    pub settings_github: SettingsGitHub,
}

impl IntegrationTrait for ToolGithub {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn integr_settings_apply(&mut self, value: &Value) -> Result<(), String> {
        match serde_json::from_value::<SettingsGitHub>(value.clone()) {
            Ok(settings_github) => {
                info!("Github settings applied: {:?}", settings_github);
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
        Ok(())
    }

    fn integr_settings_as_json(&self) -> Value {
        serde_json::to_value(&self.settings_github).unwrap_or_default()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    fn integr_upgrade_to_tool(&self, _integr_name: &str) -> Box<dyn Tool + Send> {
        Box::new(ToolGithub {
            common: self.common.clone(),
            settings_github: self.settings_github.clone()
        }) as Box<dyn Tool + Send>
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
        let output = Command::new(gh_binary_path)
            .args(&command_args)
            .current_dir(&project_dir)
            .env("gh_token", &self.settings_github.gh_token)
            .output()
            .await
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !stderr.is_empty() {
            error!("Error: {:?}", stderr);
            return Err(stderr);
        }

        let content = if stdout.starts_with("[") {
            match serde_json::from_str::<Value>(&stdout) {
                Ok(Value::Array(arr)) => {
                    let row_count = arr.len();
                    format!("{}\n\n💿 The UI has the capability to view tool result json efficiently. The result contains {} rows. Write no more than 3 rows as text and possibly \"and N more\" wording, keep it short.",
                        stdout, row_count
                    )
                },
                Ok(_) => stdout,
                Err(_) => stdout,
            }
        } else {
            stdout
        };
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

    fn confirmation_info(&self) -> Option<IntegrationConfirmation> {
        Some(self.integr_common().confirmation)
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
  gh_binary_path:
    f_type: string_long
    f_desc: "Path to the GitHub CLI binary. Leave empty to use the default 'gh' command."
    f_placeholder: "/usr/local/bin/gh"
    f_label: "GH Binary Path"
  gh_token:
    f_type: string_long
    f_desc: "GitHub Personal Access Token for authentication."
    f_placeholder: "ghp_xxxxxxxxxxxxxxxx"
description: |
  The GitHub integration allows interaction with GitHub repositories using the GitHub CLI.
  It provides functionality for various GitHub operations such as creating issues, pull requests, and more.
smartlinks:
  - sl_label: "Test"
    sl_chat:
      - role: "user"
        content: |
          🔧 The `github` (`gh`) tool should be visible now. To test the tool, list opened pull requests for `smallcloudai/refact-lsp`, and briefly describe them and express
          happiness, and change nothing. If it doesn't work or the tool isn't available, go through the usual plan in the system prompt.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
confirmation:
  ask_user_default: ["gh * delete *", "gh * close *"]
  deny_default: ["gh auth token *"]
"#;
