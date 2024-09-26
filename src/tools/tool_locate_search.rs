use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;
use indexmap::IndexMap;
use hashbrown::HashSet;
use crate::subchat::subchat;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatUsage, ContextEnum, SubchatParameters, ContextFile};
use crate::global_context::GlobalContext;
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::files_correction::{get_project_dirs, shortify_paths};
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::at_commands::at_commands::AtCommandsContext;


pub struct ToolLocateSearch;


const LS_SYSTEM_PROMPT: &str = r###"You are an expert in finding relevant files within a big project.

Here's the list of reasons a file or symbol might be relevant wrt task description:

TOCHANGE = changes to that file are necessary to complete the task
DEFINITIONS = file has classes/functions/types involved, but no changes needed
HIGHLEV = file is crucial to understand the logic, such as a database scheme, high level script
USERCODE = file has code that uses the things the task description is about
SIMILAR = has code that might provide an example of how to write things similar to elements of the task

Your job is to use search() calls and summarize the results.

Some good ideas:

search("MyClass1")                  -- if MyClass1 mentioned in the task, for each symbol
search("log message 1 mentioned")   -- when the task has log messages, for each message
search("    def f():\n        print(\"the example function!\")")   -- look for the code piece mentioned in the task
search("imaginary_call(imaginary_arguments)\nmore_calls()\n")      -- you can imagine what kind of code you need to find

Call those in parallel.
"###;



#[async_trait]
impl Tool for ToolLocateSearch {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let problem_statement = match args.get("problem_statement") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `problem_statement` is not a string: {:?}", v)),
            None => return Err("Missing argument `problem_statement`".to_string())
        };

        let params = crate::tools::tools_execute::unwrap_subchat_params(ccx.clone(), "locate_search").await?;
        let ccx_subchat = {
            let ccx_lock = ccx.lock().await;
            let mut t = AtCommandsContext::new(
                ccx_lock.global_context.clone(),
                params.subchat_n_ctx,
                30,
                false,
                ccx_lock.messages.clone(),
            ).await;
            t.subchat_tx = ccx_lock.subchat_tx.clone();
            t.subchat_rx = ccx_lock.subchat_rx.clone();
            Arc::new(AMutex::new(t))
        };

        let (res, usage, tool_message) = find_relevant_files_with_search(
            ccx_subchat,
            params,
            tool_call_id.clone(),
            problem_statement,
        ).await?;

        let gcx = ccx.lock().await.global_context.clone();
        let tool_result = result_to_json(gcx.clone(), res.clone()).await;

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: format!("{}\n\n💿 {}", tool_result, tool_message),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            usage: Some(usage),
            ..Default::default()
        }));

        for (file_path, file_info) in res {
            let text = get_file_text_from_memory_or_disk(gcx.clone(), &std::path::PathBuf::from(&file_path)).await?.to_string();
            results.push(ContextEnum::ContextFile(ContextFile {
                file_name: file_path.clone(),
                file_content: text.clone(),
                line1: 0,
                line2: text.lines().count(),
                symbols: vec![],
                gradient_type: -1,
                usefulness: file_info.relevancy as f32 / 5. * 80.,
                is_body_important: false,
            }));
        }

        ccx.lock().await.pp_skeleton = true;

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["search".to_string()]
    }
}

