use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use std::process::Stdio;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tokio::io::{AsyncBufReadExt, BufReader};
use async_trait::async_trait;
use tokio::process::Command;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::return_one_candidate_or_a_good_error;
use crate::files_correction::canonical_path;
use crate::files_correction::canonicalize_normalized_path;
use crate::files_correction::check_if_its_inside_a_workspace_or_config;
use crate::files_correction::correct_to_nearest_dir_path;
use crate::files_correction::get_active_project_path;
use crate::files_correction::get_project_dirs;
use crate::files_correction::preprocess_path_for_normalization;
use crate::files_correction::CommandSimplifiedDirExt;
use crate::global_context::GlobalContext;
use crate::tools::tools_description::{ToolParam, Tool, ToolDesc, ToolSource, ToolSourceType, MatchConfirmDeny, MatchConfirmDenyResult};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::postprocessing::pp_command_output::OutputFilter;
use crate::integrations::integr_abstract::{IntegrationCommon, IntegrationTrait};
use crate::custom_error::YamlError;
use crate::tools::tools_description::{command_should_be_denied, command_should_be_confirmed_by_user};


#[derive(Deserialize, Serialize, Clone, Default)]
pub struct SettingsShell {
    #[serde(default)]
    pub timeout: String,
    #[serde(default)]
    pub output_filter: OutputFilter,
}

#[derive(Default)]
pub struct ToolShell {
    pub common: IntegrationCommon,
    pub cfg: SettingsShell,
    pub config_path: String,
}

#[async_trait]
impl IntegrationTrait for ToolShell {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn integr_schema(&self) -> &str
    {
        SHELL_INTEGRATION_SCHEMA
    }

    async fn integr_settings_apply(&mut self, _gcx: Arc<ARwLock<GlobalContext>>, config_path: String, value: &serde_json::Value) -> Result<(), serde_json::Error> {
        self.cfg = serde_json::from_value(value.clone())?;
        self.common = serde_json::from_value(value.clone())?;
        self.config_path = config_path;
        Ok(())
    }

    fn integr_settings_as_json(&self) -> Value {
        serde_json::to_value(&self.cfg).unwrap()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    async fn integr_tools(&self, _integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        vec![Box::new(ToolShell {
            common: self.common.clone(),
            cfg: self.cfg.clone(),
            config_path: self.config_path.clone(),
        })]
    }
}

#[async_trait]
impl Tool for ToolShell {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let (gcx, subchat_tx) = {
            let ccx_lock = ccx.lock().await;
            (ccx_lock.global_context.clone(), ccx_lock.subchat_tx.clone())
        };
        let (command, workdir_maybe, custom_filter, timeout_override) = parse_args_with_filter(gcx.clone(), args).await?;
        let timeout = timeout_override.unwrap_or_else(|| self.cfg.timeout.parse::<u64>().unwrap_or(10));

        let mut error_log = Vec::<YamlError>::new();
        let env_variables = crate::integrations::setting_up_integrations::get_vars_for_replacements(gcx.clone(), &mut error_log).await;

        let output_filter = custom_filter.unwrap_or_else(|| self.cfg.output_filter.clone());

        let tool_output = execute_shell_command_with_streaming(
            &command,
            &workdir_maybe,
            timeout,
            &env_variables,
            gcx.clone(),
            &subchat_tx,
            tool_call_id,
        ).await?;

