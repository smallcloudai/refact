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
use crate::call_validation::{ChatMessage, ChatContent, ChatUsage, ContextEnum, SubchatParameters, ContextFile};
use crate::global_context::GlobalContext;
use crate::at_commands::at_commands::AtCommandsContext;


pub struct ToolLocateSearch;


const LS_SYSTEM_PROMPT: &str = r###"You are an expert in finding relevant files within a big project.

Here's the list of reasons a file or symbol might be relevant wrt task description:

FOUND = the files and the symbols that the task explicitly tells you to find, or the files and symbols the main changes should go into if the task requires changes
NEWFILE = sometimes the task requires creating new files
MORE_TOCHANGE = likely to change as well, as a consequence of completing the task
USAGE = code that uses the things the task description is about
SIMILAR = code that might provide an example of how to write similar things

Your job is to use search() calls and summarize the results.

Some good ideas:

search("MyClass1")                  -- if MyClass1 mentioned in the task, for each symbol
search("log message 1 mentioned")   -- when the task has log messages, for each message
search("    def f():\n        print(\"the example function!\")")   -- look for the code piece mentioned in the task
search("imaginary_call(imaginary_arguments)\nmore_calls()\n")      -- you can imagine what kind of code you need to find

Call any of those that make sense in parallel. Make at least two calls in parallel, pay special attention that at least one
search() call should not have a restrictive scope, because you are running the risk of getting no results at all.
"###;


const LS_WRAP_UP: &str = r###"
Look at the task at the top, and the files collected so far. Not all files are actually useful, many of them are irrelevant.
Follow these guidelines:

0. Does the task make sense at all, after looking at the files? Fill the "rejection" output structure if it doesn't, and stop.

1. If the task tells to find something, it should go to FOUND. If the task requires changes, decide which one or two files
need to go to FOUND, or is it NEWFILE, it can't be no files at all if the task requires changes, fill "rejection" if that's the case.

2. If you see similar code, take the best, most similar to whatever the task is about. It can't be one of FOUND files.
No such files (zero) is a perfectly good answer. If the task tells to implement something by analogy, the files to draw the
analogy from should go to SIMILAR. Limit the number of SIMILAR files to 1, maybe 2, 3 at most.

4. Of course there could be a lot of MORE_TOCHANGE or USAGE files, potentially every file in the project can be.
Limit the number of USAGE to 3 files, and MORE_TOCHANGE to 3 files. Prefer small and simple files.
No such files (zero) is a perfectly good answer, don't guess and make stuff up. Don't put files here just in case.
Only if you are reasonably certain they will also need to change (MORE_TOCHANGE) or they use the thing that
changes (USAGE).

If not sure, drop the file, compact output is better.

Use the following structure:

{
    "rejection": "string"                           // Fill this if there are no files matching the task, avoid generic language, name specific things that you did or didn't find, what exactly didn't add up, and stop.
}

or

{
    "NEW_FILE": {                                   // Does the task require any new files? Don't make stuff up if the task doesn't require any.
        "dir/dir/file1.ext": ""                     // For new files, don't fill any symbols to look up and prioritize (because none exist yet)
    },
    "FOUND": {                                      // Does the task require to find files or symbols? Does the task require to change existing files?
        "dir/dir/file2.ext": "symbol1,symbol2",     // Be specific, what symbols require changes or match the description in the task?
        "dir/dir/file3.ext": "symbol1,symbol2"
    },
    "SIMILAR": {                                    // Don't list the same files again
        "dir/dir/file4.ext": "symbol1,symbol2"      // For files not in FOUND, list symbols with similar code to what the task is about
    },
    "MORE_TOCHANGE": {
        ...more files and symbols in them to change...
    },
    "USAGE": {
        ...files with code that uses the thing to change, very important to name specific symbols where the use happens...
    }
}

Don't write backquotes, json format only.
"###;


