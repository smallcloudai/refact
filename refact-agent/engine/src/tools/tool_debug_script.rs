use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use chrono::Local;
use serde_json::Value;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};

use crate::{
    at_commands::at_commands::AtCommandsContext,
    at_commands::at_file::return_one_candidate_or_a_good_error,
    call_validation::{ChatContent, ChatMessage, ChatUsage, ContextEnum},
    files_correction::{correct_to_nearest_dir_path, get_project_dirs},
    global_context::GlobalContext,
    subchat::{subchat, subchat_single},
    tools::tools_description::{Tool, ToolDesc, ToolParam},
};
use crate::at_commands::at_file::file_repair_candidates;
use crate::files_in_workspace::ls_files;

struct ToolDebugScriptArgs {
    path: PathBuf,
    problem: String,
}

async fn parse_args(
    gcx: Arc<ARwLock<GlobalContext>>,
    args: &HashMap<String, Value>,
) -> Result<ToolDebugScriptArgs, String> {
    let path = match args.get("path") {
        Some(Value::String(p)) => {
            let candidates_file = file_repair_candidates(gcx.clone(), &p, 1, false).await;
            let candidates_dir = correct_to_nearest_dir_path(gcx.clone(), &p, false, 1).await;

            let corrected_path = if !candidates_file.is_empty() || candidates_dir.is_empty() {
                let file_path = match return_one_candidate_or_a_good_error(gcx.clone(), &p, &candidates_file, &get_project_dirs(gcx.clone()).await, false).await {
                    Ok(f) => f,
                    Err(e) => { return Err(e); }
                };
                file_path.clone()
            } else {
                let candidate = match return_one_candidate_or_a_good_error(gcx.clone(), &p, &candidates_dir, &get_project_dirs(gcx.clone()).await, true).await {
                    Ok(f) => f,
                    Err(e) => { return Err(e); }
                };
                let path = PathBuf::from(candidate);
                let indexing_everywhere = crate::files_blocklist::reload_indexing_everywhere_if_needed(gcx.clone()).await;
                let files_in_dir = ls_files(&indexing_everywhere, &path, false).unwrap_or(vec![]);
                if files_in_dir.is_empty() {
                    return Err(format!("File not found in the project directory: {}", &p));
                }
                if files_in_dir.len() > 1 {
                    return Err(format!("Error: Multiple files found in the project directory: {}. Use absolute filename", &p));
                }
                files_in_dir[0].to_string_lossy().to_string()
            };
            
            
            // Verify it's a Python file
            let path = PathBuf::from(corrected_path);
            if path.extension().and_then(|ext| ext.to_str()) != Some("py") {
                return Err(format!("Error: File is not a Python script: {:?}", path));
            }
            
            path
        }
        Some(v) => return Err(format!("Error: The 'path' argument must be a string, but received: {:?}", v)),
        None => return Err("Error: The 'path' argument is required but was not provided.".to_string()),
    };
    
    let problem = match args.get("problem") {
        Some(Value::String(s)) => s.clone(),
        Some(v) => return Err(format!("Error: The 'problem' argument must be a string describing the issue to debug, but received: {:?}", v)),
        None => {
            return Err(format!(
                "Error: The 'problem' argument is required. Please provide a description of the problem to investigate for '{:?}'.",
                path
            ))
        }
    };

    Ok(ToolDebugScriptArgs {
        path,
        problem,
    })
}

const DEBUG_SYSTEM_PROMPT: &str = r###"**Role**  
You are a Python debugger who uses `pdb` to track down elusive bugs.

### Mission  
Find—and prove—the root cause of the error in the supplied Python script.

### Workflow  
1. **Grasp the Context**  
   * Skim the script and any problem description to learn what *should* happen vs. what *does* happen.
2. **Outline a Strategy**  
   * Jot a brief plan: where you’ll set breakpoints, which inputs you’ll try, and why.
3. **Drive `pdb`**  
   * Run the script under `pdb`.  
   * Use all available commands.  
4. **Observe & Record**  
   * At each critical line, log variable states, branch decisions, and side effects.  
   * Pay maximum attention on implicit operations (__bool__, __call__, ...).  
   * Note exceptions, warnings, or suspicious values.
5. **Hypothesize ➜ Test ➜ Confirm**  
   * Form theories about the fault.  
   * Validate (or reject) them with targeted probes and reruns.
6. **Probe Edge Cases**  
   * Feed unexpected or boundary inputs to expose hidden flaws.
