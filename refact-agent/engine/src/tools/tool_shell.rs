use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::return_one_candidate_or_a_good_error;
use crate::call_validation::{ChatContent, ChatMessage, ContextEnum};
use crate::exec::{
    generate_short_description, sanitize_short_description, ExecMode, ExecOutputStream,
    ExecOwnerMeta, ExecProcessSnapshot, ExecRawOutput, ExecReadResult, ExecSpawnRequest,
    ExecStatus,
};
use crate::files_correction::canonical_path;
use crate::files_correction::canonicalize_normalized_path;
use crate::files_correction::check_if_its_inside_a_workspace_or_config;
use crate::files_correction::correct_to_nearest_dir_path;
use crate::files_correction::get_active_project_path;
use crate::files_correction::get_project_dirs;
use crate::files_correction::preprocess_path_for_normalization;
use crate::global_context::GlobalContext;
use crate::integrations::integr_abstract::IntegrationConfirmation;
use crate::postprocessing::pp_command_output::{
    output_mini_postprocessing, parse_output_filter_args, OutputFilter,
};
use crate::privacy::{check_file_privacy, load_privacy_if_needed, FilePrivacyLevel};
use crate::tools::file_edit::auxiliary::{active_execution_scope, scoped_path_warnings};
use crate::tools::tools_description::{
    json_schema_from_params, Tool, ToolDesc, ToolSource, ToolSourceType,
};
use crate::worktrees::scope::ExecutionScope;

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct SettingsShell {
    #[serde(default)]
    pub timeout: String,
    #[serde(default)]
    pub output_filter: OutputFilter,
}

#[derive(Default)]
pub struct ToolShell {
    pub cfg: SettingsShell,
    pub config_path: String,
}

const ASK_USER_DEFAULT: &[&str] = &[
    "*rm*",
    "*rmdir*",
    "*del /s*",
    "*deltree*",
    "*mkfs*",
    "*dd *",
    "*format*",
    "*> /dev/*",
    ":(){ :|:& };:",
    "*chmod -R*",
    "*chown -R*",
    "*chmod 777*",
    "*chmod a+rwx*",
    "*git push*",
    "*git reset --hard*",
    "curl * | sh",
    "curl * | bash",
    "wget * -O - | sh",
    "wget * -O - | bash",
    "*apt-get remove*",
    "*apt-get purge*",
    "*apt remove*",
    "*apt purge*",
    "*yum remove*",
    "*yum erase*",
    "*dnf remove*",
    "*pacman -R*",
    "*brew uninstall*",
    "*docker rm*",
    "*docker rmi*",
    "*docker system prune*",
    "*kubectl delete*",
    "*kill -9*",
    "*killall*",
    "*pkill*",
    "*shutdown*",
    "*reboot*",
    "*halt*",
    "*poweroff*",
    "*init 0*",
    "*init 6*",
    "*systemctl stop*",
    "*systemctl disable*",
    "*service * stop",
    "*truncate -s 0*",
    "*fdisk*",
    "*parted*",
    "*mkswap*",
    "*swapon*",
    "*swapoff*",
    "*mount*",
    "*umount*",
    "*crontab -r*",
    "*history -c*",
    "*shred*",
    "*wipe*",
    "*srm*",
];

const DENY_DEFAULT: &[&str] = &["sudo*"];
const SHELL_TRANSCRIPT_MAX_BYTES: usize = 2 * 1024 * 1024;

struct ParsedShellArgs {
    command: String,
    workdir: Option<PathBuf>,
    custom_filter: Option<OutputFilter>,
    timeout: Option<u64>,
    description: Option<String>,
    scope_warnings: Vec<String>,
}

#[async_trait]
impl Tool for ToolShell {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let (gcx, exec_registry, abort_flag, execution_scope, chat_id) = {
            let ccx_lock = ccx.lock().await;
            (
                ccx_lock.app.gcx.clone(),
                ccx_lock.app.runtime.exec_registry.clone(),
                ccx_lock.abort_flag.clone(),
                ccx_lock.execution_scope.clone(),
                ccx_lock.chat_id.clone(),
            )
        };
        let parsed = parse_args_with_filter(
            gcx.clone(),
            args,
            &self.cfg.output_filter,
            execution_scope.as_ref(),
        )
        .await?;
        let timeout = parsed
            .timeout
            .unwrap_or_else(|| self.cfg.timeout.parse::<u64>().unwrap_or(10));

