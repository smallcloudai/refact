use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use async_trait::async_trait;
use indexmap::IndexMap;
use hashbrown::HashSet;
use crate::subchat::subchat;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatUsage, ContextEnum, SubchatParameters, ContextFile};
use crate::global_context::GlobalContext;
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::files_correction::{shortify_paths};
use crate::at_commands::at_commands::AtCommandsContext;


pub struct ToolLocateSearch;


const LS_SYSTEM_PROMPT: &str = r###"You are an expert in finding relevant files within a big project.

Here's the list of reasons a file or symbol might be relevant wrt task description:

TOCHANGE = the main changes go there
MORE_TOCHANGE = likely to change as well, as a consequence of completing the task
DEFINITIONS = classes/functions/types involved, but no changes needed
HIGHLEV = crucial to understand the logic, such as a database scheme, high level script
USERCODE = code that uses the things the task description is about
SIMILAR = code that might provide an example of how to write similar things

Your job is to use search() calls and summarize the results.

Some good ideas:

search("MyClass1")                  -- if MyClass1 mentioned in the task, for each symbol
search("log message 1 mentioned")   -- when the task has log messages, for each message
search("    def f():\n        print(\"the example function!\")")   -- look for the code piece mentioned in the task
search("imaginary_call(imaginary_arguments)\nmore_calls()\n")      -- you can imagine what kind of code you need to find

Call any of those that make sense in parallel.
"###;


const LS_WRAP_UP: &str = r###"
Look at the task at the top, and the files collected so far.

Save your progress, here are some guidelines:

1. There can be only one or two files TOCHANGE. But there has to be one or two, not zero.

2. Of course there could be a lot of MORE_TOCHANGE or USERCODE files, each file in the project can be potentially.
Prefer small and simple files for MORE_TOCHANGE and USERCODE. Limit the number of USERCODE to 3 files, and MORE_TOCHANGE to 3 files.

3. Limit the number of SIMILAR files to 1. Take the best, most similar to whatever the task is about.

4. Limit the number of HIGHLEV files to 2. Take the best, most relevant high level logic for the task.

5. Limit the number of DEFINITIONS to 5.

6. Each file can occur only once in the output. Use comma-separated list for symbols within the file.

If not sure, drop the file, compact output is better.

Use the following structure:

{
    "dir/dir/file.ext": {             // A relative path to file visible in your context, with no ambiguity at all.
        "symbols": "symbol1,symbol2", // Comma-separated list of functions/classes/types/variables/etc within this file that are relevant to the task. Write "*" to indicate the whole file is necessary.
        "why_code": "string",         // Reason to include the file: TOCHANGE, DEFINITIONS, HIGHLEV, USERCODE, SIMILAR
    }
    ...all relevant files...
}

Don't write backquotes, json format only.
"###;


#[derive(Serialize, Deserialize, Debug)]
struct LocateOutput {
    symbols: String,
    why_code: String,
    // "desc_wrt_task": "string",    // What does it do that is relevant to the task? Avoid generic language, put there identifiers and actions performed.
    // desc_wrt_task: String,
}


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

        ccx.lock().await.pp_skeleton = true;

        let (res, usage, tool_message) = find_relevant_files_with_search(
            ccx_subchat,
            params,
            tool_call_id.clone(),
            problem_statement,
        ).await?;

        let gcx = ccx.lock().await.global_context.clone();

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: tool_message,
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
                usefulness: 90.0,
                // usefulness: file_info.relevancy as f32 / 5. * 80.,
                is_body_important: false,
            }));
        }

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
) -> Result<(IndexMap<String, LocateOutput>, ChatUsage, String), String> {
    let gcx: Arc<ARwLock<GlobalContext>> = ccx.lock().await.global_context.clone();
    let total_files_in_project = gcx.read().await.documents_state.workspace_files.lock().unwrap().len();

    let mut usage = ChatUsage { ..Default::default() };
    let mut real_files = IndexMap::new();
    let mut inspected_files = HashSet::new();

    if total_files_in_project == 0 {
        let tool_message = format!("Inspected 0 files, project has 0 files");
        return Ok((real_files, usage, tool_message))
    }

    let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();

    let mut msgs = vec![];
    msgs.push(ChatMessage::new("system".to_string(), LS_SYSTEM_PROMPT.to_string()));
    msgs.push(ChatMessage::new("user".to_string(), user_query.to_string()));

    let result = subchat(
        ccx.clone(),
        subchat_params.subchat_model.as_str(),
        msgs,
        vec!["search".to_string()],
        1,
        subchat_params.subchat_max_new_tokens,
        LS_WRAP_UP,
        1,
        Some(0.1),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-locate-search")),
    ).await?[0].clone();

    crate::tools::tool_relevant_files::check_for_inspected_files(&mut inspected_files, &result);

    let last_message = result.last().unwrap();
    crate::tools::tool_relevant_files::update_usage_from_message(&mut usage, &last_message);
    assert!(last_message.role == "assistant");
    // let reduced_files = parse_reduce_output(&last_message.content)?;
    let files2output = serde_json::from_str::<IndexMap<String, LocateOutput>>(last_message.content.as_str()).map_err(|e| {
        format!("Unable to parse JSON: {:?}", e)
    })?;

    // let error_log: String;
    // (real_files, error_log) = _reduced_files_to_reality(reduced_files, ccx.clone()).await;

    let mut tool_message = format!("{}\n\n💿 Used 1 expert, inspected {} files, project has {} files",
        serde_json::to_string_pretty(&files2output).unwrap(),
        inspected_files.len(),
        total_files_in_project
    );
    if !inspected_files.is_empty() {
        tool_message = format!("{}\n\nInspected context files:\n{}",
            tool_message,
            inspected_files.into_iter().collect::<Vec<_>>().join("\n"));
    }
    // if !error_log.is_empty() {
    //     tool_message = format!("{}\n\nChecking file names against what actually exists, error log:\n{}", tool_message, error_log);
    // }

    Ok((real_files, usage, tool_message))
}

// async fn result_to_json(gcx: Arc<ARwLock<GlobalContext>>, result: IndexMap<String, LocateOutput>) -> String {
//     let mut shortified = IndexMap::new();
//     for (file_name, file_output) in result {
//         let shortified_file_name = shortify_paths(gcx.clone(), vec![file_name]).await.get(0).unwrap().clone();
//         shortified.insert(shortified_file_name, file_output);
//     }
//     serde_json::to_string_pretty(&serde_json::json!(shortified)).unwrap()
// }
