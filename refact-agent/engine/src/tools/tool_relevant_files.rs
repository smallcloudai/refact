use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use regex::Regex;
use std::path::PathBuf;
use rand::prelude::SliceRandom;
use async_trait::async_trait;
use indexmap::IndexMap;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use hashbrown::HashSet;

use crate::global_context::GlobalContext;
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::files_correction::{get_project_dirs, shortify_paths};
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatContent, ChatUsage, ContextEnum, SubchatParameters, ContextFile, ChatToolCall, ChatToolFunction};
use crate::subchat::subchat;
use crate::tools::tools_description::Tool;


async fn result_to_json(gcx: Arc<ARwLock<GlobalContext>>, result: IndexMap<String, ReduceFileOutput>) -> String {
    let mut shortified = IndexMap::new();
    for (file_name, file_output) in result {
        let shortified_file_name = shortify_paths(gcx.clone(), &vec![file_name]).await.get(0).unwrap().clone();
        shortified.insert(shortified_file_name, file_output);
    }
    serde_json::to_string_pretty(&serde_json::json!(shortified)).unwrap()
}

pub fn pretend_tool_call(tool_name: &str, tool_arguments: &str, content: String) -> ChatMessage {
    let mut rng = rand::thread_rng();
    let hex_chars: Vec<char> = "0123456789abcdef".chars().collect();
    let random_hex: String = (0..6)
        .map(|_| *hex_chars.choose(&mut rng).unwrap())
        .collect();
    let tool_call = ChatToolCall {
        id: format!("{tool_name}_{random_hex}"),
        function: ChatToolFunction {
            arguments: tool_arguments.to_string(),
            name: tool_name.to_string()
        },
        tool_type: "function".to_string(),
    };
    ChatMessage {
        role: "assistant".to_string(),
        content: ChatContent::SimpleText(content),
        tool_calls: Some(vec![tool_call]),
        tool_call_id: "".to_string(),
        ..Default::default()
    }
}



pub struct ToolRelevantFiles;

#[async_trait]
impl Tool for ToolRelevantFiles {
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

        let expand_depth = match args.get("expand_depth") {
            Some(Value::Number(n)) => n.as_u64().unwrap() as usize,
            Some(v) => return Err(format!("argument `expand_depth` is not a number: {:?}", v)),
            None => 2,
        };

        let params = crate::tools::tools_execute::unwrap_subchat_params(ccx.clone(), "locate").await?;
        let ccx_subchat = {
            let ccx_lock = ccx.lock().await;
            let mut t = AtCommandsContext::new(
                ccx_lock.global_context.clone(),
                params.subchat_n_ctx,
                30,
                false,
                ccx_lock.messages.clone(),
                ccx_lock.chat_id.clone(),
                ccx_lock.should_execute_remotely,
            ).await;
            t.subchat_tx = ccx_lock.subchat_tx.clone();
            t.subchat_rx = ccx_lock.subchat_rx.clone();
            Arc::new(AMutex::new(t))
        };

        let (res, usage, tool_message) = find_relevant_files(
            ccx_subchat,
            params,
            tool_call_id.clone(),
            problem_statement,
            expand_depth,
        ).await?;

        let gcx = ccx.lock().await.global_context.clone();
        let tool_result = result_to_json(gcx.clone(), res.clone()).await;

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(format!("{}\n\n💿 {}", tool_result, tool_message)),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            usage: Some(usage),
            ..Default::default()
        }));

        for (file_path, file_info) in res {
            let text = get_file_text_from_memory_or_disk(gcx.clone(), &PathBuf::from(&file_path)).await?.to_string();
            let mut ast_symbols = vec![];
            if let Some(ast_service) = gcx.read().await.ast_service.clone() {
                let ast_index = ast_service.lock().await.ast_index.clone();
                let doc_symbols = crate::ast::ast_db::doc_defs(ast_index.clone(), &file_path).await;
                let symbols = file_info.symbols.split(",").map(|x|x.to_string()).collect::<Vec<_>>();
                ast_symbols = doc_symbols.into_iter().filter(|s| symbols.contains(&s.name())).collect::<Vec<_>>();
            }

            // relevancy 1..5, normalized to 0..1 (because of skeleton param behavior) and then multiplied by 80 for usefulness
            // NOTE: 80 usefulness is the most controversial, probably we need to use some non-linear mapping from relevancy into usefulness
            let usefulness = file_info.relevancy as f32 / 5. * 80.;
            results.push(ContextEnum::ContextFile(ContextFile {
                file_name: file_path.clone(),
                file_content: text.clone(),
                line1: 0,
                line2: text.lines().count(),
                symbols: vec![],
                gradient_type: -1,
                usefulness,
            }));

            for symbol in ast_symbols {
                results.push(ContextEnum::ContextFile(ContextFile {
                    file_name: file_path.clone(),
                    file_content: "".to_string(),
                    line1: symbol.full_line1(),
                    line2: symbol.full_line2(),
                    symbols: vec![symbol.path()],
                    gradient_type: -1,
                    usefulness: 100.,
                }));
            }
        }

        ccx.lock().await.pp_skeleton = true;

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}


