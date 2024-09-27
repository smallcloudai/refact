use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
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
use crate::at_commands::at_commands::AtCommandsContext;


pub struct ToolLocateSearch;


const LS_SYSTEM_PROMPT: &str = r###"You are an expert in finding relevant files within a big project.

Here's the list of reasons a file or symbol might be relevant wrt task description:

TOCHANGE = the main changes go there
MORE_TOCHANGE = likely to change as well, as a consequence of completing the task
ADD_NEARBY = good place to add new code
DEFINITIONS = classes/functions/types involved in the code that has to be changed
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
    "TOCHANGE": {                                   // one or two files, start with the best place to start making changes
        "dir/dir/file1.ext": "symbol1,symbol2",     // comma-separated symbols found in this file that need to be changed
        "dir/dir/file2.ext": "symbol1,symbol2",
    },
    "MORE_TOCHANGE": {                              // follow max values for number of files
        ...more files and symbols to change...
    },
    "ADD_NEARBY": {                                 // not necessarily you need to add any new things to complete the task, maybe the dict should be empty
        "dir/dir/file3.ext": "symbol1,symbol2",     // but if you need to, find a good place
    },
    "DEFINITIONS": {                                // don't list the same things again, if they are already in TOCHANGE
        ...files that have relevant definitions...
    },
    "HIGHLEV": {                                    // don't list the same things again
        ...
    },
    "USERCODE": {
        ...
    },
    "SIMILAR": {                                    // don't list the same things again
        ...
    }
}

Don't write backquotes, json format only.
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

        ccx.lock().await.pp_skeleton = true;

        let mut results: Vec<ContextEnum>;
        let usage: ChatUsage;
        let tool_message: String;
        (results, usage, tool_message) = find_relevant_files_with_search(
            ccx_subchat,
            params,
            tool_call_id.clone(),
            problem_statement,
        ).await?;

        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: tool_message,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            usage: Some(usage),
            ..Default::default()
        }));

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}

async fn find_relevant_files_with_search(
    ccx: Arc<AMutex<AtCommandsContext>>,
    subchat_params: SubchatParameters,
    tool_call_id: String,
    user_query: String,
) -> Result<(Vec<ContextEnum>, ChatUsage, String), String> {
    let gcx: Arc<ARwLock<GlobalContext>> = ccx.lock().await.global_context.clone();
    let total_files_in_project = gcx.read().await.documents_state.workspace_files.lock().unwrap().len();

    let mut usage = ChatUsage { ..Default::default() };
    // let mut real_files = IndexMap::new();
    let mut inspected_files = HashSet::new();
    let mut results: Vec<ContextEnum> = vec![];

    if total_files_in_project == 0 {
        let tool_message = format!("Inspected 0 files, project has 0 files");
        return Ok((results, usage, tool_message))
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

    let assistant_output = serde_json::from_str::<IndexMap<String, IndexMap<String, String>>>(last_message.content.as_str()).map_err(|e| {
        format!("Unable to parse JSON: {:?}", e)
    })?;

    for (category, files) in assistant_output.iter() {
        for (file_path, symbols) in files {
            let text = get_file_text_from_memory_or_disk(gcx.clone(), &std::path::PathBuf::from(&file_path)).await?.to_string();
            let lines_count = text.lines().count();
            let symbols_vec: Vec<String> = symbols.split(',').map(|s| s.to_string()).collect();

            match category.as_str() {
                "TOCHANGE" | "ADD_NEARBY" => {
                    results.push(ContextEnum::ContextFile(ContextFile {
                        file_name: file_path.clone(),
                        file_content: text.clone(),
                        line1: 0,
                        line2: lines_count,
                        symbols: symbols_vec.clone(),
                        gradient_type: -1,
                        usefulness: 100.0,
                        is_body_important: false,
                    }));
                },
                "MORE_TOCHANGE" | "DEFINITIONS" | "HIGHLEV" | "SIMILAR" | "USERCODE" => {
                    let usefulness = match category.as_str() {
                        "MORE_TOCHANGE" => 75.0,
                        "DEFINITIONS" => 80.0,
                        "HIGHLEV" => 75.0,
                        "SIMILAR" => 75.0,
                        "USERCODE" => 75.0,
                        _ => 0.0,
                    };
                    for symbol in symbols_vec {
                        results.push(ContextEnum::ContextFile(ContextFile {
                            file_name: file_path.clone(),
                            file_content: text.clone(),
                            line1: 0,
                            line2: lines_count,
                            symbols: vec![symbol.clone()],
                            gradient_type: -1,
                            usefulness,
                            is_body_important: false,
                        }));
                    }
                },
                _ => {},
            }
        }
    }

    let mut tool_message = format!(
            "{}\n\n💿 Used 1 expert, inspected {} files, project has {} files. Files are attached below. Don't call cat() for the same files, you alrady have them. Proceed to make changes using 📍-notation, if user has requested them. If you need to summarize the code, do it briefly, without extensive quotations. Answer in the language user prefers.",
        serde_json::to_string_pretty(&assistant_output).unwrap(),
        inspected_files.len(),
        total_files_in_project
    );

    // if !inspected_files.is_empty() {
    //     tool_message = format!("{}\n\nInspected context files:\n{}",
    //         tool_message,
    //         inspected_files.into_iter().collect::<Vec<_>>().join("\n"));
    // }

    // if !error_log.is_empty() {
    //     tool_message = format!("{}\n\nChecking file names against what actually exists, error log:\n{}", tool_message, error_log);
    // }

    Ok((results, usage, tool_message))
}