        let result = vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(tool_output),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            output_filter: Some(output_filter),
            ..Default::default()
        })];

        Ok((false, result))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "shell".to_string(),
            display_name: "Shell".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Integration,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Execute a single command, using the \"sh\" on unix-like systems and \"powershell.exe\" on windows. Use it for one-time tasks like dependencies installation. Don't call this unless you have to. Not suitable for regular work because it requires a confirmation at each step. Output is compressed by default - use output_filter and output_limit parameters to see specific parts if needed. Note: sudo commands cannot be run - if you need elevated privileges, ask the user to run them directly.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "command".to_string(),
                    param_type: "string".to_string(),
                    description: "shell command to execute".to_string(),
                },
                ToolParam {
                    name: "workdir".to_string(),
                    param_type: "string".to_string(),
                    description: "workdir for the command".to_string(),
                },
                ToolParam {
                    name: "output_filter".to_string(),
                    param_type: "string".to_string(),
                    description: "Optional regex pattern to filter output lines. Only lines matching this pattern (and context) will be shown. Use to find specific errors or content in large outputs.".to_string(),
                },
                ToolParam {
                    name: "output_limit".to_string(),
                    param_type: "string".to_string(),
                    description: "Optional. Max lines to show (default: 40). Use higher values like '200' or 'all' to see more output.".to_string(),
                },
                ToolParam {
                    name: "timeout".to_string(),
                    param_type: "string".to_string(),
                    description: "Optional. Timeout in seconds for the command (default: 10). Use higher values for long-running commands.".to_string(),
                },
            ],
            parameters_required: vec![
                "command".to_string(),
                "workdir".to_string(),
            ],
        }
    }

    async fn match_against_confirm_deny(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>
    ) -> Result<MatchConfirmDeny, String> {
        let command_to_match = self.command_to_match_against_confirm_deny(ccx.clone(), &args).await.map_err(|e| {
            format!("Error getting tool command to match: {}", e)
        })?;
        if command_to_match.is_empty() {
            return Err("Empty command to match".to_string());
        }
        if let Some(rules) = &self.confirm_deny_rules() {
            let (is_denied, deny_rule) = command_should_be_denied(&command_to_match, &rules.deny);
            if is_denied {
                return Ok(MatchConfirmDeny {
                    result: MatchConfirmDenyResult::DENY,
                    command: command_to_match.clone(),
                    rule: deny_rule.clone(),
                });
            }

            let (needs_confirmation, confirmation_rule) = command_should_be_confirmed_by_user(&command_to_match, &rules.ask_user);
            if needs_confirmation {
                return Ok(MatchConfirmDeny {
                    result: MatchConfirmDenyResult::CONFIRMATION,
                    command: command_to_match.clone(),
                    rule: confirmation_rule.clone(),
                });
            }
        }

        Ok(MatchConfirmDeny {
            result: MatchConfirmDenyResult::PASS,
            command: command_to_match.clone(),
            rule: "".to_string(),
        })
    }

    async fn command_to_match_against_confirm_deny(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        let gcx = ccx.lock().await.global_context.clone();
        let (command, _) = parse_args(gcx, args).await?;
        Ok(command)
    }

    fn has_config_path(&self) -> Option<String> {
        Some(self.config_path.clone())
    }
}

fn send_streaming_update(
    subchat_tx: &Arc<AMutex<tokio::sync::mpsc::UnboundedSender<serde_json::Value>>>,
    tool_call_id: &str,
    content: &str,
) {
    let streaming_msg = json!({
        "tool_call_id": tool_call_id,
        "subchat_id": content,
        "add_message": {
            "role": "assistant",
            "content": content
        }
    });
    if let Ok(tx) = subchat_tx.try_lock() {
        let _ = tx.send(streaming_msg);
    }
}