const RF_SYSTEM_PROMPT: &str = r###"You are an expert in finding relevant files within a big project.

Here's the list of reasons a file or symbol might be relevant wrt task description:

TOCHANGE = changes to that file are necessary to complete the task
DEFINITIONS = file has classes/functions/types involved, but no changes needed
HIGHLEV = file is crucial to understand the logic, such as a database scheme, high level script
USERCODE = file has code that uses the things the task description is about
SIMILAR = has code that might provide an example of how to write things similar to elements of the task


Potential strategies:

TREEGUESS = call tree(), look at names in a tree, pick up to 10 files that might be related
to the user's query and make a single call cat("all,the,files", skeleton=True) to see what's inside.

GOTODEF = call definition("xxx", skeleton=true) in parallel for symbols either visible in task description,
or symbols you can guess; don't call definition() for symbols from standard libraries, only symbols
within the project are indexed.

VECDBSEARCH = call up to five search() in parallel, some good ideas on what to look for: symbols
mentioned in the task, one call for each symbol, strings mentioned, or write imaginary code that does the
thing to fix search("    def f():\n        print(\"the example function!\")")

You'll receive additional instructions that start with 💿. Those are not coming from the user, they are programmed to help you operate
well between chat restarts and they are always in English. Answer in the language the user prefers.
"###;

const RF_EXPERT_WRAP_UP: &str = r###"Look at the problem statement at the top, and the context collected so far. Save your progress, using the following structure:
{
    "OUTPUT": [
        "dir/dir/file.ext": {             // A relative path to file visible in your context, with no ambiguity at all.
            "SYMBOLS": "symbol1,symbol2", // Comma-separated list of functions/classes/types/variables/etc defined or used within this file that are relevant to given problem. Write "*" to indicate the whole file is necessary.
        }
    ],
}
"###;

const RF_EXPAND_REDUCE_SYSTEM_PROMPT: &str = r###"You will receive outputs generated by experts using different strategies in the following format:

{
  "OUTPUT": {
      "dir/dir/file.ext": {
          "SYMBOLS": "symbol1,symbol2", // Comma-separated list of symbols defined within this file that are actually relevant. "*" might indicate the whole file is necessary.
      }
  ],
  ...
}

Steps you need to follow:

STEP1_CAT: make a cat() call with all files and symbols. Pass skeleton=True to the cat() call.

STEP2_EXPAND: expand the visible scope by looking up everything necessary to complete the task.

* Definitions: which classes and functions are necessary to understand the task? Don't ask about any well-known library functions
or classes like String and Arc in rust, librarires like re, os, subprocess in python, because they are are already well-known and including them
will not help, and libraries are not included in the AST index anyway.

* References: what relevant symbols require looking at usages to fully understand it? If the task is to repair my_function then it's
a good idea to look up usages of my_function.

* Similar code: maybe the task is already solved somewhere in the project, write a piece of code that would be required to solve
the problem, and put it into "query" argument of a search(). You can write the entire function if it's not too big. Search also works well for
examples of tricky calls, just write a couple of lines that will be hard to get right.

Examples:
definition("my_method1")
definition("MyClass2")
references("my_method2")
search("    def f():\n        print(\"the example function!\")")
search("    my_object->tricky_call(with, weird, parameters)")

Limits on the number of calls are pretty liberal, 10 definitions, 5 references and 3 searches is a reasonable request.

Don't explain much, say STEP1_CAT or STEP2_EXPAND depending on which step you are on, and then call the functions.

IT IS FORBIDDEN TO JUST CALL TOOLS WITHOUT EXPLAINING WHICH STEP YOU ARE ON. EXPLAIN FIRST!
"###;


