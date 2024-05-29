use std::sync::Arc;
use async_trait::async_trait;
use tokio::process::Command;
use tokio::sync::Mutex as AMutex;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::execute_at::AtCommandMember;
use crate::call_validation::{ChatMessage, ContextEnum};


pub struct AtExecuteCommand {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtExecuteCommand {
    pub fn new() -> Self {
        AtExecuteCommand {
            params: vec![],
        }
    }
}

pub async fn execute_cmd(command: &String) -> Result<(String, String), String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok((stdout, stderr))
}

#[async_trait]
impl AtCommand for AtExecuteCommand {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn execute(&self, _ccx: &mut AtCommandsContext, _cmd: &mut AtCommandMember, args: &mut Vec<AtCommandMember>) -> Result<(Vec<ContextEnum>, String), String> {
        // TODO: fixme
        let command = args.iter().map(|x|x.text.clone()).collect::<Vec<_>>().join(" ");
        if command.is_empty() {
            return Err("No args provided".to_string());
        }
        
        let (stdout, stderr) = execute_cmd(&command).await?;
        
        let chat_message = ChatMessage::new(
            "assistant".to_string(),
            format!("{}{}", stdout, stderr),
        );
        let text = format!("Executed: {}", command);
        Ok((vec![ContextEnum::ChatMessage(chat_message)], text))
    }

    fn depends_on(&self) -> Vec<String> {
        vec![]
    }
}