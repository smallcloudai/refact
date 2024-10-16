use crate::tools::tools_description::Tool;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ChatUsage};
use crate::call_validation::ContextEnum;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tokio::process::Command;
use serde_yaml;
use std::path::PathBuf;
use tracing::info;

pub struct ToolPostgres {
    connection_string: String,
    psql_binary_path: PathBuf,
}

impl ToolPostgres {
    pub fn new_if_configured(integrations_value: &serde_yaml::Value) -> Option<Self> {
        let postgres = integrations_value.get("postgres")?;
        let connection_string = postgres.get("connection_string")?.as_str()?.to_string();
        let psql_binary_path = postgres.get("psql_binary_path")?.as_str()?;

        Some(ToolPostgres {
            connection_string,
            psql_binary_path: PathBuf::from(psql_binary_path),
        })
    }

    async fn run_psql_command(&self, query: &str) -> Result<String, String> {
        let mut cmd = Command::new(&self.psql_binary_path);
        cmd.arg(&self.connection_string)
            .arg("ON_ERROR_STOP=1")
            .arg("-c")
            .arg(query);

        let output = cmd.output().await
            .map_err(|e| format!("Failed to execute psql command: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(format!("psql command failed: {}", String::from_utf8_lossy(&output.stderr)))
        }
    }
}

#[async_trait]
impl Tool for ToolPostgres {
    async fn tool_execute(
        &mut self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        command: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let query = match command.get("command") {
            Some(Value::String(v)) => v.clone(),
            Some(v) => return Err(format!("argument `command` is not a string: {:?}", v)),
            None => return Err("Command is empty".to_string()),
        };
        
        let result = self.run_psql_command(&query).await?;

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(serde_json::to_string(&result).unwrap()),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));
        Ok((true, results))
    }

    fn command_to_match_against_confirm_deny(
        &self,
        args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        let mut command_args = parse_command_args(args)?;
        command_args.insert(0, "psql".to_string());
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
    if parsed_args[0] == "psql" {
        parsed_args.remove(0);
    }

    Ok(parsed_args)
}