7. **Summarize Findings**  
   * Compile a concise report that includes:
     * Key `pdb` commands and outputs that reveal the bug  
     * The definitive root cause (what, where, why)  
     * Evidence: variable dumps, stack traces, erroneous logic, etc.  
     * Suggested fix or next steps

### Style Guide  
- **Explain your reasoning** at every step; don’t just show commands.  
- Investigate *all* anomalies before moving on.  
- You can update the script and run `pdb` again if necessary.
- Keep logs clean—include only `pdb` excerpts that prove your point.  
- Write so another developer can reproduce your session and reach the same conclusion."###;

const DEBUG_SUMMARY_PROMPT: &str = r###"**Task**  
You will receive a raw debugging transcript (console output, stack traces, code snippets, notes). Create a concise and comprehensive report with the sections below.

### 1 – Problem Overview  
- **Issue summary** – one sentence describing the observed bug or unexpected behaviour.  
- **Expected behaviour** – what the script/module should have done.

### 2 – Project's Files & Symbols observed in the debugging process   
| File | Key symbols (functions / classes / vars)  | Purpose / responsibility |
|------|-------------------------------------------|--------------------------|
| …    | …                                         | …                        |

List every source file that appears in the transcript. Under “Key symbols” include notable functions, classes, globals, CLI commands, or config entries referenced.

### 3 – Debugging Timeline  
Provide a comprehensive step-by-step narrative of the investigation:  
1. Command or action executed  
2. Where execution paused, failed, or produced output  
3. Crucial variable values or state changes  
4. Fixes/experiments tried and their outcomes  

Highlight pivotal moments (first reliable reproduction, root-cause identification, fix verification).

### 4 – Lessons & Recommendations  
- **Pitfalls / anti-patterns** – missteps or code smells uncovered.  
- **Codebase insights** – architecture quirks, brittle areas, missing tests.
- Suggested fix or next steps"###;


pub struct ToolDebugScript;

#[async_trait]
impl Tool for ToolDebugScript {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let gcx = ccx.lock().await.global_context.clone();
        let params = crate::tools::tools_execute::unwrap_subchat_params(ccx.clone(), "debug_script").await?;
        let parsed_args = parse_args(gcx.clone(), args).await?;
        let path_str = parsed_args.path.to_string_lossy().to_string();
        let ccx_subchat = {
            let ccx_lock = ccx.lock().await;
            let mut ctx = AtCommandsContext::new(
                ccx_lock.global_context.clone(),
                params.subchat_n_ctx,
                1,
                false,
                ccx_lock.messages.clone(),
                ccx_lock.chat_id.clone(),
                ccx_lock.should_execute_remotely,
                ccx_lock.current_model.clone(),
            ).await;
            ctx.subchat_tx = ccx_lock.subchat_tx.clone();
            ctx.subchat_rx = ccx_lock.subchat_rx.clone();
            Arc::new(AMutex::new(ctx))
        };
        
        let mut usage_collector = ChatUsage::default();
        
        let log_prefix = Local::now().format("%Y%m%d-%H%M%S").to_string();
        tracing::info!(
            target: "debug_script",
            script = path_str,
            "Starting debugging session"
        );
        
        let script_content = std::fs::read_to_string(&parsed_args.path)
            .map_err(|e| format!("Failed to read script: {}", e))?;
        let debug_model = "refact/claude-3-7-sonnet".to_string();
        let debug_result = subchat(
            ccx_subchat.clone(),
            &debug_model,
            vec![ChatMessage::new(
                "system".to_string(),
                DEBUG_SYSTEM_PROMPT.to_string()
            ), ChatMessage::new(
                "user".to_string(),
                format!(
                    "Script to debug: {}\n\nProblem description: {}\n\nScript content:\n```python\n{}\n```\n",
                    path_str,
                    parsed_args.problem,
                    script_content
                )
            )],
            vec!["pdb".to_string(), "create_textdoc".to_string(), "update_textdoc".to_string()],
            120,
            params.subchat_max_new_tokens,
            "Summarise the debugging session transcript.",
            1,
            params.subchat_temperature,
            None,
            Some(tool_call_id.clone()),
            Some(format!("{log_prefix}-debug-script-{}", Path::new(&parsed_args.path).file_stem().unwrap_or_default().to_string_lossy())),
            Some(false)
        ).await?[0].clone();
        
