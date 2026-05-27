use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::Mutex as AMutex;
use tracing::info;

#[cfg(not(target_os = "windows"))]
use shell_escape::escape;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ContextEnum};
use crate::custom_error::YamlError;
use crate::exec::{
    generate_short_description, sanitize_short_description, ExecMode, ExecOutputStream,
    ExecOwnerMeta, ExecProcessSnapshot, ExecRawOutput, ExecReadResult, ExecSpawnRequest,
    ExecStatus,
};
use crate::global_context::GlobalContext;
use crate::integrations::integr_abstract::{
    IntegrationCommon, IntegrationConfirmation, IntegrationTrait,
};
use crate::integrations::utils::{
    deserialize_str_to_num, deserialize_str_to_opt_num, serialize_num_to_str,
    serialize_opt_num_to_str,
};
use crate::postprocessing::pp_command_output::{output_mini_postprocessing, OutputFilter};
use crate::tools::tools_description::{Tool, ToolDesc, ToolSource, ToolSourceType};
use refact_buddy_core::user_action::UserAction;

const CMDLINE_TRANSCRIPT_MAX_BYTES: usize = 2 * 1024 * 1024;

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct CmdlineParam {
    pub name: String,
    #[serde(rename = "type", default = "CmdlineParam::default_type")]
    pub param_type: String,
    #[serde(default)]
    pub description: String,
}

impl CmdlineParam {
    fn default_type() -> String {
        "string".to_string()
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct CmdlineToolConfig {
    pub command: String,
    pub command_workdir: String,

    pub description: String,
    pub parameters: Vec<CmdlineParam>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters_required: Option<Vec<String>>,

    // blocking
    #[serde(default)]
    pub timeout: String,
    #[serde(default)]
    pub output_filter: OutputFilter,

    // background
    #[serde(
        default,
        serialize_with = "serialize_opt_num_to_str",
        deserialize_with = "deserialize_str_to_opt_num"
    )]
    pub startup_wait_port: Option<u16>,
    #[serde(
        default = "_default_startup_wait",
        serialize_with = "serialize_num_to_str",
        deserialize_with = "deserialize_str_to_num"
    )]
    pub startup_wait: u64,
    #[serde(default)]
    pub startup_wait_keyword: String,
}

fn _default_startup_wait() -> u64 {
    10
}

#[derive(Default)]
pub struct ToolCmdline {
    pub common: IntegrationCommon,
    pub name: String,
    pub cfg: CmdlineToolConfig,
    pub config_path: String,
}

#[async_trait]
impl IntegrationTrait for ToolCmdline {
    async fn integr_settings_apply(
        &mut self,
        _gcx: Arc<GlobalContext>,
        config_path: String,
        value: &serde_json::Value,
    ) -> Result<(), serde_json::Error> {
        self.cfg = serde_json::from_value(value.clone())?;
        self.common = serde_json::from_value(value.clone())?;
        self.config_path = config_path;
        Ok(())
    }

    fn integr_settings_as_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    async fn integr_tools(
        &self,
        integr_name: &str,
    ) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        vec![Box::new(ToolCmdline {
            common: self.common.clone(),
            name: integr_name.to_string(),
            cfg: self.cfg.clone(),
            config_path: self.config_path.clone(),
        })]
    }

    fn integr_schema(&self) -> &str {
        CMDLINE_INTEGRATION_SCHEMA
    }
}

#[cfg(target_os = "windows")]
fn powershell_escape(s: &str) -> String {
    let mut needs_escape = s.is_empty();
    for ch in s.chars() {
        match ch {
            ' ' | '"' | '\'' | '$' | '`' | '[' | ']' | '{' | '}' | '(' | ')' | '@' | '&' | '#'
            | ',' | ';' | '.' | '\t' | '\n' | '|' | '<' | '>' | '\\' => {
                needs_escape = true;
                break;
            }
            _ => {}
        }
    }

    if !needs_escape {
        return s.to_string();
    }

    let mut es = String::with_capacity(s.len() + 2);
    es.push('"');

    for ch in s.chars() {
        match ch {
            '"' => es.push_str("`\""),
            '$' => es.push_str("`$"),
            '`' => es.push_str("``"),
            '\t' => es.push_str("`t"),
            '\n' => es.push_str("`n"),
            '\\' => es.push_str("\\"),
            _ => es.push(ch),
        }
    }

    es.push('"');
    es
}