fn spawn_output_streaming_task(
    subchat_tx: Arc<AMutex<tokio::sync::mpsc::UnboundedSender<serde_json::Value>>>,
    tool_call_id: String,
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
    cancel_token: tokio_util::sync::CancellationToken,
    output_collector: Arc<AMutex<(Vec<String>, Vec<String>)>>,
) {
    tokio::spawn(async move {
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();
        let mut last_update = tokio::time::Instant::now();
        let update_interval = tokio::time::Duration::from_secs(2);

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    break;
                }
                result = stdout_reader.next_line() => {
                    match result {
                        Ok(Some(line)) => {
                            let stripped = strip_ansi_escapes::strip(line.as_bytes());
                            let clean_line = String::from_utf8_lossy(&stripped).to_string();
                            {
                                let mut collector = output_collector.lock().await;
                                collector.0.push(clean_line);
                            }
                            if last_update.elapsed() >= update_interval {
                                let collector = output_collector.lock().await;
                                let total_lines = collector.0.len();
                                let preview: String = if total_lines > 3 {
                                    collector.0[total_lines-3..].join("\n")
                                } else {
                                    collector.0.join("\n")
                                };
                                drop(collector);
                                send_streaming_update(
                                    &subchat_tx,
                                    &tool_call_id,
                                    &format!("📤 stdout ({} lines):\n```\n{}\n```", total_lines, preview)
                                );
                                last_update = tokio::time::Instant::now();
                            }
                        }
                        Ok(None) => {
                            // stdout closed
                            break;
                        }
                        Err(e) => {
                            tracing::warn!("Error reading stdout: {}", e);
                            break;
                        }
                    }
                }
                result = stderr_reader.next_line() => {
                    match result {
                        Ok(Some(line)) => {
                            let stripped = strip_ansi_escapes::strip(line.as_bytes());
                            let clean_line = String::from_utf8_lossy(&stripped).to_string();
                            {
                                let mut collector = output_collector.lock().await;
                                collector.1.push(clean_line.clone());
                            }
                            if !clean_line.trim().is_empty() {
                                send_streaming_update(
                                    &subchat_tx,
                                    &tool_call_id,
                                    &format!("⚠️ stderr: {}", clean_line)
                                );
                            }
                        }
                        Ok(None) => {
                            // stderr closed, but keep reading stdout
                        }
                        Err(e) => {
                            tracing::warn!("Error reading stderr: {}", e);
                        }
                    }
                }
            }
        }
    });
}

pub async fn execute_shell_command_with_streaming(
    command: &str,
    workdir_maybe: &Option<PathBuf>,
    timeout: u64,
    env_variables: &HashMap<String, String>,
    gcx: Arc<ARwLock<GlobalContext>>,
    subchat_tx: &Arc<AMutex<tokio::sync::mpsc::UnboundedSender<serde_json::Value>>>,
    tool_call_id: &str,
) -> Result<String, String> {
    let shell = if cfg!(target_os = "windows") { "powershell.exe" } else { "sh" };
    let shell_arg = if cfg!(target_os = "windows") { "-Command" } else { "-c" };
    let mut cmd = Command::new(shell);

    if let Some(workdir) = workdir_maybe {
        cmd.current_dir_simplified(workdir);
    } else if let Some(project_path) = get_active_project_path(gcx.clone()).await {
        cmd.current_dir_simplified(&project_path);
    } else {
        tracing::warn!("no working directory, using whatever directory this binary is run :/");
    }

    for (key, value) in env_variables {
        cmd.env(key, value);
    }

    cmd.arg(shell_arg).arg(command);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    tracing::info!("SHELL: running command directory {:?}\n{:?}", workdir_maybe, command);

    send_streaming_update(subchat_tx, tool_call_id, &format!("🔧 Running: {}", command));

    let t0 = tokio::time::Instant::now();
    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn command: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    let output_collector: Arc<AMutex<(Vec<String>, Vec<String>)>> = Arc::new(AMutex::new((Vec::new(), Vec::new())));
    let cancel_token = tokio_util::sync::CancellationToken::new();

    spawn_output_streaming_task(
        subchat_tx.clone(),
        tool_call_id.to_string(),
        stdout,
        stderr,
        cancel_token.clone(),
        output_collector.clone(),
    );

    let timeout_duration = tokio::time::Duration::from_secs(timeout);
    let wait_result = tokio::time::timeout(timeout_duration, child.wait()).await;

    cancel_token.cancel();
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let duration = t0.elapsed();
    tracing::info!("SHELL: /finished in {:.3}s", duration.as_secs_f64());

    let exit_status = match wait_result {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => return Err(format!("Failed to wait for command: {}", e)),
        Err(_) => {
            let _ = child.kill().await;
            return Err(format!("Command '{}' timed out after {} seconds", command, timeout));
        }
    };

    let (stdout_lines, stderr_lines) = {
        let collector = output_collector.lock().await;
        (collector.0.clone(), collector.1.clone())
    };

    let stdout_str = stdout_lines.join("\n");
    let stderr_str = stderr_lines.join("\n");

    let mut out = crate::integrations::integr_cmdline::format_output(&stdout_str, &stderr_str);
    let exit_code = exit_status.code().unwrap_or_default();
    out.push_str(&format!("The command was running {:.3}s, finished with exit code {exit_code}\n", duration.as_secs_f64()));

    send_streaming_update(
        subchat_tx,
        tool_call_id,
        &format!("✅ Finished (exit code: {}, {:.1}s)", exit_code, duration.as_secs_f64())
    );

    Ok(out)
}

