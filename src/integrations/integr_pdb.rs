use std::any::Any;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::SystemTime;
use std::fmt::Debug;
use std::future::Future;
use serde_json::Value;
use tokio::io::BufReader;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tokio::process::{Command, Child, ChildStdin, ChildStdout, ChildStderr};
use tokio::time::Duration;
use async_trait::async_trait;
use tracing::{error, info};
use serde::{Deserialize, Serialize};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ContextEnum, ChatMessage, ChatContent};
use crate::integrations::sessions::{IntegrationSession, get_session_hashmap_key};
use crate::global_context::GlobalContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam};
use crate::integrations::process_io_utils::{first_n_chars, last_n_chars, last_n_lines, write_to_stdin_and_flush, blocking_read_until_token_or_timeout};

const SESSION_TIMEOUT_AFTER_INACTIVITY: Duration = Duration::from_secs(30 * 60);
const PDB_TOKEN: &str = "(Pdb)";

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct IntegrationPdb {
    pub python_path: Option<String>,
}
pub struct ToolPdb {
    integration_pdb: IntegrationPdb,
}

pub struct PdbSession {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,
    last_usage_ts: u64,
}

impl Drop for PdbSession {
    fn drop(&mut self) {
        self.process.start_kill().map_err(|e| error!("Failed to kill process: {}", e)).ok();
    }
}

impl IntegrationSession for PdbSession
{
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn is_expired(&self) -> bool {
        let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        self.last_usage_ts + SESSION_TIMEOUT_AFTER_INACTIVITY.as_secs() < current_time
    }

    fn try_stop(&mut self) -> Box<dyn Future<Output = String> + Send + '_> {
        Box::new(async { "".to_string() })
    }
}

impl ToolPdb {
    pub fn new_from_yaml(v: &serde_yaml::Value) -> Result<Self, String> {
        let integration_pdb = serde_yaml::from_value::<IntegrationPdb>(v.clone()).map_err(|e| {
            let location = e.location().map(|loc| format!(" at line {}, column {}", loc.line(), loc.column())).unwrap_or_default();
            format!("{}{}", e.to_string(), location)
        })?;
        Ok(Self { integration_pdb })
    }
}

#[async_trait]
impl Tool for ToolPdb {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let command = parse_command(args)?;
        let mut command_args = split_command(&command)?;

        let (gcx, chat_id) = {
            let ccx_lock = ccx.lock().await;
            (ccx_lock.global_context.clone(), ccx_lock.chat_id.clone())
        };

        let session_hashmap_key = get_session_hashmap_key("pdb", &chat_id);
        let python_command = self.integration_pdb.python_path.clone().unwrap_or_else(|| "python3".to_string());

        if command_args.windows(2).any(|w| w == ["-m", "pdb"]) {
            let output = start_pdb_session(&python_command, &mut command_args, &session_hashmap_key, gcx.clone(), 10).await?;
            return Ok(tool_answer(output, tool_call_id));
        }
        
        let command_session = {
            let gcx_locked = gcx.read().await;
            gcx_locked.integration_sessions.get(&session_hashmap_key)
                .ok_or("There is no active pdb session in this chat, you can open it by running pdb(\"python -m pdb my_script.py\")")?
                .clone()
        };
    
        let mut command_session_locked = command_session.lock().await;
        let mut pdb_session = command_session_locked.as_any_mut().downcast_mut::<PdbSession>()
            .ok_or("Failed to downcast to PdbSession")?;