pub fn replace_args(x: &str, args_str: &HashMap<String, String>) -> String {
    let mut result = x.to_string();
    for (key, value) in args_str {
        let escaped_value = if value == "" {
            // special case for an empty paramter, we want it empty as replacement, rather than escaped empty string ""
            "".to_string()
        } else {
            #[cfg(target_os = "windows")]
            let x = powershell_escape(value);
            #[cfg(not(target_os = "windows"))]
            let x = escape(std::borrow::Cow::from(value.as_str())).to_string();
            x
        };
        result = result.replace(&format!("%{}%", key), &escaped_value);
    }
    result
}

pub fn format_output(stdout_out: &str, stderr_out: &str) -> String {
    let mut out = String::new();
    if !stdout_out.is_empty() && stderr_out.is_empty() {
        // special case: just clean output, nice
        out.push_str(&format!("{}\n\n", stdout_out));
    } else {
        if !stdout_out.is_empty() {
            out.push_str(&format!("STDOUT\n```\n{}```\n\n", stdout_out));
        }
        if !stderr_out.is_empty() {
            out.push_str(&format!("STDERR\n```\n{}```\n\n", stderr_out));
        }
        if stdout_out.is_empty() && stderr_out.is_empty() {
            out.push_str(&format!("Nothing in STDOUT/STDERR\n\n"));
        }
    }
    out
}

fn resolve_cmdline_workdir(command_workdir: &str, project_dirs: &[PathBuf]) -> Option<PathBuf> {
    if command_workdir.is_empty() {
        if let Some(first_project_dir) = project_dirs.first() {
            return Some(dunce::simplified(first_project_dir).to_path_buf());
        }
        tracing::warn!("no working directory, using whatever directory this binary is run :/");
        None
    } else {
        let path = PathBuf::from(command_workdir);
        Some(dunce::simplified(path.as_path()).to_path_buf())
    }
}

fn collect_exec_output(read: &ExecReadResult) -> (String, String) {
    let mut stdout = String::new();
    let mut stderr = String::new();
    for chunk in &read.chunks {
        match chunk.stream {
            ExecOutputStream::Stdout | ExecOutputStream::Combined => stdout.push_str(&chunk.text),
            ExecOutputStream::Stderr => stderr.push_str(&chunk.text),
        }
    }
    (stdout, stderr)
}

fn collect_foreground_output(
    read: &ExecReadResult,
    raw_output: Option<&ExecRawOutput>,
) -> (String, String) {
    raw_output
        .map(|raw| (raw.stdout.clone(), raw.stderr.clone()))
        .unwrap_or_else(|| collect_exec_output(read))
}

fn exec_status_label(status: &ExecStatus) -> &'static str {
    match status {
        ExecStatus::Starting => "starting",
        ExecStatus::Running => "running",
        ExecStatus::Exited { .. } => "exited",
        ExecStatus::Failed { .. } => "failed",
        ExecStatus::Killed => "killed",
        ExecStatus::TimedOut => "timed_out",
    }
}

fn exec_exit_code(status: &ExecStatus) -> Option<i32> {
    match status {
        ExecStatus::Exited { exit_code } => *exit_code,
        ExecStatus::Starting
        | ExecStatus::Running
        | ExecStatus::Failed { .. }
        | ExecStatus::Killed
        | ExecStatus::TimedOut => None,
    }
}