const RF_REDUCE_WRAP_UP: &str = r###"
Experts can make mistakes. Your role is to reduce their noisy output into a single more reliable output. Think step by step. Follow this plan:

1. Write down a couple of interpretations of the original task, something like "Interpretation 1: user wants to do this, and the best place to start this change is at file1.ext, near my_function1, in my_function2".
2. Decide which interpretation is most likely correct.
3. Decide which files (at least one) will receive the most meaningful updates if the user was to change the code in that interpretation. You'll need to label them TOCHANGE later.
4. Write down which files might support the change, some of them contain high-level logic, some have definitions, some similar code.
5. All the files cannot have relevancy 5; most of them are likely 3, "might provide good insight into the logic behind the program but not directly relevant", but you can
write 1 or 2 if you accidentally wrote a file name and changed your mind about how useful it is, not a problem.
6. After you have completed 1-5, go ahead and formalize your best interpretation in the following JSON format, write "REDUCE_OUTPUT", and continue with triple backquotes. This format is crucial for the following parsing.

REDUCE_OUTPUT
```
{
    "dir/dir/file.ext": {
        "SYMBOLS": "symbol1,symbol2",     // Comma-separated list of symbols defined within this file that are actually relevant for initial problem. Use your own judgement, don't copy from experts.
        "WHY_CODE": "string",             // Write down the reason to include this file in output, pick one of: TOCHANGE, DEFINITIONS, HIGHLEV, USERCODE, SIMILAR.
        "WHY_DESC": "string",             // Describe why this file matters wrt the task, what's going on inside? Describe the file in general in a sentense or two, and then describe what specifically is the relation to the task.
        "RELEVANCY": 0                    // Critically evaluate how is this file really relevant to your interpretation of the task. Rate from 1 to 5. 1 = role is unclear, 3 = might provide good insight into the logic behind the program but not directly relevant, 5 = exactly what is needed.
    }
}
```
"###;



// REDUCE2 cat(files, symbols, skeleton=True) definition() usage() search() --EXPAND--> definition() usage() search() calls
// EXPAND cat(files, symbols) -> definition() usage() search() calls -> JSON2 files/symbols/RELEVANCY
// Experts make mistakes; take their RELEVANCY ratings critically, and write your own by looking at the actual code and the best interpretation.
// REDUCE2 cat(fles, symbols) definition() usage() search() -> JSON3
// 1. Confirm relevant symbols: look at the files already present in context, and write down all relevant
// Write a very short pseudo code of the most important piece to fix, mentioning classes and functions necessary.The pseudo code from point 1 might help.
// You have to be mindful of the token count, as some files are large. It's essential to
// select specific symbols within a file that are relevant. Another expert will
// pick up your results, likely they will have to only look at symbols selected by you,
// not whole files, because of the space constraints.

// You'll receive additional instructions that start with 💿. Those are not coming from the user, they are programmed to help you operate
// well between chat restarts and they are always in English. Answer in the language the user prefers.

// "WHY_CODE": "string",         // The reason to include this file in expert's output, one of: TOCHANGE, DEFINITIONS, HIGHLEV, USERCODE.
// "WHY_DESC": "string",         // Description why this file matters wrt the task.
// "RELEVANCY": 0                // Expert's own evaluation of their results, 1 to 5. 1 = this file doesn't even exist, 3 = might provide good insight into the logic behind the program but not directly relevant, 5 = exactly what is needed.

// "WHY_CODE": "string",         // Write down the reason to include this file in output, pick one of: TOCHANGE, DEFINITIONS, HIGHLEV, USERCODE. Put TBD if you didn't look inside.
// "WHY_DESC": "string",         // Describe why this file matters wrt the task, what's going on inside? Put TBD if you didn't look inside.
// "RELEVANCY": 0                // Critically evaluate how is this file really relevant to the task. Rate from 1 to 5. 1 = no evidence this file even exists, 2 = file exists but you didn't look inside, 3 = might provide good insight into the logic behind the program but not directly relevant, 5 = exactly what is needed.