        let output = match command_args[0].as_str() {
            "kill" => {
                let mut gcx_locked = gcx.write().await;
                gcx_locked.integration_sessions.remove(&session_hashmap_key);
                "Pdb session has been killed".to_string() 
            },
            "wait" => {
                if command_args.len() < 2 {
                    return Err("Argument `n_seconds` in `wait n_seconds` is missing".to_string());
                }
                let timeout_seconds = command_args[1].parse::<u64>().map_err(|_| "Argument `n_seconds` in `wait n_seconds` is not a number".to_string())?;
                interact_with_pdb("", &mut pdb_session, &session_hashmap_key, gcx.clone(), timeout_seconds).await?
            }
            _ => { interact_with_pdb(&command, &mut pdb_session, &session_hashmap_key, gcx.clone(), 10).await? }
        };
        Ok(tool_answer(output, tool_call_id))
    }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "pdb".to_string(),
            agentic: true,
            experimental: true,
            description: "Python debugger for inspecting variables and exploring what the program really does. This tool executes only one command at a time. Start with python -m pdb ...".to_string(),
            parameters: vec![
                ToolParam {
                    name: "command".to_string(),
                    param_type: "string".to_string(),
                    description: "Examples: 'python -m pdb script.py', 'break module_name.function_name', 'break 10', 'continue', 'print(variable_name)', 'list', 'quit'".to_string(),
                },
            ],
            parameters_required: vec!["command".to_string()],
        }
    }

    fn command_to_match_against_confirm_deny(
        &self,
        args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        let commmand = parse_command(args)?;
        let command_args = split_command(&commmand)?;
        Ok(command_args.join(" "))
    }
}

fn parse_command(args: &HashMap<String, Value>) -> Result<String, String> {
    match args.get("command") {
        Some(Value::String(s)) => Ok(s.to_string()),
        Some(v) => Err(format!("argument `command` is not a string: {:?}", v)),
        None => Err("Missing argument `command`".to_string()),
    }
}

fn split_command(command: &str) -> Result<Vec<String>, String> {
    let parsed_args = shell_words::split(command).map_err(|e| e.to_string())?;
    if parsed_args.is_empty() {
        return Err("Parsed command is empty".to_string());
    }

    Ok(parsed_args)
}

async fn start_pdb_session(
    python_command: &String, 
    command_args: &mut Vec<String>, 
    session_hashmap_key: &String, 
    gcx: Arc<ARwLock<GlobalContext>>, 
    timeout_seconds: u64,
) -> Result<String, String> {
    if !(command_args.len() >= 3 && command_args[0] == "python" && command_args[1] == "-m" && command_args[2] == "pdb") {
        return Err("Usage: python -m pdb ... To use a different Python environment, set `python_path` in `integrations.yaml`.".to_string());
    }
    command_args.remove(0);

    info!("Starting pdb session with command: {} {:?}", python_command, command_args);
    let mut process = Command::new(python_command)
        .args(&command_args[..])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            error!("Failed to start pdb process: {}", e);
            e.to_string()
        })?;

    let stdin = process.stdin.take().ok_or("Failed to open stdin for pdb process")?;
    let stdout = BufReader::new(process.stdout.take().ok_or("Failed to open stdout for pdb process")?);
    let stderr = BufReader::new(process.stderr.take().ok_or("Failed to open stderr for pdb process")?);
    let mut pdb_session = PdbSession {process, stdin, stdout, stderr, last_usage_ts: 0};

    let output = interact_with_pdb("", &mut pdb_session, &session_hashmap_key, gcx.clone(), timeout_seconds).await?;

    let command_session: Box<dyn IntegrationSession> = Box::new(pdb_session);
    {
        let mut gcx_locked = gcx.write().await;
        gcx_locked.integration_sessions.insert(
            session_hashmap_key.clone(), Arc::new(AMutex::new(command_session))
        );
    }
    Ok(output)
}