fn append_status_line(
    out: &mut String,
    status: &ExecStatus,
    duration: Duration,
    timeout_secs: u64,
) {
    match status {
        ExecStatus::Exited { exit_code } => out.push_str(&format!(
            "The command was running {:.3}s, finished with exit code {}\n",
            duration.as_secs_f64(),
            exit_code.unwrap_or_default()
        )),
        ExecStatus::Killed => out.push_str(&format!(
            "⚠️ The command was interrupted by user after {:.3}s (process killed). Output above may be incomplete.\n",
            duration.as_secs_f64()
        )),
        ExecStatus::TimedOut => out.push_str(&format!(
            "⚠️ The command timed out after {} seconds (process killed). Output above may be incomplete.\n",
            timeout_secs
        )),
        ExecStatus::Failed { message } => out.push_str(&format!(
            "⚠️ The command failed after {:.3}s: {}\n",
            duration.as_secs_f64(),
            message
        )),
        ExecStatus::Starting | ExecStatus::Running => out.push_str(&format!(
            "⚠️ The command did not reach a terminal state after {:.3}s (status: {}).\n",
            duration.as_secs_f64(),
            exec_status_label(status)
        )),
    }
}

fn tool_failed_for_status(status: &ExecStatus) -> Option<bool> {
    match status {
        ExecStatus::Failed { .. } | ExecStatus::Killed | ExecStatus::TimedOut => Some(true),
        ExecStatus::Starting | ExecStatus::Running | ExecStatus::Exited { .. } => None,
    }
}

fn exec_extra(
    snapshot: &ExecProcessSnapshot,
    duration: Duration,
) -> serde_json::Map<String, Value> {
    let mut extra = serde_json::Map::new();
    extra.insert(
        "exec".to_string(),
        json!({
            "process_id": snapshot.meta.process_id.as_str(),
            "status": exec_status_label(&snapshot.status),
            "exit_code": exec_exit_code(&snapshot.status),
            "duration_ms": duration.as_millis() as u64,
            "short_description": snapshot.meta.short_description,
        }),
    );
    extra
}

fn cmdline_short_description(name: &str, description: &str, command: &str) -> String {
    let raw = match (name.trim().is_empty(), description.trim().is_empty()) {
        (false, false) => format!("{}: {}", name, description),
        (false, true) => name.to_string(),
        (true, false) => description.to_string(),
        (true, true) => String::new(),
    };
    let description = sanitize_short_description(&raw);
    if description.is_empty() {
        generate_short_description(command, &ExecMode::Foreground)
    } else {
        description
    }
}

pub async fn execute_blocking_command(
    command: &str,
    cfg: &CmdlineToolConfig,
    command_workdir: &str,
    env_variables: &HashMap<String, String>,
    project_dirs: Vec<PathBuf>,
    exec_registry: &crate::exec::ExecRegistry,
    owner: ExecOwnerMeta,
    abort_flag: Arc<AtomicBool>,
    short_description: String,
) -> Result<(String, ExecProcessSnapshot, Duration), String> {
    info!("EXEC workdir {:?}:\n{:?}", command_workdir, command);

    let timeout_secs = cfg.timeout.parse::<u64>().unwrap_or(10);
    let cwd = resolve_cmdline_workdir(command_workdir, &project_dirs);
    let mut request = ExecSpawnRequest::foreground(command.to_string())
        .with_timeout(Duration::from_secs(timeout_secs))
        .with_env_map(env_variables.clone())
        .with_owner(owner)
        .with_transcript_limit(CMDLINE_TRANSCRIPT_MAX_BYTES)
        .with_short_description(short_description)
        .with_abort_flag(abort_flag);
    if let Some(cwd) = cwd {
        request = request.with_cwd(cwd);
    }
    tracing::info!("command: {}", command);

    let started = tokio::time::Instant::now();
    let result = exec_registry.spawn(request).await?;
    let duration = started.elapsed();
    info!("EXEC: /finished in {:?}", duration);

    let read = exec_registry
        .read(&result.snapshot.meta.process_id, 0, None)
        .await;
    let raw_output = exec_registry
        .read_raw_capture(&result.snapshot.meta.process_id)
        .await;
    let (stdout, stderr) = collect_foreground_output(&read, raw_output.as_ref());

    let stdout = output_mini_postprocessing(&cfg.output_filter, &stdout);
    let stderr = output_mini_postprocessing(&cfg.output_filter, &stderr);

    let mut out = format_output(&stdout, &stderr);
    if read.is_truncated {
        out.push_str(&format!(
            "⚠️ Output was truncated by exec transcript limits ({} bytes kept, {} bytes dropped, {} chunks truncated).\n",
            read.current_bytes, read.dropped_bytes, read.truncated_chunks
        ));
    }
    if let Some(raw_output) = raw_output.as_ref() {
        if raw_output.is_truncated() {
            out.push_str(&format!(
                "⚠️ Raw foreground capture reached limits (stdout: {}/{} bytes kept, {} bytes elided; stderr: {}/{} bytes kept, {} bytes elided).\n",
                raw_output.stdout_captured_bytes,
                raw_output.stdout_max_bytes,
                raw_output.stdout_elided_bytes,
                raw_output.stderr_captured_bytes,
                raw_output.stderr_max_bytes,
                raw_output.stderr_elided_bytes
            ));
        }
    }
    append_status_line(&mut out, &result.snapshot.status, duration, timeout_secs);
    Ok((out, result.snapshot, duration))
}