async fn parse_args(gcx: Arc<ARwLock<GlobalContext>>, args: &HashMap<String, Value>) -> Result<(String, Option<PathBuf>), String> {
    let (command, workdir, _, _) = parse_args_with_filter(gcx, args).await?;
    Ok((command, workdir))
}

async fn parse_args_with_filter(gcx: Arc<ARwLock<GlobalContext>>, args: &HashMap<String, Value>) -> Result<(String, Option<PathBuf>, Option<OutputFilter>, Option<u64>), String> {
    let command = match args.get("command") {
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err("Command is empty".to_string());
            } else {
                s.clone()
            }
        },
        Some(v) => return Err(format!("argument `command` is not a string: {:?}", v)),
        None => return Err("Missing argument `command`".to_string())
    };

    let workdir = match args.get("workdir") {
        Some(Value::String(s)) => {
            if s.is_empty() {
                None
            } else {
                Some(resolve_shell_workdir(gcx.clone(), s).await?)
            }
        },
        Some(v) => return Err(format!("argument `workdir` is not a string: {:?}", v)),
        None => None
    };

    let custom_filter = parse_output_filter_args(args);

    let timeout_override = args.get("timeout")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok());

    Ok((command, workdir, custom_filter, timeout_override))
}

fn parse_output_filter_args(args: &HashMap<String, Value>) -> Option<OutputFilter> {
    let output_filter_pattern = args.get("output_filter")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let output_limit = args.get("output_limit")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if output_filter_pattern.is_none() && output_limit.is_none() {
        return None;
    }

    let is_unlimited = matches!(output_limit.as_deref(), Some("all") | Some("full"));

    let limit_lines = if is_unlimited {
        usize::MAX
    } else {
        output_limit.as_deref().and_then(|s| s.parse::<usize>().ok()).unwrap_or(40)
    };

    Some(OutputFilter {
        limit_lines,
        limit_chars: if is_unlimited { usize::MAX } else { limit_lines * 200 },
        valuable_top_or_bottom: "top".to_string(),
        grep: output_filter_pattern.unwrap_or_else(|| "(?i)error".to_string()),
        grep_context_lines: 5,
        remove_from_output: "".to_string(),
        limit_tokens: if is_unlimited { None } else { Some(limit_lines * 50) },
    })
}

async fn resolve_shell_workdir(gcx: Arc<ARwLock<GlobalContext>>, raw_path: &str) -> Result<PathBuf, String> {
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
            return_one_candidate_or_a_good_error(gcx.clone(), &path_str, &candidates, &project_dirs, true).await?
        )
    };
    if !workdir.exists() {
        Err("Workdir doesn't exist".to_string())
    } else {
        Ok(workdir)
    }
}

pub const SHELL_INTEGRATION_SCHEMA: &str = r#"
fields:
  timeout:
    f_type: string_short
    f_desc: "The command must immediately return the results, it can't be interactive. If the command runs for too long, it will be terminated and stderr/stdout collected will be presented to the model."
    f_default: "10"
  output_filter:
    f_type: "output_filter"
    f_desc: "The output from the command can be long or even quasi-infinite. This section allows to set limits, prioritize top or bottom, or use regexp to show the model the relevant part."
    f_extra: true
description: |
  Allows to execute any command line tool. Most commands execute without confirmation. Dangerous commands require user confirmation.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
confirmation:
  ask_user_default: [
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
    "*srm*"
  ]
  deny_default: ["sudo*"]
"#;