async fn interact_with_pdb(
    input_command: &str,
    pdb_session: &mut PdbSession,
    session_hashmap_key: &String,
    gcx: Arc<ARwLock<GlobalContext>>,
    timeout_seconds: u64,
) -> Result<String, String> {
    if !input_command.is_empty() {
        let (prev_output, prev_error, _) = blocking_read_until_token_or_timeout(
            &mut pdb_session.stdout, &mut pdb_session.stderr, 100, PDB_TOKEN).await?;
        if !prev_output.is_empty() || !prev_error.is_empty() {
            return Err(format!("There is leftover output from previous commands, run pdb tool again with \"wait n_seconds\" to wait for it or \"kill\" command to kill the session.\nstdout:\n{}\nstderr:\n{}", prev_output, prev_error));
        }
    }
    
    let (output_main_command, error_main_command) = send_command_and_get_output_and_error(
        pdb_session, input_command, session_hashmap_key, gcx.clone(), timeout_seconds * 1000, true).await?;
    let (output_list, error_list) = send_command_and_get_output_and_error(
        pdb_session, "list", session_hashmap_key, gcx.clone(), 2000, false).await?;
    let (output_where, error_where) = send_command_and_get_output_and_error(
        pdb_session, "where", session_hashmap_key, gcx.clone(), 2000, false).await?;
    let (output_locals, error_locals) = send_command_and_get_output_and_error(
        pdb_session, "p {k: __import__('reprlib').repr(v) for k, v in locals().items() if not k.startswith('__')}", session_hashmap_key, gcx.clone(), 5000, false).await?;

    pdb_session.last_usage_ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(format_all_output(&output_main_command, &error_main_command, &output_list, &error_list,
        &output_where, &error_where, &output_locals, &error_locals))
}

async fn send_command_and_get_output_and_error(
    pdb_session: &mut PdbSession, 
    input_command: &str, 
    session_hashmap_key: &str, 
    gcx: Arc<ARwLock<GlobalContext>>,
    timeout_ms: u64,
    ask_for_continuation_if_timeout: bool,
) -> Result<(String, String), String> {
    if !input_command.is_empty() {
        write_to_stdin_and_flush(&mut pdb_session.stdin, input_command).await?;
    }
    let (output, mut error, have_the_token) = blocking_read_until_token_or_timeout(
        &mut pdb_session.stdout, &mut pdb_session.stderr, timeout_ms, PDB_TOKEN).await?;

    let exit_status = pdb_session.process.try_wait().map_err(|e| e.to_string())?;
    if let Some(exit_status) = exit_status {
        gcx.write().await.integration_sessions.remove(session_hashmap_key);
        return Err(format!("Pdb process exited with status: {:?}", exit_status));
    }

    if !have_the_token {
        let mut timeout_error = format!("Command {} timed out after {} seconds.", input_command, timeout_ms / 1000);
        if ask_for_continuation_if_timeout {
            timeout_error = timeout_error + " Call pdb tool again with \"wait n_seconds\" command to wait for n seconds for the process to finish, or \"kill\" command to forcedly stop it.";
            return Err(timeout_error);
        }
        error += &format!("\n{timeout_error}");
    }

    Ok((output, error))
}

fn tool_answer(output: String, tool_call_id: &String) -> (bool, Vec<ContextEnum>)
{
    (false, vec![
        ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(output),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })
    ])
}


fn format_all_output(output_main_command: &str, error_main_command: &str, output_list: &str, error_list: &str, output_where: &str, error_where: &str, output_locals: &str, error_locals: &str) -> String
{
    format!(
        "Command output:\n{}\n{}\nCurrent code section:\n{}{}\nStack trace:\n{}{}\nLocal variables:\n{}{}",
        last_n_chars(output_main_command, 5000),
        format_error("Command error", &last_n_chars(error_main_command, 5000)),
        output_list.replace(PDB_TOKEN, ""),
        format_error("list error", error_list),
        last_n_lines(&output_where.replace(PDB_TOKEN, ""), 8),
        format_error("where error", error_where),
        first_n_chars(&output_locals.replace(PDB_TOKEN, ""), 1000),
        format_error("locals error", error_locals),
    )
}

fn format_error(error_title: &str, error: &str) -> String
{
    if !error.is_empty() {
        format!("{}:\n{}\n", error_title, error)
    } else {
        "".to_string()
    }
}