fn _parse_command_args(
    args: &HashMap<String, serde_json::Value>,
    cfg: &CmdlineToolConfig,
) -> Result<(String, String), String> {
    let mut args_str: HashMap<String, String> = HashMap::new();
    let valid_params: Vec<String> = cfg.parameters.iter().map(|p| p.name.clone()).collect();

    for (k, v) in args.iter() {
        if !valid_params.contains(k) {
            return Err(format!("Unexpected argument `{}`", k));
        }
        match v {
            serde_json::Value::String(s) => {
                args_str.insert(k.clone(), s.clone());
            }
            _ => return Err(format!("argument `{}` is not a string: {:?}", k, v)),
        }
    }

    for param in &cfg.parameters {
        if cfg
            .parameters_required
            .as_ref()
            .map_or(false, |req| req.contains(&param.name))
            && !args_str.contains_key(&param.name)
        {
            return Err(format!("Missing required argument `{}`", param.name));
        }
    }

    let command = replace_args(cfg.command.as_str(), &args_str);
    let workdir = replace_args(cfg.command_workdir.as_str(), &args_str);
    Ok((command, workdir))
}

#[async_trait]
impl Tool for ToolCmdline {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let (command, workdir) = _parse_command_args(args, &self.cfg)?;

        let (gcx, exec_registry, abort_flag, chat_id) = {
            let cgcx = ccx.lock().await;
            (
                cgcx.global_context.clone(),
                cgcx.app.runtime.exec_registry.clone(),
                cgcx.abort_flag.clone(),
                cgcx.chat_id.clone(),
            )
        };
        let user_activity = gcx.user_activity.clone();
        if let Ok(mut ring) = user_activity.try_lock() {
            ring.push(UserAction::CommandRun {
                command_preview: command.chars().take(80).collect(),
                chat_id: chat_id.clone(),
                ts: Utc::now(),
            });
        };
        let mut error_log = Vec::<YamlError>::new();
        let env_variables =
            crate::integrations::setting_up_integrations::get_vars_for_replacements(
                gcx.clone(),
                &mut error_log,
            )
            .await;
        let project_dirs = crate::files_correction::get_project_dirs(gcx.clone()).await;

        let owner = ExecOwnerMeta {
            chat_id: Some(chat_id),
            tool_call_id: Some(tool_call_id.clone()),
            service_name: None,
            workspace: project_dirs.first().cloned(),
        };
        let short_description =
            cmdline_short_description(&self.name, &self.cfg.description, &command);
        let (tool_output, snapshot, duration) = execute_blocking_command(
            &command,
            &self.cfg,
            &workdir,
            &env_variables,
            project_dirs,
            &exec_registry,
            owner,
            abort_flag,
            short_description,
        )
        .await?;