async fn find_relevant_files_with_search(
    ccx: Arc<AMutex<AtCommandsContext>>,
    subchat_params: SubchatParameters,
    tool_call_id: String,
    user_query: String,
) -> Result<(IndexMap<String, ReduceFileOutput>, ChatUsage, String), String> {
    let gcx: Arc<ARwLock<GlobalContext>> = ccx.lock().await.global_context.clone();
    let total_files_in_project = gcx.read().await.documents_state.workspace_files.lock().unwrap().len();

    let mut usage = ChatUsage { ..Default::default() };
    let mut real_files = IndexMap::new();
    let mut inspected_files = HashSet::new();

    if total_files_in_project == 0 {
        let tool_message = format!("Used 0 experts, inspected 0 files, project has 0 files");
        return Ok((real_files, usage, tool_message))
    }

    let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();

    let mut strategy_messages = vec![];
    strategy_messages.push(ChatMessage::new("system".to_string(), RF_SYSTEM_PROMPT.to_string()));
    strategy_messages.push(ChatMessage::new("user".to_string(), user_query.to_string()));
    strategy_messages.push(ChatMessage::new("user".to_string(), "💿 Use SEARCH strategy.".to_string()));

    let result = subchat(
        ccx.clone(),
        subchat_params.subchat_model.as_str(),
        strategy_messages,
        vec!["search".to_string()],
        1,
        subchat_params.subchat_max_new_tokens,
        RF_EXPERT_WRAP_UP,
        1,
        Some(0.4),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-locate-search")),
    ).await?[0].clone();

    check_for_inspected_files(&mut inspected_files, &result);

    let last_message = result.last().unwrap();
    update_usage_from_message(&mut usage, &last_message);

    let reduced_files = parse_reduce_output(&last_message.content)?;

    let error_log: String;
    (real_files, error_log) = _reduced_files_to_reality(reduced_files, ccx.clone()).await;

    let mut tool_message = format!("Used 1 expert, inspected {} files, project has {} files",
        inspected_files.len(),
        total_files_in_project
    );
    if !inspected_files.is_empty() {
        tool_message = format!("{}\n\nInspected context files:\n{}",
            tool_message,
            inspected_files.into_iter().collect::<Vec<_>>().join("\n"));
    }
    if !error_log.is_empty() {
        tool_message = format!("{}\n\nChecking file names against what actually exists, error log:\n{}", tool_message, error_log);
    }

    Ok((real_files, usage, tool_message))
}

async fn result_to_json(gcx: Arc<ARwLock<GlobalContext>>, result: IndexMap<String, ReduceFileOutput>) -> String {
    let mut shortified = IndexMap::new();
    for (file_name, file_output) in result {
        let shortified_file_name = shortify_paths(gcx.clone(), vec![file_name]).await.get(0).unwrap().clone();
        shortified.insert(shortified_file_name, file_output);
    }
    serde_json::to_string_pretty(&serde_json::json!(shortified)).unwrap()
}

fn check_for_inspected_files(inspected_files: &mut HashSet<String>, messages: &[ChatMessage]) {
    for context_file_msg in messages.iter().filter(|msg| msg.role == "context_file").cloned().collect::<Vec<ChatMessage>>() {
        if let Ok(context_files) = serde_json::from_str::<Vec<ContextFile>>(&context_file_msg.content) {
            for context_file in context_files {
                inspected_files.insert(context_file.file_name.clone());
            }
        }
    }
}

fn update_usage_from_message(usage: &mut ChatUsage, message: &ChatMessage) {
    if let Some(u) = message.usage.as_ref() {
        usage.total_tokens += u.total_tokens;
        usage.completion_tokens += u.completion_tokens;
        usage.prompt_tokens += u.prompt_tokens;
    }
}

fn parse_reduce_output(content: &str) -> Result<IndexMap<String, ReduceFileOutput>, String> {
    let re = regex::Regex::new(r"(?s)REDUCE_OUTPUT\s*```(?:json)?\s*(.+?)\s*```").unwrap();
    let json_str = re.captures(content)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim())
        .ok_or_else(|| {
            tracing::warn!("Unable to find REDUCE_OUTPUT section:\n{}", content);
            "Unable to find REDUCE_OUTPUT section".to_string()
        })?;
    let output = serde_json::from_str::<IndexMap<String, ReduceFileOutput>>(json_str).map_err(|e| {
            tracing::warn!("Unable to parse JSON:\n{}({})", json_str, e);
            format!("Unable to parse JSON: {:?}", e)
        })?;

    // sort output by relevancy
    let mut output_vec: Vec<(String, ReduceFileOutput)> = output.into_iter().collect();
    output_vec.sort_by(|a, b| b.1.relevancy.cmp(&a.1.relevancy));
    let sorted_output = output_vec.into_iter().collect::<IndexMap<String, ReduceFileOutput>>();

    Ok(sorted_output)
}

async fn _reduced_files_to_reality(
    reduced_files: IndexMap<String, ReduceFileOutput>,
    ccx: Arc<AMutex<AtCommandsContext