        let mut error_log = Vec::new();
        let env_variables =
            crate::integrations::setting_up_integrations::get_vars_for_replacements(
                gcx.clone(),
                &mut error_log,
            )
            .await;
        let output_filter = parsed
            .custom_filter
            .clone()
            .unwrap_or_else(|| self.cfg.output_filter.clone());
        let cwd = match parsed.workdir.clone() {
            Some(workdir) => Some(workdir),
            None => get_active_project_path(gcx.clone()).await,
        };
        let short_description = parsed
            .description
            .as_deref()
            .map(sanitize_short_description)
            .filter(|desc| !desc.is_empty())
            .unwrap_or_else(|| generate_short_description(&parsed.command, &ExecMode::Foreground));
        let owner = ExecOwnerMeta {
            chat_id: Some(chat_id),
            tool_call_id: Some(tool_call_id.clone()),
            service_name: None,
            workspace: active_execution_scope(execution_scope.as_ref())
                .map(|scope| scope.effective_root().to_path_buf()),
        };
        let mut request = ExecSpawnRequest::foreground(parsed.command.clone())
            .with_timeout(Duration::from_secs(timeout))
            .with_env_map(env_variables)
            .with_owner(owner)
            .with_transcript_limit(SHELL_TRANSCRIPT_MAX_BYTES)
            .with_short_description(short_description)
            .with_abort_flag(abort_flag);
        if let Some(cwd) = cwd {
            request = request.with_cwd(cwd);
        }

        let started = tokio::time::Instant::now();
        let result = exec_registry.spawn(request).await?;
        let duration_secs = started.elapsed().as_secs_f64();
        let read = exec_registry
            .read(&result.snapshot.meta.process_id, 0, None)
            .await;
        let raw_output = exec_registry
            .read_raw_capture(&result.snapshot.meta.process_id)
            .await;
        let (stdout, stderr) = collect_foreground_output(&read, raw_output.as_ref());
        let filtered_stdout = output_mini_postprocessing(&output_filter, &stdout);
        let filtered_stderr = output_mini_postprocessing(&output_filter, &stderr);

        let mut out =
            crate::integrations::integr_cmdline::format_output(&filtered_stdout, &filtered_stderr);
        if !parsed.scope_warnings.is_empty() {
            out = format!("{}\n{}", parsed.scope_warnings.join("\n"), out);
        }
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
        append_status_line(&mut out, &result.snapshot.status, duration_secs, timeout);