fn parse_reduce_output(content: &str) -> Result<IndexMap<String, ReduceFileOutput>, String> {
    let re = Regex::new(r"(?s)REDUCE_OUTPUT\s*```(?:json)?\s*(.+?)\s*```").unwrap();
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


pub fn update_usage_from_message(usage: &mut ChatUsage, message: &ChatMessage) {
    if let Some(u) = message.usage.as_ref() {
        usage.total_tokens += u.total_tokens;
        usage.completion_tokens += u.completion_tokens;
        usage.prompt_tokens += u.prompt_tokens;
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
struct ReduceFileOutput {
    #[serde(rename = "SYMBOLS")]
    symbols: String,
    #[serde(rename = "WHY_CODE")]
    why_code: String,
    #[serde(rename = "WHY_DESC")]
    why_desc: String,
    #[serde(rename = "RELEVANCY")]
    relevancy: u8,
}


async fn find_relevant_files(
    ccx: Arc<AMutex<AtCommandsContext>>,
    subchat_params: SubchatParameters,
    tool_call_id: String,
    user_query: String,
    expand_depth: usize,
) -> Result<(IndexMap<String, ReduceFileOutput>, ChatUsage, String), String> {
    let gcx: Arc<ARwLock<GlobalContext>> = ccx.lock().await.global_context.clone();
    let (vecdb_on, total_files_in_project) = {
        let gcx_locked = gcx.read().await;
        #[cfg(feature="vecdb")]
        let vecdb_on = gcx_locked.vec_db.lock().await.is_some();
        #[cfg(not(feature="vecdb"))]
        let vecdb_on = false;
        let total_files_in_project = gcx_locked.documents_state.workspace_files.lock().unwrap().len();
        (vecdb_on, total_files_in_project)
    };

    let mut usage = ChatUsage { ..Default::default() };
    let mut real_files = IndexMap::new();
    let mut inspected_files = HashSet::new();

    if total_files_in_project == 0 {
        let tool_message = format!("Used {} experts, inspected {} files, project has {} files", 0, 0, 0);
        return Ok((real_files, usage, tool_message))
    }

    let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();

    let mut strategy_messages = vec![];
    strategy_messages.push(ChatMessage::new("system".to_string(), RF_SYSTEM_PROMPT.to_string()));
    strategy_messages.push(ChatMessage::new("user".to_string(), user_query.to_string()));

    let mut futures = vec![];

    // ----- TREEGUESS ------
    let strategy_tree_tools = vec!["tree", "cat"];
    let mut strategy_tree = strategy_messages.clone();
    strategy_tree.push(
        pretend_tool_call(
            "tree", "{}",
            "💿 I'll use TREEGUESS strategy, to do that I need to start with a tree() call.".to_string()
        )
    );

    futures.push(subchat(
        ccx.clone(),
        subchat_params.subchat_model.as_str(),
        strategy_tree,
        strategy_tree_tools.iter().map(|x|x.to_string()).collect::<Vec<_>>(),
        1,
        subchat_params.subchat_max_new_tokens,
        RF_EXPERT_WRAP_UP,
        1,
        Some(0.4),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-rf-step1-treeguess")),
        Some(false),  // prepend_system_prompt=false for o3
    ));

    // ----- VECDBSEARCH ------
    let mut strategy_search_tools = vec!["definition", "references"];
    let mut strategy_search = strategy_messages.clone();
    if vecdb_on {
        strategy_search_tools.push("search");
        strategy_search.push(ChatMessage::new("user".to_string(), "💿 Use VECDBSEARCH strategy.".to_string()));
    } else {
        strategy_search.push(ChatMessage::new("user".to_string(), "💿 Use GOTODEF strategy.".to_string()));
    }

    futures.push(subchat(
        ccx.clone(),
        subchat_params.subchat_model.as_str(),
        strategy_search,
        strategy_search_tools.iter().map(|x|x.to_string()).collect::<Vec<_>>(),
        2,
        subchat_params.subchat_max_new_tokens,
        RF_EXPERT_WRAP_UP,
        1,
        Some(0.4),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-rf-step1-gotodef")),
        Some(false),  // prepend_system_prompt=false for o3
    ));

    let results: Vec<Vec<Vec<ChatMessage>>> = join_all(futures).await.into_iter().filter_map(|x| x.ok()).collect();

    let mut expert_results = Vec::new();
    for choices in results.iter() {
        for messages in choices.iter() {
            if let Some(assistant_msg) = messages.iter().rfind(|msg| msg.role == "assistant").cloned() {
                update_usage_from_message(&mut usage, &assistant_msg);
                expert_results.push(assistant_msg);
            }
            check_for_inspected_files(&mut inspected_files, &messages);
        }
    }

    // ----- EXPAND/REDUCE ------
    let expand_reduce_tools = vec!["cat", "definition", "references", "search"];

    let mut messages = vec![];
    messages.push(ChatMessage::new("system".to_string(), RF_EXPAND_REDUCE_SYSTEM_PROMPT.to_string()));
    messages.push(ChatMessage::new("user".to_string(), format!("User provided task:\n\n{}", user_query)));
    for (i, expert_message) in expert_results.clone().into_iter().enumerate() {
        messages.push(ChatMessage::new("user".to_string(), format!("Expert {} says:\n\n{}", i + 1, expert_message.content.content_text_only())));
    }
    messages.push(ChatMessage::new("user".to_string(), "Start your answer with STEP1_CAT".to_string()));

    {
        let mut ccx_locked = ccx.lock().await;
        ccx_locked.correction_only_up_to_step = 0;  // NOTE: don't do unnecessary steps
    }

    let result = subchat(
        ccx.clone(),
        subchat_params.subchat_model.as_str(),
        messages,
        expand_reduce_tools.iter().map(|x|x.to_string()).collect::<Vec<_>>(),
        expand_depth + 1,  // expand_depth parameter slows down execution
        subchat_params.subchat_max_new_tokens,
        RF_REDUCE_WRAP_UP,
        1,
        Some(0.0),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-rf-step2-reduce")),
        Some(false),  // prepend_system_prompt=false for o3
    ).await?[0].clone();

    check_for_inspected_files(&mut inspected_files, &result);

    let last_message = result.last().unwrap();
    update_usage_from_message(&mut usage, &last_message);

    let reduced_files = parse_reduce_output(&last_message.content.content_text_only())?;

    let error_log: String;
    (real_files, error_log) = _reduced_files_to_reality(reduced_files, ccx.clone()).await;

    let mut tool_message = format!("Used {} experts, inspected {} files, project has {} files",
        expert_results.len(),
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


pub fn check_for_inspected_files(inspected_files: &mut HashSet<String>, messages: &[ChatMessage]) {
    for context_file_msg in messages.iter().filter(|msg| msg.role == "context_file").cloned().collect::<Vec<ChatMessage>>() {
        if let Ok(context_files) = serde_json::from_str::<Vec<ContextFile>>(&context_file_msg.content.content_text_only()) {
            for context_file in context_files {
                inspected_files.insert(context_file.file_name.clone());
            }
        }
    }
}

async fn _reduced_files_to_reality(
    reduced_files: IndexMap<String, ReduceFileOutput>,
    ccx: Arc<AMutex<AtCommandsContext>>,
) -> (IndexMap<String, ReduceFileOutput>, String) {
    let (gcx, top_n) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.global_context.clone(), ccx_locked.top_n)
    };

    let mut error_log = vec![];
    let mut reality = IndexMap::new();

    for (file_path, file_output) in reduced_files {
        let mut symbols = vec![];
        if !vec!["", "*"].contains(&file_output.symbols.as_str()) {
            symbols = file_output.symbols.split(",").map(|x| x.trim().to_string()).collect::<Vec<_>>()
        };

        // try to find single normalized file path
        let candidates_file = file_repair_candidates(gcx.clone(), &file_path, top_n, false).await;
        let real_path = match return_one_candidate_or_a_good_error(gcx.clone(), &file_path, &candidates_file, &get_project_dirs(gcx.clone()).await, false).await {
            Ok(f) => f,
            Err(e) => {
                error_log.push(e);
                continue;
            }
        };
        if reality.contains_key(&real_path) {
            // NOTE: idk what should we say in tool message about this situation
            continue;
        }

        // refine symbols according to ast
        let mut symbols_intersection = vec![];
        if let Some(ast_service) = gcx.read().await.ast_service.clone() {
            let ast_index = ast_service.lock().await.ast_index.clone();
            let doc_syms = crate::ast::ast_db::doc_defs(ast_index.clone(), &real_path).await;
            symbols_intersection = doc_syms.into_iter().filter(|s| symbols.contains(&s.name())).collect::<Vec<_>>();
        }
        let mut refined_file_output = file_output.clone();
        // NOTE: for now we are simply skipping non-existing symbols, but it can be presented in tool message
        refined_file_output.symbols = symbols_intersection.iter().map(|x| x.name()).collect::<Vec<_>>().join(",");
        reality.insert(real_path, refined_file_output);
    }

    (reality, error_log.join("\n"))
}
