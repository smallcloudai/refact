use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::ContextEnum;
use crate::call_validation::{ChatContent, ChatMessage, ChatUsage};
use crate::tools::tools_description::Tool;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_yaml;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex as AMutex;


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct IntegrationPostgres {
    pub psql_binary_path: Option<String>,
    pub connection_string: String,
}

pub struct ToolPostgres {
    integration_postgres: IntegrationPostgres,
}

impl ToolPostgres {
    pub fn new_if_configured(integrations_value: &serde_yaml::Value) -> Option<Self> {
        let integration_postgres_value = integrations_value.get("postgres")?;

        let integration_postgres = serde_yaml::from_value::<IntegrationPostgres>(integration_postgres_value.clone()).or_else(|e| {
            tracing::error!("postgres integration exists, but there is a syntax error in yaml:\n{}", e);
            Err(e)
        }).ok()?;

        Some(Self { integration_postgres })
    }

    async fn run_psql_command(&self, query: &str) -> Result<String, String> {
        let psql_command = self.integration_postgres.psql_binary_path.as_deref().unwrap_or("psql");
        let output_future = Command::new(psql_command)
            .arg(&self.integration_postgres.connection_string)
            .arg("ON_ERROR_STOP=1")
            .arg("-c")
            .arg(query)
            .output();
        if let Ok(output) = tokio::time::timeout(tokio::time::Duration::from_millis(10_000), output_future).await {
            if output.is_err() {
                let err_text = format!("{}", output.unwrap_err());
                tracing::error!("psql didn't work:\n{}\n{}\n{}", self.integration_postgres.connection_string, query, err_text);
                return Err(format!("psql failed:\n{}", err_text));
            }
            let output = output.unwrap();
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                // XXX: limit stderr, can be infinite
                let stderr_string = String::from_utf8_lossy(&output.stderr);
                tracing::error!("psql didn't work:\n{}\n{}\n{}", self.integration_postgres.connection_string, query, stderr_string);
                Err(format!("psql failed:\n{}", stderr_string))
            }
        } else {
            tracing::error!("psql timed out:\n{}\n{}", self.integration_postgres.connection_string, query);
            Err("psql command timed out".to_string())
        }
    }
}

#[async_trait]
impl Tool for ToolPostgres {
    async fn tool_execute(
        &mut self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let query = match args.get("query") {
            Some(Value::String(v)) => v.clone(),
            Some(v) => return Err(format!("argument `query` is not a string: {:?}", v)),
            None => return Err("no `query` argument found".to_string()),
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
        let query = match args.get("query") {
            Some(Value::String(v)) => v.clone(),
            Some(v) => return Err(format!("argument `query` is not a string: {:?}", v)),
            None => return Err("no `query` argument found".to_string()),
        };
        Ok(format!("psql {}", query))
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
