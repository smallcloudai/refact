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
use schemars::JsonSchema;
use tokio::process::Command;
use tokio::sync::Mutex as AMutex;
use crate::integrations::integr::{json_schema, Integration};


#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema, Default)]
pub struct IntegrationPostgres {
    #[schemars(description = "Path to the psql binary.")]
    pub psql_binary_path: Option<String>,
    #[schemars(description = "Connection string for the PSQL database.")]
    pub connection_string: String,
}

#[derive(Default)]
pub struct ToolPostgres {
    pub integration_postgres: IntegrationPostgres,
}

impl Integration for ToolPostgres {
    fn name(&self) -> String {
        "postgres".to_string()
    }

    fn update_from_json(&mut self, value: &Value) -> Result<(), String> {
        let integration_postgres = serde_json::from_value::<IntegrationPostgres>(value.clone())
            .map_err(|e|e.to_string())?;
        self.integration_postgres = integration_postgres;
        Ok(())
    }

    fn from_yaml_validate_to_json(&self, value: &serde_yaml::Value) -> Result<Value, String> {
        let integration_github = serde_yaml::from_value::<IntegrationPostgres>(value.clone()).map_err(|e| {
            let location = e.location().map(|loc| format!(" at line {}, column {}", loc.line(), loc.column())).unwrap_or_default();
            format!("{}{}", e.to_string(), location)
        })?;
        serde_json::to_value(&integration_github).map_err(|e| e.to_string())
    }

    fn to_tool(&self) -> Box<dyn Tool + Send> {
        Box::new(ToolPostgres {integration_postgres: self.integration_postgres.clone()}) as Box<dyn Tool + Send>
    }

    fn to_json(&self) -> Result<Value, String> {
        serde_json::to_value(&self.integration_postgres).map_err(|e| e.to_string())
    }

    fn to_schema_json(&self) -> Value {
        json_schema::<IntegrationPostgres>().unwrap()
    }
    fn default_value(&self) -> String { DEFAULT_POSTGRES_INTEGRATION_YAML.to_string() }
    fn icon_link(&self) -> String { "https://cdn-icons-png.flaticon.com/512/5968/5968342.png".to_string() }
}

impl ToolPostgres {

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

const DEFAULT_POSTGRES_INTEGRATION_YAML: &str = r#"
# Postgres database

# psql_binary_path: "/path/to/psql"  # Uncomment to set a custom path for the psql binary, defaults to "psql"
# connection_string: "postgresql://username:password@localhost/dbname"  # To get a connection string, check out https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING
"#;
