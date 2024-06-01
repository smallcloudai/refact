use std::sync::Arc;
use async_trait::async_trait;
use tokio::process::Command;
use tokio::sync::Mutex as AMutex;
use tokio::time::{timeout, Duration};

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

pub async fn execute_cmd(command: &String, timeout_secs: usize) -> Result<(String, String), String> {
    let timeout_duration = Duration::from_secs(timeout_secs as u64);

    let output = timeout(timeout_duration, Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output())
        .await
        .map_err(|_| "Command timed out".to_string())?
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
        let mut new_args = vec![];
        for a in args.iter() {
            if a.text.is_empty() { break; }
            new_args.push(a.clone());
        }
        args.clear();
        args.extend(new_args);

        let command = args.iter().map(|x|x.text.clone()).collect::<Vec<_>>().join(" ");
        if command.is_empty() {
            return Err("No args provided".to_string());
        }

        let (stdout, stderr) = execute_cmd(&command, 300).await?;

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

pub struct AtExecuteCustCommand {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
    pub command: String,
    pub timeout: usize,
    pub postprocess: String,
}

#[allow(dead_code)]    // not used for custom at-commands
impl AtExecuteCustCommand {
    pub fn new(command: String, timeout: usize, postprocess: String) -> Self {
        AtExecuteCustCommand {
            params: vec![],
            command,
            timeout,
            postprocess,
        }
    }
}

#[async_trait]
impl AtCommand for AtExecuteCustCommand {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }
    async fn execute(&self, ccx: &mut AtCommandsContext, cmd: &mut AtCommandMember, args: &mut Vec<AtCommandMember>) -> Result<(Vec<ContextEnum>, String), String> {
        args.clear();
        if ccx.is_preview {
            cmd.reason = Some("does not run in preview".to_string());
            return Ok((vec![], "preview".to_string()));
        }
        let (stdout, stderr) = execute_cmd(&self.command, self.timeout).await?;
        let chat_message = ChatMessage::new(
            "assistant".to_string(),
            format!("{}{}", stdout, stderr),
        );
        let text = format!("Executed: {}", self.command);
        // TODO: is err, update highlight to ok=false
        Ok((vec![ContextEnum::ChatMessage(chat_message)], text))
    }
}