#[async_trait]
impl Tool for ToolLocateSearch {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
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
                7,  // top_n
                false,
                ccx_lock.messages.clone(),
                ccx_lock.chat_id.clone(),
                ccx_lock.should_execute_remotely,
            ).await;
            t.subchat_tx = ccx_lock.subchat_tx.clone();
            t.subchat_rx = ccx_lock.subchat_rx.clone();
            Arc::new(AMutex::new(t))
        };

        ccx.lock().await.pp_skeleton = true;

        let (mut results, usage, tool_message, cd_instruction) = find_relevant_files_with_search(
            ccx_subchat,
            params,
            tool_call_id.clone(),
            problem_statement,
        ).await?;

        tracing::info!("\n{}", tool_message);

        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(tool_message),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            usage: Some(usage),
            ..Default::default()
        }));

        if !cd_instruction.is_empty() {
            tracing::info!("\n{}", cd_instruction);
            results.push(ContextEnum::ChatMessage(ChatMessage {
                role: "cd_instruction".to_string(),
                content: ChatContent::SimpleText(cd_instruction),
                tool_calls: None,
                tool_call_id: "".to_string(),
                usage: None,
                ..Default::default()
            }));
        }

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
) -> Result<(Vec<ContextEnum>, ChatUsage, String, String), String> {
    ccx.lock().await.pp_skeleton = true;
    let gcx: Arc<ARwLock<GlobalContext>> = ccx.lock().await.global_context.clone();
    let total_files_in_project = gcx.read().await.documents_state.workspace_files.lock().unwrap().len();

    let mut usage = ChatUsage { ..Default::default() };
    // let mut real_files = IndexMap::new();
    let mut inspected_files = HashSet::new();
    let mut results: Vec<ContextEnum> = vec![];

    if total_files_in_project == 0 {
        let tool_message = format!("Inspected 0 files, project has 0 files");
        return Ok((results, usage, tool_message, "".to_string()))
    }

    let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();

    let mut msgs = vec![];
    msgs.push(ChatMessage::new("system".to_string(), LS_SYSTEM_PROMPT.to_string()));
    msgs.push(ChatMessage::new("user".to_string(), user_query.to_string()));
    msgs.push(ChatMessage::new("cd_instruction".to_string(), "Look at user query above. Follow the system prompt. Run several search() calls in parallel.".to_string()));

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
        Some(false),  
    ).await?[0].clone();

    crate::tools::tool_relevant_files::check_for_inspected_files(&mut inspected_files, &result);

    let last_message = result.last().unwrap();
    crate::tools::tool_relevant_files::update_usage_from_message(&mut usage, &last_message);
    assert!(last_message.role == "assistant");

    let assistant_output1 = serde_json::from_str::<IndexMap<String, serde_json::Value>>(last_message.content.content_text_only().as_str()).map_err(|e| {
        tracing::warn!("\n{}\nUnable to parse JSON: {:?}", last_message.content.content_text_only(), e);
        format!("Unable to parse JSON: {:?}", e)
    })?;
    let rejection = assistant_output1.get("rejection");
    if let Some(_rejection_message) = rejection {
        let cd_instruction = format!("💿 locate() looked inside of {} files, workspace has {} files.", inspected_files.len(), total_files_in_project).replace("\n", " ");
        return Ok((results, usage, serde_json::to_string_pretty(&assistant_output1).unwrap(), cd_instruction));
    }

    let assistant_output2 = serde_json::from_str::<IndexMap<String, IndexMap<String, String>>>(last_message.content.content_text_only().as_str()).map_err(|e| {
        tracing::warn!("\n{}\nUnable to parse JSON: {:?}", last_message.content.content_text_only(), e);
        format!("Unable to parse JSON: {:?}", e)
    })?;

    let processed_results = process_assistant_output(&assistant_output2).await?;
    results.extend(processed_results);

    let cd_instruction = format!(r###"💿 locate() looked inside of {} files, workspace has {} files. Files relevant to the task were attached above.
Don't call cat() for the same files, you already have them. Follow your task and the system prompt.
"###, inspected_files.len(), total_files_in_project).replace("\n", " ");

// You can proceed to make changes, if the user has requested the changes, change two files at most. If you see more files you need to change,
// list the files you know, maybe try to come up with a generalized way to find such files, for example references("the_function_that_changed"), write about it
// and stop. If you need to summarize the code, do it briefly, without extensive quotations. Answer in the language the user prefers. Follow the system prompt.

    // if !inspected_files.is_empty() {
    //     tool_message = format!("{}\n\nInspected context files:\n{}",
    //         tool_message,
    //         inspected_files.into_iter().collect::<Vec<_>>().join("\n"));
    // }

    // if !error_log.is_empty() {
    //     tool_message = format!("{}\n\nChecking file names against what actually exists, error log:\n{}", tool_message, error_log);
    // }

    Ok((results, usage, serde_json::to_string_pretty(&assistant_output2).unwrap(), cd_instruction))
}


async fn process_assistant_output(
    assistant_output: &IndexMap<String, IndexMap<String, String>>,
) -> Result<Vec<ContextEnum>, String> {
    let mut results: Vec<ContextEnum> = vec![];

    for (category, files) in assistant_output.iter() {
        for (file_path, symbols) in files {
            match category.as_str() {
                "FOUND" => {
                    let file_usefulness = match category.as_str() {
                        "FOUND" => 100.0,
                        _ => panic!("unexpected category: {:?}", category),
                    };
                    results.push(ContextEnum::ContextFile(ContextFile {
                        file_name: file_path.clone(),
                        file_content: "".to_string(),
                        line1: 0,
                        line2: 0,
                        symbols: vec![],
                        gradient_type: -1,
                        usefulness: file_usefulness,
                    }));
                },
                "MORE_TOCHANGE" | "SIMILAR" | "USAGE" => {
                    let symbols_vec: Vec<String> = symbols.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let symbol_usefulness = match category.as_str() {
                        "MORE_TOCHANGE" => 75.0,
                        "SIMILAR" => 75.0,
                        "USAGE" => 75.0,
                        _ => panic!("unexpected category: {:?}", category),
                    };
                    for symbol in symbols_vec {
                        results.push(ContextEnum::ContextFile(ContextFile {
                            file_name: file_path.clone(),
                            file_content: "".to_string(),
                            line1: 0,
                            line2: 0,
                            symbols: vec![symbol.clone()],
                            gradient_type: -1,
                            usefulness: symbol_usefulness,
                        }));
                    }
                },
                _ => {
                    tracing::warn!("unexpected category: {:?}", category);
                },
            }
        }
    }

    Ok(results)
}