        // Extract the debugging session content
        let mut debug_session = "".to_string();
        for message in debug_result.iter() {
            let iter_row = match message.role.as_str() {
                "user" => {
                    format!("👤:\n{}\n\n", &message.content.content_text_only())
                }
                "assistant" => {
                    let tool_call_mb = message.tool_calls.clone().map(|x|{
                        let tool_call = x.get(0).unwrap();
                        format!("{}({})", tool_call.function.name, tool_call.function.arguments).to_string()
                    }).unwrap_or_default();
                    format!("🤖:\n{}\n{}\n", &message.content.content_text_only(), tool_call_mb)
                }
                "tool" => {
                    format!("📎:\n{}\n\n", &message.content.content_text_only())
                }
                _ => {
                    "".to_string()
                }
            };
            debug_session.push_str(&iter_row);
        }
        let ccx_summary = {
            let ccx_lock = ccx.lock().await;
            let mut ctx = AtCommandsContext::new(
                ccx_lock.global_context.clone(),
                params.subchat_n_ctx,
                1,
                false,
                ccx_lock.messages.clone(),
                ccx_lock.chat_id.clone(),
                ccx_lock.should_execute_remotely,
                ccx_lock.current_model.clone(),
            ).await;
            ctx.subchat_tx = ccx_lock.subchat_tx.clone();
            ctx.subchat_rx = ccx_lock.subchat_rx.clone();
            Arc::new(AMutex::new(ctx))
        };
        let summary_prompt = format!(
            "{}\n\nScript: {}\n\nProblem description: {}\n\nDebugging session transcript:\n\n{}",
            DEBUG_SUMMARY_PROMPT,
            path_str,
            parsed_args.problem,
            debug_session
        );
        let summary_model = "refact/o4-mini".to_string();
        let summary_result = subchat_single(
            ccx_summary.clone(),
            &summary_model,
            vec![ChatMessage::new(
                "user".to_string(),
                summary_prompt
            )],
            Some(vec![]),
            None,
            false,
            params.subchat_temperature,
            Some(params.subchat_max_new_tokens),
            1,
            params.subchat_reasoning_effort.clone(),
            false,
            Some(&mut usage_collector),
            Some(tool_call_id.clone()),
            Some(format!("{log_prefix}-debug-summary-{}", Path::new(&parsed_args.path).file_stem().unwrap_or_default().to_string_lossy()))
        ).await?[0].clone();
        
        let summary = if let Some(last_msg) = summary_result.last() {
            match &last_msg.content {
                ChatContent::SimpleText(text) => text.clone(),
                _ => "No summary available".to_string(),
            }
        } else {
            "No summary available".to_string()
        };
        
        if let Some(last_debug_msg) = debug_result.last() {
            if let Some(usage) = &last_debug_msg.usage {
                usage_collector.prompt_tokens += usage.prompt_tokens;
                usage_collector.completion_tokens += usage.completion_tokens;
                usage_collector.total_tokens += usage.total_tokens;
            }
        }
        
        if let Some(last_summary_msg) = summary_result.last() {
            if let Some(usage) = &last_summary_msg.usage {
                usage_collector.prompt_tokens += usage.prompt_tokens;
                usage_collector.completion_tokens += usage.completion_tokens;
                usage_collector.total_tokens += usage.total_tokens;
            }
        }
        
        let prompt_tokens = usage_collector.prompt_tokens;
        let completion_tokens = usage_collector.completion_tokens;
        let total_tokens = usage_collector.total_tokens;
        
        tracing::info!(
            target: "debug_script",
            script = path_str,
            prompt_tokens = prompt_tokens,
            completion_tokens = completion_tokens,
            total_tokens = total_tokens,
            "Completed debugging session"
        );
        
        let final_result = vec![
            ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(format!(
                "# Debugging Summary for {}\n\n{}\n\n",
                path_str,
                summary
            )),
            usage: Some(usage_collector),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }),
            ContextEnum::ChatMessage(ChatMessage {
            role: "cd_instruction".to_string(),
            content: ChatContent::SimpleText(format!("💿 Open all mentioned files using `cat(file1,file2,file3,...)` and then fix the problem!")),
            ..Default::default()
        })];
        
        Ok((false, final_result))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "debug_script".into(),
            agentic: true,
            experimental: true,
            description: "Uses pdb to debug a Python script and investigate a problem, then summarizes the debugging session.".into(),
            parameters: vec![
                ToolParam {
                    name: "path".into(),
                    description: "Path to the file which needs to be debugged".into(),
                    param_type: "string".into(),
                },
                ToolParam {
                    name: "problem".into(),
                    description: "Description of the problem to investigate".into(),
                    param_type: "string".into(),
                },
            ],
            parameters_required: vec!["path".into(), "problem".into()],
        }
    }
}