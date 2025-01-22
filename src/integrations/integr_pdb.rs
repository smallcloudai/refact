use std::any::Any;
use std::path::PathBuf;
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
use crate::call_validation::{ContextEnum, ChatMessage, ChatContent, ChatUsage};
use crate::files_correction::get_active_project_path;
use crate::integrations::sessions::{IntegrationSession, get_session_hashmap_key};
use crate::global_context::GlobalContext;
use crate::integrations::integr_abstract::{IntegrationCommon, IntegrationConfirmation, IntegrationTrait};
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam};
use crate::integrations::process_io_utils::{first_n_chars, last_n_chars, last_n_lines, write_to_stdin_and_flush, blocking_read_until_token_or_timeout};


const SESSION_TIMEOUT_AFTER_INACTIVITY: Duration = Duration::from_secs(30 * 60);
const PDB_TOKEN: &str = "(Pdb)";

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct SettingsPdb {
    pub python_path: String,
}

#[derive(Default)]
pub struct ToolPdb {
    pub common:  IntegrationCommon,
    pub settings_pdb: SettingsPdb,
    pub config_path: String,
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

#[async_trait]
impl IntegrationTrait for ToolPdb {
    fn as_any(&self) -> &dyn Any { self }

    async fn integr_settings_apply(&mut self, _gcx: Arc<ARwLock<GlobalContext>>, config_path: String, value: &serde_json::Value) -> Result<(), String> {
        match serde_json::from_value::<SettingsPdb>(value.clone()) {
            Ok(settings_pdb) => {
                info!("PDB settings applied: {:?}", settings_pdb);
                self.settings_pdb = settings_pdb;
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
        serde_json::to_value(&self.settings_pdb).unwrap_or_default()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    async fn integr_tools(&self, _integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        vec![Box::new(ToolPdb {
            common: self.common.clone(),
            settings_pdb: self.settings_pdb.clone(),
            config_path: self.config_path.clone(),
        })]
    }

    fn integr_schema(&self) -> &str { PDB_INTEGRATION_SCHEMA }
}

#[async_trait]
impl Tool for ToolPdb {
    fn as_any(&self) -> &dyn Any { self }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let (command, workdir_maybe) = parse_args(args)?;
        let mut command_args = split_command(&command)?;

        let (gcx, chat_id) = {
            let ccx_lock = ccx.lock().await;
            (ccx_lock.global_context.clone(), ccx_lock.chat_id.clone())
        };

        let session_hashmap_key = get_session_hashmap_key("pdb", &chat_id);
        let mut python_command = self.settings_pdb.python_path.clone();
        if python_command.is_empty() {
            python_command = "python3".to_string();
        }
        if command_args.windows(2).any(|w| w == ["-m", "pdb"]) {
            let output = start_pdb_session(&python_command, &mut command_args, &session_hashmap_key, &workdir_maybe, gcx.clone(), 10).await?;
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

    fn command_to_match_against_confirm_deny(
        &self,
        args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        let (command, _) = parse_args(args)?;
        let command_args = split_command(&command)?;
        Ok(command_args.join(" "))
    }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "pdb".to_string(),
            agentic: true,
            experimental: false,
            description: "Python debugger for inspecting variables and exploring what the program really does. This tool executes only one command at a time. Start with python -m pdb ...".to_string(),
            parameters: vec![
                ToolParam {
                    name: "command".to_string(),
                    param_type: "string".to_string(),
                    description: "Examples: 'python -m pdb script.py', 'break module_name.function_name', 'break 10', 'continue', 'print(variable_name)', 'list', 'quit'".to_string(),
                },
                ToolParam {
                    name: "workdir".to_string(),
                    param_type: "string".to_string(),
                    description: "Working directory for the command, needed to start a pdb session from a relative path.".to_string(),
                },
            ],
            parameters_required: vec!["command".to_string()],
        }
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

fn parse_args(args: &HashMap<String, Value>) -> Result<(String, Option<PathBuf>), String> {
    let command = match args.get("command") {
        Some(Value::String(s)) => s.to_string(),
        Some(v) => return Err(format!("argument `command` is not a string: {:?}", v)),
        None => return Err("Missing argument `command`".to_string()),
    };
    let workdir_maybe = match args.get("workdir") {
        Some(Value::String(s)) => {
            if s.is_empty() {
                None
            } else {
                let workdir = crate::files_correction::to_pathbuf_normalize(s);
                if !workdir.exists() {
                    return Err("Workdir doesn't exist".to_string());
                } else {
                    Some(workdir)
                }
            }
        },
        Some(v) => return Err(format!("argument `workdir` is not a string: {:?}", v)),
        None => None
    };
    Ok((command, workdir_maybe))
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
    workdir_maybe: &Option<PathBuf>,
    gcx: Arc<ARwLock<GlobalContext>>,
    timeout_seconds: u64,
) -> Result<String, String> {
    if !(command_args.len() >= 3 && command_args[0] == "python" && command_args[1] == "-m" && command_args[2] == "pdb") {
        return Err("Usage: python -m pdb ... To use a different Python environment, use a path to python binary.".to_string());
    }
    command_args.remove(0);

    info!("Starting pdb session with command: {} {:?}", python_command, command_args);
    let mut process_command = Command::new(python_command);
    process_command.args(&command_args[..])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if let Some(workdir) = workdir_maybe {
        process_command.current_dir(workdir);
    } else if let Some(project_path) = get_active_project_path(gcx.clone()).await {
        process_command.current_dir(project_path);
    } else {
        tracing::warn!("no working directory, using whatever directory this binary is run :/");
    }

    let mut process = process_command.spawn().map_err(|e| {
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

const PDB_INTEGRATION_SCHEMA: &str = r#"
fields:
  python_path:
    f_type: string_long
    f_desc: "Path to the Python interpreter. Leave empty to use the default 'python3' command."
    f_placeholder: "/opt/homebrew/bin/python3"
    f_label: "Python Interpreter Path"
description: |
  The PDB integration allows interaction with the Python debugger for inspecting variables and exploring program execution.
  It provides functionality for debugging Python scripts and applications.
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
          🔧 The pdb tool should be visible now. To test the tool, start a debugging session for a simple Python script, set a breakpoint, and inspect some variables.
          If it doesn't work or the tool isn't available, go through the usual plan in the system prompt.
    sl_enable_only_with_tool: true
"#;
