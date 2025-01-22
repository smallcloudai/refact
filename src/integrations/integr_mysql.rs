use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use async_trait::async_trait;

use crate::global_context::GlobalContext;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::ContextEnum;
use crate::call_validation::{ChatContent, ChatMessage, ChatUsage};
use crate::integrations::go_to_configuration_message;
use crate::tools::tools_description::Tool;
use crate::integrations::integr_abstract::{IntegrationCommon, IntegrationConfirmation, IntegrationTrait};


#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct SettingsMysql {
    #[serde(default)]
    pub mysql_binary_path: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub database: String,
}

#[derive(Default)]
pub struct ToolMysql {
    pub common:  IntegrationCommon,
    pub settings_mysql: SettingsMysql,
    pub config_path: String,
}

#[async_trait]
impl IntegrationTrait for ToolMysql {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn integr_settings_apply(&mut self, _gcx: Arc<ARwLock<GlobalContext>>, config_path: String, value: &serde_json::Value) -> Result<(), String> {
        match serde_json::from_value::<SettingsMysql>(value.clone()) {
            Ok(settings_mysql) => self.settings_mysql = settings_mysql,
            Err(e) => {
                tracing::error!("Failed to apply settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        match serde_json::from_value::<IntegrationCommon>(value.clone()) {
            Ok(x) => self.common = x,
            Err(e) => {
                tracing::error!("Failed to apply common settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        self.config_path = config_path;
        Ok(())
    }

    fn integr_settings_as_json(&self) -> Value {
        serde_json::to_value(&self.settings_mysql).unwrap()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    async fn integr_tools(&self, _integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        vec![Box::new(ToolMysql {
            common: self.common.clone(),
            settings_mysql: self.settings_mysql.clone(),
            config_path: self.config_path.clone(),
        })]
    }

    fn integr_schema(&self) -> &str
    {
        MYSQL_INTEGRATION_SCHEMA
    }
}

impl ToolMysql {
  async fn run_mysql_command(&self, query: &str) -> Result<String, String> {
      let mut mysql_command = self.settings_mysql.mysql_binary_path.clone();
      if mysql_command.is_empty() {
          mysql_command = "mysql".to_string();
      }
      let output_future = Command::new(mysql_command)
          .arg("-h")
          .arg(&self.settings_mysql.host)
          .arg("-P")
          .arg(&self.settings_mysql.port)
          .arg("-u")
          .arg(&self.settings_mysql.user)
          .arg(format!("-p{}", &self.settings_mysql.password))
          .arg(&self.settings_mysql.database)
          .arg("-e")
          .arg(query)
          .stdin(std::process::Stdio::null())
          .output();
      if let Ok(output) = tokio::time::timeout(tokio::time::Duration::from_millis(10_000), output_future).await {
          if output.is_err() {
              let err_text = format!("{}", output.unwrap_err());
              tracing::error!("mysql didn't work:\n{}\n{}", query, err_text);
              return Err(format!("{}, mysql failed:\n{}", go_to_configuration_message("mysql"), err_text));
          }
          let output = output.unwrap();
          if output.status.success() {
              Ok(String::from_utf8_lossy(&output.stdout).to_string())
          } else {
              // XXX: limit stderr, can be infinite
              let stderr_string = String::from_utf8_lossy(&output.stderr);
              tracing::error!("mysql didn't work:\n{}\n{}", query, stderr_string);
              Err(format!("{}, mysql failed:\n{}", go_to_configuration_message("mysql"), stderr_string))
          }
      } else {
          tracing::error!("mysql timed out:\n{}", query);
          Err("mysql command timed out".to_string())
      }
  }
}

#[async_trait]
impl Tool for ToolMysql {
    fn as_any(&self) -> &dyn std::any::Any { self }

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

        let result = self.run_mysql_command(&query).await?;

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
        Ok(format!("mysql {}", query))
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

pub const MYSQL_INTEGRATION_SCHEMA: &str = r#"
fields:
  host:
    f_type: string_long
    f_desc: "Connect to this host, for example 127.0.0.1 or docker container name."
    f_default: "127.0.0.1"
  port:
    f_type: string_short
    f_desc: "Which port to use."
    f_default: "3306"
  user:
    f_type: string_short
    f_placeholder: "$MYSQL_USER"
    smartlinks:
      - sl_label: "Open variables.yaml"
        sl_goto: "EDITOR:variables.yaml"
  password:
    f_type: string_short
    f_default: "$MYSQL_PASSWORD"
    smartlinks:
      - sl_label: "Open secrets.yaml"
        sl_goto: "EDITOR:secrets.yaml"
  database:
    f_type: string_short
    f_placeholder: "mysql"
  mysql_binary_path:
    f_type: string_long
    f_desc: "If it can't find a path to `mysql` you can provide it here, leave blank if not sure."
    f_placeholder: "mysql"
    f_label: "MYSQL Binary Path"
    f_extra: true
description: |
  The Mysql tool is for the AI model to call, when it wants to look at data inside your database, or make any changes.
  On this page you can also see Docker containers with Mysql servers.
  You can ask model to create a new container with a new database for you,
  or ask model to configure the tool to use an existing container with existing database.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
confirmation:
  ask_user_default: []
  deny_default: []
smartlinks:
  - sl_label: "Test"
    sl_chat:
      - role: "user"
        content: |
          🔧 The mysql tool should be visible now. To test the tool, list the tables available, briefly describe the tables and express
          happiness, and change nothing. If it doesn't work or the tool isn't available, go through the usual plan in the system prompt.
          The current config file is %CURRENT_CONFIG%.
    sl_enable_only_with_tool: true
  - sl_label: "Look at the project, help me set it up"
    sl_chat:
      - role: "user"
        content: |
          🔧 Your goal is to set up mysql client. Look at the project, especially files like "docker-compose.yaml" or ".env". Call tree() to see what files the project has.
          After that is completed, go through the usual plan in the system prompt.
          Keep MYSQL_HOST MYSQL_PORT MYSQL_USER MYSQL_PASSWORD MYSQL_DATABASE in variables.yaml so they can be reused by command line tools later.
docker:
  filter_label: ""
  filter_image: "mysql"
  new_container_default:
    image: "mysql:8.4"
    environment:
      MYSQL_DATABASE: "$MYSQL_DB"
      MYSQL_USER: "$MYSQL_USER"
      MYSQL_PASSWORD: "$MYSQL_PASSWORD"
    ports:
      - "3306:3306"
  smartlinks:
    - sl_label: "Add Database Container"
      sl_chat:
        - role: "user"
          content: |
            🔧 Your job is to create a mysql container, using the image and environment from new_container_default section in the current config file: %CURRENT_CONFIG%. Follow the system prompt.
  smartlinks_for_each_container:
    - sl_label: "Use for integration"
      sl_chat:
        - role: "user"
          content: |
            🔧 Your job is to modify mysql connection config in the current file to match the variables from the container, use docker tool to inspect the container if needed. Current config file: %CURRENT_CONFIG%.
"#;