        let result = vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(tool_output),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            tool_failed: tool_failed_for_status(&snapshot.status),
            output_filter: Some(OutputFilter::no_limits()),
            extra: exec_extra(&snapshot, duration),
            ..Default::default()
        })];

        Ok((false, result))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }

    fn tool_description(&self) -> ToolDesc {
        let required: Vec<String> = self
            .cfg
            .parameters_required
            .clone()
            .unwrap_or_else(|| self.cfg.parameters.iter().map(|p| p.name.clone()).collect());
        let mut properties = serde_json::Map::new();
        for p in &self.cfg.parameters {
            properties.insert(
                p.name.clone(),
                json!({
                    "type": p.param_type,
                    "description": p.description
                }),
            );
        }
        let input_schema = json!({
            "type": "object",
            "properties": properties,
            "required": required
        });
        ToolDesc {
            name: self.name.clone(),
            display_name: self.name.clone(),
            source: ToolSource {
                source_type: ToolSourceType::Integration,
                config_path: self.config_path.clone(),
            },
            experimental: false,
            allow_parallel: false,
            description: self.cfg.description.clone(),
            input_schema,
            output_schema: None,
            annotations: None,
        }
    }

    async fn command_to_match_against_confirm_deny(
        &self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<String, String> {
        let (command, _workdir) = _parse_command_args(args, &self.cfg)?;
        Ok(command)
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(self.integr_common().confirmation)
    }

    fn has_config_path(&self) -> Option<String> {
        Some(self.config_path.clone())
    }
}

pub const CMDLINE_INTEGRATION_SCHEMA: &str = r#"
fields:
  command:
    f_type: string_long
    f_desc: "The command to execute. To let model produce part of the command, use %param_name% notation."
    f_placeholder: "echo Hello World"
  command_workdir:
    f_type: string_long
    f_desc: "The working directory for the command. If empty then workspace directory will be used. There you can use %param_name% as well."
    f_placeholder: "/path/to/workdir"
  description:
    f_type: string_long
    f_desc: "The model will see this description, why the model should call this?"
  parameters:
    f_type: "tool_parameters"
    f_desc: "The parameters that the model should fill out. Use description to tell the model what a parameter does. The only way you can use values coming from the model is to put them into %param_name% notation in the command or the working directory."
  timeout:
    f_type: string_short
    f_desc: "The command must immediately return the results, it can't be interactive. If the command runs for too long, it will be terminated and stderr/stdout collected will be presented to the model."
    f_default: "10"
  output_filter:
    f_type: "output_filter"
    f_desc: "The output from the command can be long or even quasi-infinite. This section allows to set limits, prioritize top or bottom, or use regexp to show the model the relevant part."
    f_placeholder: "filter"
    f_extra: true
description: |
  There you can adapt any command line tool for use by AI model. You can give the model instructions why to call it, which parameters to provide,
  set a timeout and restrict the output. If you want a tool that runs in the background such as a web server, use service_* instead.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
confirmation:
  ask_user_default: ["*"]
  deny_default: ["sudo*"]
