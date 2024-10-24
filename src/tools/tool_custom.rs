use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;
use async_trait::async_trait;
use indexmap::IndexMap;
use regex::Regex;
use tokio::sync::Mutex as AMutex;
use tracing::info;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use std::process::Stdio;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{AtParamDict, Tool};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};


pub struct ToolCustom {
    pub name: String,
    pub parameters: Vec<AtParamDict>,
    pub parameters_required: Vec<String>,
    pub command: String,
    pub runs_in_background: bool,
    pub runs_in_background_false_timeout: usize,
    pub output_filter: IndexMap<String, String> // todo
}

fn replace_magics_from_command(
    command: &str, 
    args: &HashMap<String, Value>, 
    required_params: &Vec<String>
) -> Result<String, String> {
    let args_str: HashMap<String, String> = args.iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
        .collect();

    let mut command_clone = command.to_string();
    for (key, value) in &args_str {
        let pattern = format!("%{}%", key);
        let re = Regex::new(&regex::escape(&pattern)).unwrap();
        command_clone = re.replace_all(&command_clone, value.as_str()).to_string();
    }

    for param in required_params {
        let pattern = format!("%{}%", param);
        if command_clone.contains(&pattern) {
            return Err(format!("Required parameter '{}' is missing in the arguments", param));
        }
    }
    Ok(command_clone)
}

async fn execute_command_with_timeout(
    command: &str, 
    timeout_duration: Duration
) -> Result<String, String> {
    let command_future = async {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let mut res = "".to_string();
        if output.status.success() {
            res.push_str(&format!("EXEC SUCCESS: {command}\n"));
        } else {
            res.push_str(&format!("EXEC FAIL: {command}\n"));
        }
        res.push_str(&format!("STDOUT:\n{stdout}"));
        if !stderr.is_empty() {
            res.push_str(&format!("\nSTDERR:\n{stderr}"));
        }
        Ok(res)
    };

    timeout(timeout_duration, command_future).await
        .unwrap_or_else(|_| Err("Command execution timed out".to_string()))
}


#[async_trait]
impl Tool for ToolCustom {
    async fn tool_execute(
        &mut self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let command = replace_magics_from_command(self.command.as_str(), args, &self.parameters_required)?;
        
        let resp = if !self.runs_in_background {
            info!("EXEC: {command}");
            execute_command_with_timeout(&command, Duration::from_secs(self.runs_in_background_false_timeout as u64)).await?
        } else {
            !unimplemented!()
        };

        let result = vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(resp),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })];

        Ok((false, result))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}