        let msg = vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(out),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            tool_failed: tool_failed_for_status(&result.snapshot.status),
            output_filter: Some(OutputFilter::no_limits()),
            extra: exec_extra(&result.snapshot, &read, duration_secs, timeout),
            ..Default::default()
        })];

        Ok((false, msg))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "shell".to_string(),
            display_name: "Shell".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            experimental: false,
            allow_parallel: false,
            description: "Execute a single command, using the \"sh\" on unix-like systems and \"powershell.exe\" on windows. Use it for one-time tasks like dependencies installation. Don't call this unless you have to. Not suitable for regular work because it requires a confirmation at each step. Output is compressed by default - use output_filter and output_limit parameters to see specific parts if needed. In worktree-scoped chats, the default cwd and explicit workdir are enforced to the active worktree or privacy-permitted outside paths with visible warnings; the shell command text itself is not OS-sandboxed. Note: sudo commands cannot be run - if you need elevated privileges, ask the user to run them directly.".to_string(),
            input_schema: json_schema_from_params(&[("command", "string", "shell command to execute"), ("workdir", "string", "workdir for the command"), ("timeout", "string", "Optional. Timeout in seconds for the command (default: 10). Use higher values for long-running commands."), ("description", "string", "Optional short description shown in execution UI metadata."), ("output_filter", "string", "Optional regex pattern to filter output lines. Only lines matching this pattern (and context) will be shown. Use to find specific errors or content in large outputs."), ("output_limit", "string", "Optional. Max lines to show (default: 40). Use higher values like '200' or 'all' to see more output.")], &["command"]),
            output_schema: None,
            annotations: None,
        }
    }

    async fn command_to_match_against_confirm_deny(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        let (gcx, execution_scope) = {
            let cgcx = ccx.lock().await;
            (cgcx.app.gcx.clone(), cgcx.execution_scope.clone())
        };
        let (command, _) = parse_args(gcx, args, execution_scope.as_ref()).await?;
        Ok(command)
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(IntegrationConfirmation {
            ask_user: ASK_USER_DEFAULT.iter().map(|s| s.to_string()).collect(),
            deny: DENY_DEFAULT.iter().map(|s| s.to_string()).collect(),
        })
    }

    fn has_config_path(&self) -> Option<String> {
        Some(self.config_path.clone())
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
    duration_secs: f64,
    timeout_secs: u64,
) {
    match status {
        ExecStatus::Exited { exit_code } => out.push_str(&format!(
            "The command was running {:.3}s, finished with exit code {}\n",
            duration_secs,
            exit_code.unwrap_or_default()
        )),
        ExecStatus::Killed => out.push_str(&format!(
            "⚠️ The command was interrupted by user after {:.3}s (process killed). Output above may be incomplete.\n",
            duration_secs
        )),
        ExecStatus::TimedOut => out.push_str(&format!(
            "⚠️ The command timed out after {} seconds (process killed). Output above may be incomplete.\n",
            timeout_secs
        )),
        ExecStatus::Failed { message } => out.push_str(&format!(
            "⚠️ The command failed after {:.3}s: {}\n",
            duration_secs, message
        )),
        ExecStatus::Starting | ExecStatus::Running => out.push_str(&format!(
            "⚠️ The command did not reach a terminal state after {:.3}s (status: {}).\n",
            duration_secs,
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
    read: &ExecReadResult,
    duration_secs: f64,
    timeout_secs: u64,
) -> serde_json::Map<String, Value> {
    let mut extra = serde_json::Map::new();
    let cwd = snapshot
        .meta
        .cwd
        .as_ref()
        .map(|path| path.to_string_lossy().to_string());
    let status_detail = serde_json::to_value(&snapshot.status).unwrap_or(Value::Null);
    extra.insert(
        "exec".to_string(),
        json!({
            "process_id": snapshot.meta.process_id.as_str(),
            "status": exec_status_label(&snapshot.status),
            "status_detail": status_detail,
            "exit_code": exec_exit_code(&snapshot.status),
            "short_description": snapshot.meta.short_description,
            "command": snapshot.meta.command,
            "cwd": cwd,
            "mode": snapshot.meta.mode.to_string(),
            "duration_secs": duration_secs,
            "timeout_secs": timeout_secs,
            "created_at_ms": snapshot.meta.created_at_ms,
            "started_at_ms": snapshot.meta.started_at_ms,
            "ended_at_ms": snapshot.meta.ended_at_ms,
            "transcript": {
                "total_bytes_appended": read.total_bytes_appended,
                "total_lines_appended": read.total_lines_appended,
                "dropped_chunks": read.dropped_chunks,
                "dropped_bytes": read.dropped_bytes,
                "truncated_chunks": read.truncated_chunks,
                "current_bytes": read.current_bytes,
                "max_bytes": read.max_bytes,
                "chunk_count": read.chunk_count,
                "is_truncated": read.is_truncated,
            }
        }),
    );
    extra
}

async fn parse_args(
    gcx: Arc<GlobalContext>,
    args: &HashMap<String, Value>,
    execution_scope: Option<&ExecutionScope>,
) -> Result<(String, Option<PathBuf>), String> {
    let parsed =
        parse_args_with_filter(gcx, args, &OutputFilter::default(), execution_scope).await?;
    Ok((parsed.command, parsed.workdir))
}

async fn parse_args_with_filter(
    gcx: Arc<GlobalContext>,
    args: &HashMap<String, Value>,
    config_filter: &OutputFilter,
    execution_scope: Option<&ExecutionScope>,
) -> Result<ParsedShellArgs, String> {
    let command = match args.get("command") {
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err("Command is empty".to_string());
            }
            s.clone()
        }
        Some(v) => return Err(format!("argument `command` is not a string: {:?}", v)),
        None => return Err("Missing argument `command`".to_string()),
    };

    let raw_workdir = match args.get("workdir") {
        Some(Value::String(s)) if s.is_empty() => None,
        Some(Value::String(s)) => Some(s.as_str()),
        Some(v) => return Err(format!("argument `workdir` is not a string: {:?}", v)),
        None => None,
    };
    let (workdir, scope_warnings) =
        resolve_shell_workdir(gcx.clone(), raw_workdir, execution_scope).await?;

    let has_filter_override =
        args.get("output_filter").is_some() || args.get("output_limit").is_some();
    let custom_filter = if has_filter_override {
        Some(parse_output_filter_args(args, config_filter))
    } else {
        None
    };

    let timeout = match args.get("timeout") {
        Some(Value::String(s)) if s.is_empty() => None,
        Some(Value::String(s)) => Some(
            s.parse::<u64>()
                .map_err(|_| "argument `timeout` must be seconds as an integer".to_string())?,
        ),
        Some(Value::Number(n)) => n.as_u64(),
        Some(v) => {
            return Err(format!(
                "argument `timeout` is not a string or number: {:?}",
                v
            ))
        }
        None => None,
    };

    let description = match args.get("description") {
        Some(Value::String(s)) if s.is_empty() => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(v) => return Err(format!("argument `description` is not a string: {:?}", v)),
        None => None,
    };

    Ok(ParsedShellArgs {
        command,
        workdir,
        custom_filter,
        timeout,
        description,
        scope_warnings,
    })
}

async fn resolve_shell_workdir(
    gcx: Arc<GlobalContext>,
    raw_path: Option<&str>,
    execution_scope: Option<&ExecutionScope>,
) -> Result<(Option<PathBuf>, Vec<String>), String> {
    if let Some(scope) = active_execution_scope(execution_scope) {
        let scoped = scope.resolve_workdir(raw_path).map_err(|e| {
            format!(
                "⚠️ Cannot resolve shell workdir in active worktree '{}': {}",
                scope.effective_root().display(),
                e
            )
        })?;
        let privacy_settings = load_privacy_if_needed(gcx.clone()).await;
        if let Err(e) = check_file_privacy(
            privacy_settings,
            &scoped.path,
            &FilePrivacyLevel::AllowToSendAnywhere,
        ) {
            return Err(format!(
                "⚠️ Cannot use shell workdir '{}' (blocked by privacy: {}). Active worktree root: '{}'",
                scoped.path.display(),
                e,
                scope.effective_root().display()
            ));
        }
        let mut warnings = scoped_path_warnings(&scoped, scope);
        warnings.push(format!(
            "⚠️ Worktree scope: shell cwd/workdir is enforced as '{}', but shell command text is not OS-sandboxed",
            scoped.path.display()
        ));
        return Ok((Some(scoped.path), warnings));
    }

    let Some(raw_path) = raw_path else {
        return Ok((None, Vec::new()));
    };
    let path_str = preprocess_path_for_normalization(raw_path.to_string());
    let path = PathBuf::from(&path_str);

    let workdir = if path.is_absolute() {
        let path = canonicalize_normalized_path(path);
        check_if_its_inside_a_workspace_or_config(gcx.clone(), &path).await?;
        path
    } else {
        let project_dirs = get_project_dirs(gcx.clone()).await;
        let candidates = correct_to_nearest_dir_path(gcx.clone(), &path_str, false, 3).await;
        canonical_path(
            return_one_candidate_or_a_good_error(
                gcx.clone(),
                &path_str,
                &candidates,
                &project_dirs,
                true,
            )
            .await?,
        )
    };
    if !workdir.exists() {
        Err("Workdir doesn't exist".to_string())
    } else {
        Ok((Some(workdir), Vec::new()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use crate::app_state::AppState;
    use crate::tools::tools_description::Tool;

    use super::*;

    fn args(entries: Vec<(&str, Value)>) -> HashMap<String, Value> {
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect()
    }

    async fn ccx_with_abort(abort_flag: Option<Arc<AtomicBool>>) -> Arc<AMutex<AtCommandsContext>> {
        let gcx = crate::global_context::tests::make_test_gcx().await;
        Arc::new(AMutex::new(
            AtCommandsContext::new_with_abort(
                AppState::from_gcx(gcx).await,
                4096,
                20,
                false,
                Vec::new(),
                "chat".to_string(),
                None,
                "model".to_string(),
                None,
                None,
                abort_flag,
            )
            .await,
        ))
    }

    async fn run_shell(args: HashMap<String, Value>) -> ChatMessage {
        let ccx = ccx_with_abort(None).await;
        let mut shell = ToolShell::default();
        let (_, messages) = shell
            .tool_execute(ccx, &"shell".to_string(), &args)
            .await
            .unwrap();
        only_chat_message(messages)
    }

    fn only_chat_message(messages: Vec<ContextEnum>) -> ChatMessage {
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

    fn stderr_command() -> String {
        if cfg!(target_os = "windows") {
            "[Console]::Error.Write('warn')".to_string()
        } else {
            "printf warn >&2".to_string()
        }
    }

    fn nonzero_command() -> String {
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

    fn multiline_command() -> String {
        if cfg!(target_os = "windows") {
            "[Console]::Out.Write(\"line1`nline2`nline3`nline4`nline5`n\")".to_string()
        } else {
            "printf 'line1\nline2\nline3\nline4\nline5\n'".to_string()
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

    fn above_raw_capture_limit_command() -> String {
        if cfg!(target_os = "windows") {
            "$max = 16 * 1024 * 1024; $tail = \"`nsmall1`nsmall2`nsmall3`nsmall4`n\"; $line = ('x' * 1024) + \"`n\"; $count = [math]::Floor(($max - $tail.Length - 64) / $line.Length); for ($i = 0; $i -lt $count; $i++) { [Console]::Out.Write($line) }; [Console]::Out.Write('x' * ($max - $tail.Length - 64 - ($count * $line.Length))); [Console]::Out.Write($tail); [Console]::Out.Write(('y' * 1024) * 1024)".to_string()
        } else {
            "max_bytes=$((16 * 1024 * 1024)); tail='\nsmall1\nsmall2\nsmall3\nsmall4\n'; tail_len=29; line=$(printf '%01024d\\n' 0 | tr '0' x); line_len=1025; count=$(((max_bytes - tail_len - 64) / line_len)); i=0; while [ $i -lt $count ]; do printf '%s\\n' \"$line\"; i=$((i + 1)); done; rem=$((max_bytes - tail_len - 64 - count * line_len)); if [ $rem -gt 0 ]; then printf '%*s' \"$rem\" '' | tr ' ' x; fi; printf '%b' \"$tail\"; chunk=$(printf '%01024d\\n' 0 | tr '0' y); i=0; while [ $i -lt 1024 ]; do printf '%s\\n' \"$chunk\"; i=$((i + 1)); done".to_string()
        }
    }

    #[test]
    fn shell_tool_workdir_is_optional_in_schema() {
        let tool = ToolShell::default();
        let desc = tool.tool_description();
        let required = desc.input_schema["required"].as_array().unwrap().clone();
        let required_names: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(required_names.contains(&"command"));
        assert!(!required_names.contains(&"workdir"));
    }

    #[tokio::test]
    async fn shell_exec_success_contains_output_and_metadata() {
        let message = run_shell(args(vec![
            ("command", json!(success_command())),
            ("description", json!("Run hello")),
        ]))
        .await;

        let body = text(&message);
        let exec = exec(&message);
        assert!(body.contains("hello"));
        assert!(body.contains("exit code 0"));
        assert_eq!(exec["status"], "exited");
        assert_eq!(exec["exit_code"], 0);
        assert_eq!(exec["short_description"], "Run hello");
        assert!(exec["process_id"].as_str().unwrap().starts_with("exec_"));
        assert!(message.tool_failed.is_none());
    }

    #[tokio::test]
    async fn shell_exec_captures_stderr() {
        let message = run_shell(args(vec![("command", json!(stderr_command()))])).await;
        let body = text(&message);

        assert!(body.contains("STDERR"));
        assert!(body.contains("warn"));
        assert_eq!(exec(&message)["status"], "exited");
    }

    #[tokio::test]
    async fn shell_exec_reports_nonzero_exit() {
        let message = run_shell(args(vec![("command", json!(nonzero_command()))])).await;
        let body = text(&message);

        assert!(body.contains("bad"));
        assert!(body.contains("exit code 7"));
        assert_eq!(exec(&message)["status"], "exited");
        assert_eq!(exec(&message)["exit_code"], 7);
        assert!(message.tool_failed.is_none());
    }

    #[tokio::test]
    async fn shell_exec_timeout_returns_partial_output_and_failed_metadata() {
        let message = run_shell(args(vec![
            ("command", json!(slow_command())),
            ("timeout", json!(1)),
        ]))
        .await;
        let body = text(&message);

        assert!(body.contains("start"));
        assert!(body.contains("timed out"));
        assert_eq!(exec(&message)["status"], "timed_out");
        assert_eq!(message.tool_failed, Some(true));
    }

    #[tokio::test]
    async fn shell_exec_abort_returns_partial_output_and_failed_metadata() {
        let abort_flag = Arc::new(AtomicBool::new(false));
        let ccx = ccx_with_abort(Some(abort_flag.clone())).await;
        let mut shell = ToolShell::default();
        let tool_call_id = "shell".to_string();
        let run = tokio::spawn(async move {
            shell
                .tool_execute(
                    ccx,
                    &tool_call_id,
                    &args(vec![
                        ("command", json!(slow_command())),
                        ("timeout", json!(10)),
                    ]),
                )
                .await
                .unwrap()
        });
        tokio::time::sleep(Duration::from_millis(200)).await;
        abort_flag.store(true, Ordering::Relaxed);
        let (_, messages) = run.await.unwrap();
        let message = only_chat_message(messages);
        let body = text(&message);

        assert!(body.contains("start"));
        assert!(body.contains("interrupted by user"));
        assert_eq!(exec(&message)["status"], "killed");
        assert_eq!(message.tool_failed, Some(true));
    }

    #[tokio::test]
    async fn shell_exec_output_filter_is_applied() {
        let message = run_shell(args(vec![
            ("command", json!(multiline_command())),
            ("output_filter", json!("line4")),
            ("output_limit", json!("3")),
        ]))
        .await;
        let body = text(&message);

        assert!(body.contains("line4"));
        assert!(body.contains("filtered"));
        let filtered_output = body
            .split("The command was running")
            .next()
            .unwrap_or(&body);
        let line1_count = filtered_output.matches("line1").count();
        assert_eq!(line1_count, 1);
    }

    #[tokio::test]
    async fn foreground_output_filter_finds_late_match() {
        let marker = "MARKER_FOREGROUND_LATE_MATCH";
        let message = run_shell(args(vec![
            ("command", json!(late_marker_command(marker))),
            ("timeout", json!(20)),
            ("output_filter", json!(marker)),
            ("output_limit", json!("8")),
        ]))
        .await;
        let body = text(&message);

        assert!(body.contains(marker));
        assert!(body.contains("filtered"));
        assert_eq!(exec(&message)["status"], "exited");
    }

    #[tokio::test]
    async fn foreground_output_filter_bounded() {
        let message = run_shell(args(vec![
            ("command", json!(above_raw_capture_limit_command())),
            ("timeout", json!(20)),
            ("output_filter", json!("small|bytes elided")),
            ("output_limit", json!("20")),
        ]))
        .await;
        let body = text(&message);

        assert!(body.contains("bytes elided]"));
        assert!(body.contains("Raw foreground capture reached limits"));
        assert!(body.len() < 100_000);
        assert_eq!(exec(&message)["status"], "exited");
    }

    #[tokio::test]
    async fn shell_exec_description_is_sanitized_in_metadata() {
        let message = run_shell(args(vec![
            ("command", json!(success_command())),
            ("description", json!("  Build\tthing\nignore this\x01")),
        ]))
        .await;

        assert_eq!(exec(&message)["short_description"], "Build thing");
    }
}