smartlinks:
  - sl_label: "Test"
    sl_chat:
      - role: "user"
        content: |
          🔧 Test the tool that corresponds to %CURRENT_CONFIG%
          If the tool isn't available or doesn't work, go through the usual plan in the system prompt. If it works express happiness, and change nothing.
    sl_enable_only_with_tool: true
  - sl_label: "Auto Configure"
    sl_chat:
      - role: "user"
        content: |
          🔧 Please write %CURRENT_CONFIG% based on what you see in the project. Follow the plan in the system prompt.
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::AppState;
    use crate::tools::tools_description::{MatchConfirmDenyResult, Tool};

    fn args(entries: Vec<(&str, Value)>) -> HashMap<String, Value> {
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect()
    }

    async fn test_ccx() -> (Arc<GlobalContext>, Arc<AMutex<AtCommandsContext>>) {
        let gcx = crate::global_context::tests::make_test_gcx().await;
        let ccx = AtCommandsContext::new_with_abort(
            AppState::from_gcx(gcx.clone()).await,
            4096,
            20,
            false,
            Vec::new(),
            "chat".to_string(),
            None,
            "model".to_string(),
            None,
            None,
            None,
        )
        .await;
        (gcx, Arc::new(AMutex::new(ccx)))
    }

    fn only_message(messages: Vec<ContextEnum>) -> ChatMessage {
        match messages.into_iter().next().unwrap() {
            ContextEnum::ChatMessage(message) => message,
            ContextEnum::ContextFile(_) => panic!("expected chat message"),
        }
    }

    fn text(message: &ChatMessage) -> String {
        match &message.content {
            ChatContent::SimpleText(text) => text.clone(),
            _ => String::new(),
        }
    }

    fn exec(message: &ChatMessage) -> &Value {
        message.extra.get("exec").unwrap()
    }

    fn success_command() -> String {
        if cfg!(target_os = "windows") {
            "[Console]::Out.Write('hello')".to_string()
        } else {
            "printf hello".to_string()
        }
    }

    fn failure_command() -> String {
        if cfg!(target_os = "windows") {
            "[Console]::Out.Write('bad'); exit 7".to_string()
        } else {
            "printf bad; exit 7".to_string()
        }
    }

    fn slow_command() -> String {
        if cfg!(target_os = "windows") {
            "[Console]::Out.Write('start'); Start-Sleep -Seconds 5".to_string()
        } else {
            "printf start; sleep 5".to_string()
        }
    }

    fn late_marker_command(marker: &str) -> String {
        if cfg!(target_os = "windows") {
            format!(
                "$chunk = ('f' * 1024) + \"`n\"; [Console]::Out.Write($chunk * 1024); [Console]::Out.Write('{marker}`n'); $tail = ('t' * 1024) + \"`n\"; [Console]::Out.Write($tail * 3072)"
            )
        } else {
            format!(
                "chunk=$(printf '%01024d\\n' 0 | tr '0' f); i=0; while [ $i -lt 1024 ]; do printf '%s\\n' \"$chunk\"; i=$((i + 1)); done; printf '%s\\n' '{marker}'; chunk=$(printf '%01024d\\n' 0 | tr '0' t); i=0; while [ $i -lt 3072 ]; do printf '%s\\n' \"$chunk\"; i=$((i + 1)); done"
            )
        }
    }

    async fn run_tool(mut tool: ToolCmdline, args: HashMap<String, Value>) -> ChatMessage {
        let (_, ccx) = test_ccx().await;
        let (_, messages) = tool
            .tool_execute(ccx, &"tool_call".to_string(), &args)
            .await
            .unwrap();
        only_message(messages)
    }

    #[test]
    fn replace_args_keeps_shell_escaping_and_empty_replacement() {
        let replaced = replace_args(
            "echo %target% %empty%",
            &HashMap::from([
                ("target".to_string(), "hello world".to_string()),
                ("empty".to_string(), String::new()),
            ]),
        );

        #[cfg(target_os = "windows")]
        assert_eq!(replaced, "echo \"hello world\" ");
        #[cfg(not(target_os = "windows"))]
        assert_eq!(replaced, "echo 'hello world' ");
    }

    #[tokio::test]
    async fn configured_cmdline_success_uses_exec_runtime_and_metadata() {
        let (gcx, ccx) = test_ccx().await;
        let mut tool = ToolCmdline {
            name: "cmdline_hello".to_string(),
            cfg: CmdlineToolConfig {
                command: success_command(),
                description: "Say hello".to_string(),
                parameters: Vec::new(),
                ..CmdlineToolConfig::default()
            },
            ..ToolCmdline::default()
        };

        let (_, messages) = tool
            .tool_execute(ccx, &"tool_call".to_string(), &args(vec![]))
            .await
            .unwrap();
        let message = only_message(messages);
        let body = text(&message);
        let exec = exec(&message);

        assert!(body.contains("hello"));
        assert!(body.contains("exit code 0"));
        assert_eq!(exec["status"], "exited");
        assert_eq!(exec["exit_code"], 0);
        assert_eq!(exec["short_description"], "cmdline_hello: Say hello");
        let process_id = exec["process_id"].as_str().unwrap();
        assert!(process_id.starts_with("exec_"));
        assert!(gcx
            .exec_registry
            .list(Default::default())
            .await
            .iter()
            .any(|snapshot| snapshot.meta.process_id.as_str() == process_id));
        assert!(message.tool_failed.is_none());
    }

    #[tokio::test]
    async fn configured_cmdline_failure_reports_exit_code_in_metadata() {
        let message = run_tool(
            ToolCmdline {
                name: "cmdline_fail".to_string(),
                cfg: CmdlineToolConfig {
                    command: failure_command(),
                    description: "Fail nicely".to_string(),
                    parameters: Vec::new(),
                    ..CmdlineToolConfig::default()
                },
                ..ToolCmdline::default()
            },
            args(vec![]),
        )
        .await;
        let body = text(&message);
        let exec = exec(&message);

        assert!(body.contains("bad"));
        assert!(body.contains("exit code 7"));
        assert_eq!(exec["status"], "exited");
        assert_eq!(exec["exit_code"], 7);
        assert!(message.tool_failed.is_none());
    }

    #[tokio::test]
    async fn configured_cmdline_timeout_returns_partial_output_and_metadata() {
        let message = run_tool(
            ToolCmdline {
                name: "cmdline_slow".to_string(),
                cfg: CmdlineToolConfig {
                    command: slow_command(),
                    description: "Slow command".to_string(),
                    parameters: Vec::new(),
                    timeout: "1".to_string(),
                    ..CmdlineToolConfig::default()
                },
                ..ToolCmdline::default()
            },
            args(vec![]),
        )
        .await;
        let body = text(&message);
        let exec = exec(&message);

        assert!(body.contains("start"));
        assert!(body.contains("timed out"));
        assert_eq!(exec["status"], "timed_out");
        assert!(exec["exit_code"].is_null());
        assert_eq!(message.tool_failed, Some(true));
    }

    #[tokio::test]
    async fn configured_cmdline_abort_returns_partial_output_and_metadata() {
        let (gcx, ccx) = test_ccx().await;
        let abort_flag = Arc::new(AtomicBool::new(false));
        {
            let mut ccx_lock = ccx.lock().await;
            ccx_lock.abort_flag = abort_flag.clone();
        }
        let mut tool = ToolCmdline {
            name: "cmdline_abort".to_string(),
            cfg: CmdlineToolConfig {
                command: slow_command(),
                description: "Abort command".to_string(),
                parameters: Vec::new(),
                timeout: "10".to_string(),
                ..CmdlineToolConfig::default()
            },
            ..ToolCmdline::default()
        };
        let run = tokio::spawn({
            let ccx = ccx.clone();
            async move {
                tool.tool_execute(ccx, &"tool_call".to_string(), &args(vec![]))
                    .await
                    .unwrap()
            }
        });
        tokio::time::sleep(Duration::from_millis(200)).await;
        abort_flag.store(true, std::sync::atomic::Ordering::Relaxed);
        let (_, messages) = run.await.unwrap();
        let message = only_message(messages);
        let body = text(&message);
        let exec = exec(&message);

        assert!(body.contains("start"));
        assert!(body.contains("interrupted by user"));
        assert_eq!(exec["status"], "killed");
        assert_eq!(message.tool_failed, Some(true));
        let process_id = exec["process_id"].as_str().unwrap();
        let snapshot = gcx
            .exec_registry
            .get(&crate::exec::ExecProcessId(process_id.to_string()))
            .await
            .unwrap();
        assert_eq!(snapshot.status, ExecStatus::Killed);
    }

    #[tokio::test]
    async fn configured_cmdline_output_filter_finds_late_match() {
        let marker = "CMDLINE_FOREGROUND_LATE_MATCH";
        let message = run_tool(
            ToolCmdline {
                name: "cmdline_filter".to_string(),
                cfg: CmdlineToolConfig {
                    command: late_marker_command(marker),
                    description: "Find marker".to_string(),
                    parameters: Vec::new(),
                    timeout: "20".to_string(),
                    output_filter: OutputFilter {
                        grep: marker.to_string(),
                        limit_lines: 8,
                        limit_chars: 2000,
                        ..OutputFilter::default()
                    },
                    ..CmdlineToolConfig::default()
                },
                ..ToolCmdline::default()
            },
            args(vec![]),
        )
        .await;
        let body = text(&message);

        assert!(body.contains(marker));
        assert!(body.contains("filtered"));
        assert_eq!(exec(&message)["status"], "exited");
    }

    #[tokio::test]
    async fn configured_cmdline_confirmation_matches_configured_command_text() {
        let (_, ccx) = test_ccx().await;
        let tool = ToolCmdline {
            common: IntegrationCommon {
                confirmation: IntegrationConfirmation {
                    ask_user: vec!["echo*".to_string()],
                    deny: vec!["sudo*".to_string()],
                },
                ..IntegrationCommon::default()
            },
            cfg: CmdlineToolConfig {
                command: "echo %target%".to_string(),
                parameters: vec![CmdlineParam {
                    name: "target".to_string(),
                    param_type: "string".to_string(),
                    description: String::new(),
                }],
                parameters_required: Some(vec!["target".to_string()]),
                ..CmdlineToolConfig::default()
            },
            ..ToolCmdline::default()
        };

        let matched = tool
            .match_against_confirm_deny(ccx, &args(vec![("target", json!("hello world"))]))
            .await
            .unwrap();

        assert_eq!(matched.result, MatchConfirmDenyResult::CONFIRMATION);
        #[cfg(target_os = "windows")]
        assert_eq!(matched.command, "echo \"hello world\"");
        #[cfg(not(target_os = "windows"))]
        assert_eq!(matched.command, "echo 'hello world'");
        assert_eq!(matched.rule, "echo*");
    }

    #[test]
    fn configured_cmdline_schema_keeps_required_fields() {
        let schema: serde_yaml::Value = serde_yaml::from_str(CMDLINE_INTEGRATION_SCHEMA).unwrap();
        let fields = schema.get("fields").unwrap();

        for field in [
            "command",
            "command_workdir",
            "description",
            "parameters",
            "timeout",
            "output_filter",
        ] {
            assert!(fields.get(field).is_some(), "missing field {field}");
        }
        assert_eq!(
            schema["confirmation"]["ask_user_default"][0].as_str(),
            Some("*")
        );
        assert_eq!(
            schema["confirmation"]["deny_default"][0].as_str(),
            Some("sudo*")
        );
    }

    #[tokio::test]
    async fn configured_cmdline_required_param_validation_is_unchanged() {
        let tool = ToolCmdline {
            cfg: CmdlineToolConfig {
                command: "echo %target%".to_string(),
                parameters: vec![CmdlineParam {
                    name: "target".to_string(),
                    param_type: "string".to_string(),
                    description: String::new(),
                }],
                parameters_required: Some(vec!["target".to_string()]),
                ..CmdlineToolConfig::default()
            },
            ..ToolCmdline::default()
        };

        let err = tool
            .command_to_match_against_confirm_deny(test_ccx().await.1, &args(vec![]))
            .await
            .unwrap_err();

        assert_eq!(err, "Missing required argument `target`");
    }
}
